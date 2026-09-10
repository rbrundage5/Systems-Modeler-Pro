#!/usr/bin/env python3
"""Plan and publish one immutable Windows release from successful main CI."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import time
from urllib.error import HTTPError
from urllib.parse import quote
from urllib.request import Request, urlopen

REPO = "rbrundage5/Systems-Modeler-Pro"
REQUIRED = {"core", "desktop-check", "desktop-linux-check", "configuration",
            "startup-update", "release-contract", "signed-package"}

def version_for(run_number):
    if not isinstance(run_number, int) or isinstance(run_number, bool) or run_number < 1:
        raise ValueError("Native CI run number must be positive")
    minor, patch = 2 + run_number // 60000, run_number % 60000
    if minor > 65535:
        raise ValueError("Windows version range exhausted")
    return f"0.{minor}.{patch}"

def version_tuple(version):
    if not re.fullmatch(r"0\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", version):
        raise ValueError("Unexpected release version")
    values = tuple(map(int, version.split(".")))
    if max(values) > 65535:
        raise ValueError("Version component exceeds Windows range")
    return values

def validate_run(run):
    if (run.get("event") != "push" or run.get("head_branch") != "main"
            or run.get("status") != "completed" or run.get("conclusion") != "success"
            or run.get("head_repository", {}).get("full_name") != REPO
            or run.get("path") != ".github/workflows/ci.yml"
            or not re.fullmatch("[0-9a-f]{40}", run.get("head_sha", ""))):
        raise ValueError("Release source must be successful native CI for this repository's main push")
    return {"sha": run["head_sha"], "version": version_for(run["run_number"]),
            "run_id": run["id"], "tag": "desktop-v" + version_for(run["run_number"])}

def check_state(checks):
    latest = {}
    for check in checks:
        # workflow_run also creates skipped validation jobs on the same SHA.
        # Those are not executions and must not supersede real push validation.
        if (check.get("app", {}).get("slug") == "github-actions"
                and check.get("conclusion") != "skipped"):
            name = check["name"]
            if name not in latest or check["id"] > latest[name]["id"]:
                latest[name] = check
    for name in REQUIRED:
        check = latest.get(name)
        if check and check["status"] == "completed" and check["conclusion"] != "success":
            raise ValueError(f"Required check failed: {name}")
    return all(name in latest and latest[name]["status"] == "completed"
               and latest[name]["conclusion"] == "success" for name in REQUIRED)

class GitHub:
    def request(self, path, method="GET", data=None, missing_ok=False):
        payload = None if data is None else json.dumps(data).encode()
        request = Request("https://api.github.com/repos/" + REPO + path, data=payload, method=method,
            headers={"Authorization": "Bearer " + os.environ["GH_TOKEN"],
                     "Accept": "application/vnd.github+json", "Content-Type": "application/json",
                     "X-GitHub-Api-Version": "2022-11-28"})
        try:
            with urlopen(request, timeout=45) as response:
                return json.load(response)
        except HTTPError as error:
            if missing_ok and error.code == 404:
                return None
            raise RuntimeError(f"GitHub request failed: {method} {path}: HTTP {error.code}") from None

    def upload(self, release_id, path):
        request = Request(
            f"https://uploads.github.com/repos/{REPO}/releases/{release_id}/assets?name={quote(path.name)}",
            data=path.read_bytes(), method="POST",
            headers={"Authorization": "Bearer " + os.environ["GH_TOKEN"],
                     "Content-Type": "application/octet-stream"})
        with urlopen(request, timeout=180) as response:
            return json.load(response)

    def checks(self, sha):
        result = []
        for page in range(1, 11):
            batch = self.request(f"/commits/{sha}/check-runs?filter=all&per_page=100&page={page}")["check_runs"]
            result.extend(batch)
            if len(batch) < 100:
                return result
        raise RuntimeError("Check pagination exceeded bound")

def still_current(api, plan):
    if api.request("/branches/main")["commit"]["sha"] != plan["sha"]:
        return False
    release = api.request("/releases/latest", missing_ok=True)
    if release:
        tag = release["tag_name"]
        if not tag.startswith("desktop-v"):
            raise ValueError("Latest release is outside the desktop release channel")
        if version_tuple(tag.removeprefix("desktop-v")) >= version_tuple(plan["version"]):
            return False
    return True

def plan_release(api, run_id, attempts=120):
    plan = validate_run(api.request(f"/actions/runs/{run_id}"))
    for attempt in range(attempts):
        if not still_current(api, plan):
            return None
        if check_state(api.checks(plan["sha"])):
            return plan
        if attempt + 1 < attempts:
            time.sleep(10)
    raise RuntimeError("Timed out waiting for required checks; no release authorized")

def write_manifest(directory, plan):
    directory = Path(directory)
    installers = list(directory.glob("*-setup.exe"))
    if len(installers) != 1:
        raise ValueError("Expected exactly one installer")
    installer = installers[0]
    signature = Path(str(installer) + ".sig")
    if not signature.is_file() or not signature.read_text().strip():
        raise ValueError("Signed installer is required")
    version_tuple(plan["version"])
    if plan["tag"] != "desktop-v" + plan["version"]:
        raise ValueError("Release tag does not match version")
    metadata = json.loads((directory / "BUILD_INFO.json").read_text(encoding="utf-8-sig"))
    if metadata["source_commit"] != plan["sha"] or metadata["application_version"] != plan["version"]:
        raise ValueError("Build provenance does not match release plan")
    if metadata.get("signature_verified") is not True:
        raise ValueError("Build did not record successful signature verification")
    digest = hashlib.sha256(installer.read_bytes()).hexdigest()
    if metadata["sha256"] != digest:
        raise ValueError("Installer digest does not match build provenance")
    manifest = {"version": plan["version"], "notes": f"Tested source commit: {plan['sha']}",
        "platforms": {"windows-x86_64": {
            "signature": signature.read_text().strip(),
            "url": f"https://github.com/{REPO}/releases/download/{plan['tag']}/{quote(installer.name)}"}}}
    (directory / "latest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return [installer, signature, directory / "BUILD_INFO.json", directory / "latest.json"]

def publish(api, directory, plan):
    if not still_current(api, plan):
        return False
    if not check_state(api.checks(plan["sha"])):
        raise ValueError("Required checks are no longer successful")
    files = write_manifest(directory, plan)
    if api.request("/releases/tags/" + plan["tag"], missing_ok=True):
        raise ValueError("Release tag already exists; never overwrite an existing or partial release")
    release = api.request("/releases", "POST", {
        "tag_name": plan["tag"], "target_commitish": plan["sha"],
        "name": "Systems Modeler Pro " + plan["version"], "draft": True,
        "prerelease": False,
        "body": f"Windows x64 desktop update. Source: {plan['sha']}. Native CI run: {plan['run_id']}.\n"
                "Updates install before opening a modeling session. This release does not certify whole-product SysML completeness."})
    for path in files:
        api.upload(release["id"], path)
    # A newer commit arriving during upload leaves this release as an unpublished draft.
    if not still_current(api, plan):
        return False
    if not check_state(api.checks(plan["sha"])):
        raise ValueError("Required checks changed during upload; release remains draft")
    api.request(f"/releases/{release['id']}", "PATCH", {"draft": False, "make_latest": "true"})
    return True

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("operation", choices=["plan", "publish"])
    parser.add_argument("--run-id", type=int)
    parser.add_argument("--plan", default="release-plan.json")
    parser.add_argument("--directory", default="dist")
    args = parser.parse_args()
    api = GitHub()
    if args.operation == "plan":
        if not args.run_id or args.run_id < 1:
            parser.error("--run-id is required")
        plan = plan_release(api, args.run_id)
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            output.write(f"publish={'true' if plan else 'false'}\n")
            if plan:
                for key in ("sha", "version", "tag", "run_id"):
                    output.write(f"{key}={plan[key]}\n")
        if plan:
            Path(args.plan).write_text(json.dumps(plan))
    else:
        plan = json.loads(Path(args.plan).read_text(encoding="utf-8-sig"))
        print("Release published." if publish(api, args.directory, plan)
              else "Superseded release skipped; no published release changed.")

if __name__ == "__main__":
    main()
