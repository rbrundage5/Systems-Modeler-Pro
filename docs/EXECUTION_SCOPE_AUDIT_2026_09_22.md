# Execution scope and unexpected-code audit

Date: 22 September 2026. Product baseline: `848b9e9650627ff819cea3d2e2cff382fa77d3af`.
Scope: the user's follow-up request to check for bad, unrelated or unexpected execution in Systems-Modeler-Pro. This supplements the [tool/SysML review](TOOL_SYSML_REVIEW_2026_09_22.md).

## Conclusion

**No evidence of unrelated or deliberately malicious behavior was found in the inspected first-party application code.** The traced privileged operations have modeling, collaboration, persistence or software-delivery purposes. This conclusion is narrower than a guarantee that every dependency or installed binary is safe, or that the application contains no defects.

One unnecessary execution path was found in development automation: the obsolete PR38 workflow can apply historical source replacements and push a commit on its old branch. A separately scoped deletion is prepared in [PR129](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/129), candidate `c6264b21d5ef9f79f299300b9a042e09a25b17f2`. It deletes the workflow and its sole helper. It is a cleanup of historical automation, not a finding of malware. The candidate is not merged; current main still contains those files.

The existing `csp: null`, broad frontend-supplied native file paths, dependency qualification and input resource budgets remain hardening work. They must not be confused with evidence that the tool currently performs unrelated operations.

## What was inspected

- All 394 tracked baseline paths were inventoried, including hidden repository configuration, workflows, scripts, application assets, build manifests and the lockfile. Source searches covered execution, network, filesystem traversal, dynamic loading, credentials, persistence and background operations; sensitive paths were then traced manually.
- Reviewed the desktop entry point/build script, Tauri configuration and plugin registration, startup updater and state gate, collaboration client/server, model-script parser, execution expression parser, import/export and temporary-file handlers, application preferences, installer hook, release scripts and workflow triggers/permissions.
- Inspected ten bundled XLSX fixture archives for VBA projects, external-link parts, embedded objects and selected network-capable formula patterns. No matching parts/patterns were found. This was archive inspection, not execution in Excel or a complete office-document security scanner.
- Read only repository files and the approved project brief. Did not access personal directories, real credentials or unrelated services. Did not execute the bootstrap, signing setup, system dependency installer or Windows installer. No workers were dispatched.
- No native desktop tracing, dependency-source audit, external advisory lookup, binary provenance attestation or independent security review was performed. No statement below extends to an unknown installed binary or a different commit.

## Expected execution and data access

| Surface | What actually happens | Purpose and controls observed |
| --- | --- | --- |
| Desktop startup | Loads bundled local HTML/JS and native Rust state | No external script/CDN asset URLs found in first-party HTML/CSS/JS. `build.rs` calls Tauri build and watches the updater public-key environment variable |
| Model behavior/parametric execution | Rust interprets bounded model expressions and simulation operations | No general-purpose host shell, JavaScript `eval`, dynamic `Function`, native library loader or direct process launcher found in the inspected first-party application sources |
| `.groovy` / model-script import | Extracts a JSON `modelScript` payload and compiles it into Rust ModelBuildPlan operations | No JVM/Groovy runtime is embedded. Other surrounding Groovy text is not executed. The importer exposes modeling operations, not a filesystem/process/network API |
| Native project files | Reads/writes SQLite model and presentation data at an explicitly supplied project path | Directly related to Open/Save. Existing transaction defects remain in the main review; this is not a file-access sandbox |
| Spreadsheet, ReqIF, XMI and JSON exchange | Parses user-selected data, stages bounded uploads, writes requested exports | No first-party external URL resolver or script runner found in these adapters. XML namespace strings are identifiers, not evidence of network requests |
| Temporary uploads | Writes randomized `systems-modeler-*` files in the OS temporary directory | Discard handlers check the temporary parent and expected filename prefix. These are import staging files, not a general recursive deletion API |
| Workspace preferences | Reads/writes `workspace-preferences.json` under the application's config directory | Viewport, frame and panel settings. Related frontend localStorage records contain UI preferences/palette organization |
| Collaboration recovery | Stores pending model operations in `collaboration-outbox.sqlite` under application data | Needed for retry/restart recovery; server/actor identity is bound to pending operations. Model content can be present in this local database; it is not a credential vault |
| Collaboration client | Sends authenticated requests to the server origin the user enters | HTTPS required except numeric loopback HTTP; embedded credentials/query/fragment/path rejected; redirects disabled; proxy bypass configured; bounded responses and timeouts |
| Collaboration server | Separate explicitly started executable; listens on `127.0.0.1:4783` | The server rejects non-loopback listeners. Remote deployment requires an HTTPS proxy. Requests authenticate before project operations; grants and model command types limit actions |
| Collaboration polling | Refreshes the visible shared-project dialog and presence | Five-second model refresh is gated by dialog/draft/pending state; presence has a ten-second timer. These are collaboration operations, not general background telemetry |
| Update check | Production Windows startup checks this repository's latest-release metadata | Development/unsigned configurations skip automatic updates. Offline failure still allows opening the application; no first-party model upload is part of the update path |
| Update installation | User selects Install before opening the modeling session | Only the updater window may invoke update commands; checked metadata and embedded public key are held by Rust. Frontend supplies no arbitrary URL, installer path or bytes. Tauri verifies the update signature before installation |
| Installer process handling | Installer checks for running copies of this application and refuses replacement | The custom NSIS hook avoids forcibly closing an active modeling process. CI separately launches/stops its own test process for installation smoke tests |
| Build/release infrastructure | Downloads build dependencies, compiles, signs, tests and publishes qualified artifacts | Legitimate development/distribution operations, separate from the desktop modeling runtime. Release automation targets this repository and verifies the source is qualified main CI |
| Repository agent hook | Prints a refusal and exits nonzero while qualification is incomplete | It does not launch agents or provide runtime application behavior; delegation remains disabled |

