import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import desktop_release as release

SHA = "a" * 40
RUN = {"id": 77, "run_number": 1250, "event": "push", "head_branch": "main",
       "status": "completed", "conclusion": "success", "path": ".github/workflows/ci.yml",
       "head_repository": {"full_name": release.REPO}, "head_sha": SHA}

def good_checks():
    return [{"id": i, "name": name, "status": "completed", "conclusion": "success",
             "app": {"slug": "github-actions"}} for i, name in enumerate(sorted(release.REQUIRED), 1)]

class FakeGitHub:
    def __init__(self):
        self.main = SHA
        self.latest = None
        self.check_results = good_checks()
        self.existing = None
        self.writes = []
        self.uploaded = []
        self.fail_upload = False
        self.advance_on_upload = False
        self.fail_check_on_upload = False
    def checks(self, sha): return self.check_results
    def request(self, path, method="GET", data=None, missing_ok=False):
        if method != "GET":
            self.writes.append((path, method, data))
            return {"id": 99}
        if path == "/branches/main": return {"commit": {"sha": self.main}}
        if path == "/releases/latest": return self.latest
        if path.startswith("/actions/runs/"): return RUN
        if path.startswith("/releases/tags/"): return self.existing
        raise AssertionError(path)
    def upload(self, release_id, path):
        if self.fail_upload: raise RuntimeError("upload interrupted")
        self.uploaded.append(path.name)
        if self.advance_on_upload: self.main = "b" * 40
        if self.fail_check_on_upload: self.check_results[0]["conclusion"] = "failure"