The desktop therefore legitimately accesses more than a project file: imports/exports, application settings, temporary staging and collaboration recovery. That is expected scope. No first-party code that recursively scans personal/home directories, uploads unrelated files, captures global keyboard input, installs an unrelated service, mines cryptocurrency or downloads an unrelated program was identified in the inspected paths.

## Network and execution authority

### Application network destinations

The first-party runtime network authorities traced were:

1. **Updater:** fixed metadata endpoint `https://github.com/rbrundage5/Systems-Modeler-Pro/releases/latest/download/latest.json`; candidate installer URL must begin with this repository's HTTPS release-download prefix and end with `-setup.exe`. Installation additionally depends on signature verification. GitHub may deliver release assets through its download infrastructure; the prefix check is not a proof that no HTTP redirect occurs inside the dependency.
2. **Collaboration:** the explicitly entered server origin, validated by `server_url` in [collaboration.rs](../apps/desktop/src-tauri/src/collaboration.rs). Requests use that session's token and typed shared-project operations. Credentials are held in session memory, not intentionally serialized into model snapshots or outbox payloads.
3. **Separately launched server:** inbound loopback HTTP for authenticated project collaboration, implemented in [apps/server](../apps/server/src/lib.rs). The desktop startup does not launch this server or open its listener.

No frontend `fetch`, WebSocket, XMLHttpRequest, EventSource or sendBeacon call was found. Native Rust still performs the intended updater/collaboration networking. No analytics/advertising/crash-upload client was identified. SVG and OMG XML namespace URLs, example domains, and invalid-address test fixtures were classified as data/tests rather than outbound requests.

### Native execution

The desktop registers only the updater plugin in the inspected [main.rs](../apps/desktop/src-tauri/src/main.rs) startup; no generic shell, opener, filesystem or autostart plugin is declared in its manifest. No first-party application call to `std::process::Command`, OS shell evaluation, dynamic library loading or arbitrary SQL extension loading was found.

The updater deliberately executes an installer through the Tauri updater dependency. This is required software delivery, not an unrestricted user-script execution API. Its actual native signature/install behavior was covered by earlier integration qualification, not re-executed in this audit.

The [model-script host](../apps/desktop/src-tauri/src/workspace/model_script.rs), [execution expression parser](../crates/model-core/src/execution_expression.rs) and parametric parser interpret model data. “Execute Activity” and “import Groovy” do not mean running arbitrary code on the operating system. Their semantic completeness and resource limits remain separate correctness concerns.

## Findings and required follow-up

| ID | Finding | Disposition and acceptance |
| --- | --- | --- |
| EXEC-01 | Obsolete PR38 workflow rewrites source and pushes with repository write permission | **Removal prepared in PR129.** Only `.github/workflows/pr38-bootstrap.yml`, `scripts/pr38_bootstrap.py` and a maintenance record are changed. All seven remaining baseline workflows are byte-identical. No executable caller of the helper remains in the candidate. Review/merge pending |
| EXEC-02 | Frontend CSP is disabled (`csp: null`) | **Hardening gap, no exploit demonstrated.** Add a compatible restrictive policy and test native IPC, local assets, styles and import rendering. Do not add broad remote/script permissions to make tests pass |
| EXEC-03 | Native file commands trust frontend-supplied paths; command/window permissions need explicit qualification | **Hardening gap.** A normal desktop must access selected files, but current path-taking commands are not proof of user-selection authority. Design scoped file handles/path grants and main/updater window command boundaries; test rejection of unauthorized paths/windows. No unrelated file-read action was observed |
| EXEC-04 | Dependency execution is outside a first-party source-only assurance | **Qualification gap.** Cargo.lock has 520 registry packages with checksums and four workspace packages, with no git/path dependency outside the workspace identified in the inspected manifests/lock. CI uses version-tagged Actions, floating stable Rust, and a version-pinned npm Tauri CLI installed without a committed npm lock. Pin build tools/actions more tightly and qualify advisories, resolved dependencies and release provenance before making supply-chain assurances |
| EXEC-05 | Imported data and renderer boundaries require adversarial native testing | **Qualification gap.** Size checks and string escaping exist, but this is not proof against every oversized archive, recursive model, parser or HTML injection case. Test hostile documents as data, with network/process invocation denied and model/file state unchanged on rejection. Preserve resource limits and contextual diagnostics |
| EXEC-06 | Three old behavior scripts remain bundled source despite not being loaded | **Cleanup candidate carried from R20.** They contain obsolete UI patches, not an identified unrelated execution channel. Remove in their own frontend maintenance leaf after caller/package verification; do not reactivate |

EXEC-01's workflow trigger is the historical `agent/pr38-spreadsheet-mapping` branch, not ordinary main pushes. Removing it from main does not rewrite an old branch containing a historical copy; such a branch must not be reused as an active automation entry point. No remote branch was deleted or force-pushed during this audit.

The existing release workflow's write permission is retained because publishing this application's installer needs it. PR validation uses disposable test signing keys; the production signing secret is scoped to the production publish build step. The hosted setup script's package-manager/toolchain commands and the owner-run signing setup are legitimate preparation tools and were inspected, not executed. No guard, test or approval policy was weakened.

## Evidence and limitations

Executed in the repository checkout:

| Check | Result |
| --- | --- |
| Tracked-path and source execution/network/filesystem scans | 394 baseline files inventoried; no matching first-party generic host-execution calls, frontend network calls, recursive filesystem scan APIs or remote frontend asset references in the searched source patterns |
| Tracked binaries/symlinks | No executable-library binary extensions, ELF/WASM magic or tracked symlinks found in the selected checks; not a general steganography scanner |
| Cargo lock source/checksum inventory | 520 crates.io registry entries with checksums, four workspace entries; no extra lockfile source origin |
| Ten XLSX fixtures | No VBA/external-link/embedded-object parts or selected network-formula patterns found |
| Selected credential-pattern scan | No PEM private-key, GitHub token or AWS access-key pattern matches; values would not be printed. This does not prove absence of every secret format |
| `node --test scripts/test_startup_updater.cjs` | **6 passed**: installation requires interaction, offline/late results handled, failure leaves existing app available, repeated clicks guarded; native IPC mocked |
| `python3 -m unittest discover -s scripts -p test_desktop_release.py` | **11 passed**: release planning/manifest policy; GitHub operations mocked, no release published |
| PR129 deletion scope and references | Two intended executable-file deletions, one record; seven other workflows byte-identical; no remaining executable references to the removed helper; diff checks passed |

Pattern scans are supporting evidence, not formal proof of absence. Dependencies can run code at build/runtime even when their behavior is not visible in the repository. Installed Windows behavior, third-party libraries, service deployment, every HTML sink and the operating system remain outside this source-only verification. The previously documented cancellation/persistence/semantic defects are still real and are not cleared by finding no unrelated payload.