class ReleaseTests(unittest.TestCase):
    def fixture(self, folder):
        plan = release.validate_run(RUN)
        file = Path(folder) / "Systems Modeler Pro_0.2.1250_x64-setup.exe"
        file.write_bytes(b"MZ-test-installer")
        Path(str(file) + ".sig").write_text("test signature")
        (Path(folder)/"BUILD_INFO.json").write_text(json.dumps({
            "source_commit": SHA, "application_version": plan["version"],
            "signature_verified": True,
            "sha256": hashlib.sha256(file.read_bytes()).hexdigest()}))
        return plan, file

    def test_only_successful_main_native_ci_is_a_release_source(self):
        self.assertEqual(release.validate_run(RUN)["version"], "0.2.1250")
        for key, value in [("event","pull_request"),("head_branch","feature"),
                           ("conclusion","failure"),("status","in_progress"),
                           ("path","other.yml"),("head_sha","bad"),
                           ("head_repository",{"full_name":"someone/else"})]:
            run = copy.deepcopy(RUN); run[key] = value
            with self.assertRaises(ValueError): release.validate_run(run)

    def test_versions_are_monotonic_across_windows_component_rollover(self):
        versions = [release.version_for(i) for i in [1,59999,60000,60001,120000]]
        self.assertEqual(sorted(map(release.version_tuple,versions)), list(map(release.version_tuple,versions)))
        for number in [0,-1,True,3.5]:
            with self.assertRaises(ValueError): release.version_for(number)

    def test_missing_pending_failed_and_untrusted_checks_block(self):
        checks = good_checks()
        self.assertTrue(release.check_state(checks))
        self.assertFalse(release.check_state(checks[:-1]))
        checks[-1]["status"] = "in_progress"
        self.assertFalse(release.check_state(checks))
        checks[-1].update(status="completed", conclusion="failure")
        with self.assertRaises(ValueError): release.check_state(checks)
        checks = good_checks()
        checks[-1]["app"]["slug"] = "other-app"
        self.assertFalse(release.check_state(checks))

    def test_skipped_workflow_run_jobs_do_not_replace_real_push_results(self):
        checks = good_checks()
        checks.append({**checks[0], "id":999, "conclusion":"skipped"})
        self.assertTrue(release.check_state(checks))
        checks.append({**checks[0], "id":1000, "conclusion":"failure"})
        with self.assertRaises(ValueError): release.check_state(checks)

    def test_check_fetch_includes_previous_executions_and_paginates(self):
        class Pages(release.GitHub):
            def request(self, path):
                self.paths.append(path)
                return {"check_runs": [{}]*100 if len(self.paths)==1 else good_checks()}
        api=Pages(); api.paths=[]
        self.assertEqual(len(api.checks(SHA)),100+len(release.REQUIRED))
        self.assertEqual(api.paths,[f"/commits/{SHA}/check-runs?filter=all&per_page=100&page={i}" for i in [1,2]])

    def test_applicable_optional_validation_blocks_but_publication_does_not(self):
        checks=good_checks()
        active={"id":900,"status":"in_progress","conclusion":None,"app":{"slug":"github-actions"}}
        checks.extend([{**active,"name":name} for name in ["plan","publish"]])
        self.assertTrue(release.check_state(checks))
        checks.append({**active,"name":"client"})
        self.assertFalse(release.check_state(checks))
        checks[-1].update(status="completed",conclusion="failure")
        with self.assertRaises(ValueError): release.check_state(checks)
        checks[-1]["conclusion"]="success"
        self.assertTrue(release.check_state(checks))

    def test_superseded_commit_and_newer_release_are_skipped(self):
        api = FakeGitHub(); api.main = "b"*40
        self.assertIsNone(release.plan_release(api,77,attempts=1))
        api.main = SHA; api.latest = {"tag_name":"desktop-v0.3.0"}
        self.assertIsNone(release.plan_release(api,77,attempts=1))
        self.assertFalse(api.writes)

    def test_release_manifest_provenance_digest_and_signature_required(self):
        with tempfile.TemporaryDirectory() as folder:
            plan, file = self.fixture(folder)
            release.write_manifest(folder,plan)
            manifest = json.loads((Path(folder)/"latest.json").read_text())
            self.assertIn("Systems%20Modeler%20Pro",manifest["platforms"]["windows-x86_64"]["url"])
            file.write_bytes(b"changed")
            with self.assertRaises(ValueError): release.write_manifest(folder,plan)
            plan, file = self.fixture(folder)
            Path(str(file)+".sig").unlink()
            with self.assertRaises(ValueError): release.write_manifest(folder,plan)
            plan, file = self.fixture(folder)
            plan["sha"] = "b"*40
            with self.assertRaises(ValueError): release.write_manifest(folder,plan)
            plan, file = self.fixture(folder)
            plan["tag"] = "desktop-v0.2.0"
            with self.assertRaises(ValueError): release.write_manifest(folder,plan)
            plan, file = self.fixture(folder)
            info = Path(folder)/"BUILD_INFO.json"
            metadata = json.loads(info.read_text()); metadata["signature_verified"] = False
            info.write_text(json.dumps(metadata))
            with self.assertRaises(ValueError): release.write_manifest(folder,plan)

    def test_draft_uploads_precede_publication(self):
        with tempfile.TemporaryDirectory() as folder:
            plan,_ = self.fixture(folder); api = FakeGitHub()
            self.assertTrue(release.publish(api,folder,plan))
            self.assertTrue(api.writes[0][2]["draft"])
            self.assertEqual(len(api.uploaded),4)
            self.assertEqual(api.writes[-1][2],{"draft":False,"make_latest":"true"})

    def test_partial_upload_or_new_main_never_publishes(self):
        with tempfile.TemporaryDirectory() as folder:
            plan,_ = self.fixture(folder); api = FakeGitHub(); api.fail_upload=True
            with self.assertRaises(RuntimeError): release.publish(api,folder,plan)
            self.assertEqual(len(api.writes),1)
            api = FakeGitHub(); api.advance_on_upload=True
            self.assertFalse(release.publish(api,folder,plan))
            self.assertEqual(len(api.writes),1)
            api = FakeGitHub(); api.fail_check_on_upload=True
            with self.assertRaises(ValueError): release.publish(api,folder,plan)
            self.assertEqual(len(api.writes),1)

    def test_existing_release_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as folder:
            plan,_ = self.fixture(folder); api = FakeGitHub(); api.existing={"id":99}
            with self.assertRaises(ValueError): release.publish(api,folder,plan)
            self.assertFalse(api.writes)
            self.assertFalse(api.uploaded)

if __name__ == "__main__": unittest.main()