The useful assurance from this audit is specific: **the inspected first-party execution paths are explainable by the SysML tool's intended functions; one obsolete source-writing automation path has a concrete removal candidate; remaining permission and security work is explicitly recorded.**

## Hardening implementation follow-up

The user authorized implementation after this baseline audit. The following
separate candidates implement bounded controls; none changes the audit baseline
or constitutes a merged/released or independently approved security guarantee.
No worker, dependency download, personal-file access or automatic merge was used.

| Finding | Candidate | Implemented control and qualification boundary |
| --- | --- | --- |
| EXEC-01 | [PR129](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/129), `c6264b21d5ef9f79f299300b9a042e09a25b17f2` | Deletes the obsolete source-writing bootstrap workflow/helper. All four triggered workflows passed; historical branch copies are not rewritten |
| EXEC-02 | [PR130](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/130), `60b8397fdaed3fb03197cdc1f0e5d7353df20fae` | Default-deny CSP, bundled scripts, IPC-only browser connections, no frames/forms/objects/workers; keeps required dynamic styles. Adds three policy/asset regressions. Effective installed-webview behavior remains an acceptance gate |
| EXEC-03a | [PR131](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/131), `97c89990b6ac98484c6a9eff1c54ce0fc6d3eb3a` | Rust checks the native webview label and exact top-level document URL before custom command dispatch. Main and updater command groups are isolated. Four policy regressions cover valid and denied contexts. This is not an iframe-origin attestation or per-file authorization |
| EXEC-06 | [PR132](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/132), `74c38d96ff5debe972c3f1d48964b5ad2ae6498b` | Deletes three unloaded legacy scripts and the one obsolete syntax-check list entry. All 65 retained frontend files are unchanged; 33 remaining syntax checks and the existing Behavior/Rust-authority validators passed locally |

The PR descriptions record final-head CI results and any remaining gates. An
installer launch smoke test only establishes that its process stays running; it
does not establish that CSP permits every diagram renderer or that IPC rejection
works in a rendered native window. Original CI found formatting issues and the
obsolete syntax-check filename; those were corrected without weakening policy,
active-file checks, jobs or permissions.

Required installed-desktop acceptance uses disposable fixtures and canaries:

1. For each supported platform URL, open the main window and signed startup updater;
   exercise Check/Install/Open, including offline/failure paths. Verify main cannot
   invoke update operations and updater cannot read/write/import model files.
2. Author, move/resize, edit properties, undo/redo, save/reopen and import/export
   representative models in all nine families. Check injected styles, SVG markers,
   labels, dialogs and generated downloads under the effective CSP.
3. Attempt inline/event-handler/remote scripts, frames, browser network calls and
   non-bundled document IPC in the isolated test build. Verify rejection and that
   model state and synthetic unrelated files are unchanged. Do not use real personal
   files or live credentials as test targets.
4. Record the tested commit, installer hash, platform/webview version, effective
   policy and observed results. Obtain independent review before merging.

The remaining larger work must stay split into dependency-ordered leaves:

| Next leaf | Concrete scope | Acceptance before closure |
| --- | --- | --- |
| EXEC-03b | Native file-selection/grant foundation, then migrate project Open/Save in its own leaf | Cancel creates no authority or model/history change; forged/expired grants, wrong access mode and path substitution reject; selected-file Open/Save/reopen works |
| EXEC-03c | Migrate import/export adapters and temporary-upload ownership after the grant foundation | Each adapter only accesses granted inputs/outputs or its own session staging; failed/repeated discard cannot delete another session's synthetic file |
| EXEC-04a | Qualify and immutably pin build actions/tools and resolved dependencies in a separate setup-maintenance leaf | Approved source/advisory evidence, unchanged supported builds and recorded release provenance; no claim that lockfile checksums prove absence of malicious dependency code |
| EXEC-05a | Bound and adversarially test one importer/parser at a time | Oversized/deep/recursive/hostile input rejects within measured budgets, without external resolution, host execution or partial model mutation; valid round trips remain supported |

These open items, rendered acceptance and independent review prevent an absolute
assurance claim. The earlier data-integrity and SysML findings also remain open;
the security candidates do not silently close them.
