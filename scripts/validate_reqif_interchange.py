from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def require(path: str, needles: list[str]) -> None:
    source = text(path)
    missing = [needle for needle in needles if needle not in source]
    if missing:
        raise SystemExit(f"{path} missing ReqIF contract markers: {missing}")


require(
    "apps/desktop/src-tauri/src/workspace/reqif_interchange.rs",
    [
        "ReqifImportConfiguration",
        "ReqifSynchronizationPolicy",
        "AuthoritativeReqifScope",
        "ReqifAction",
        "ReqifValue::Xhtml",
        "validate_references",
        "serialize_reqif",
    ],
)
require(
    "apps/desktop/src-tauri/src/workspace/reqif_runtime.rs",
    [
        "ModelBuildPlan",
        "apply_unified_model_build",
        "preview_reqif_import",
        "apply_reqif_import",
        "export_reqif",
        "stage_reqif_upload",
        "discard_staged_reqif",
        "portable_from_states",
        "save_reqif_metadata",
        "load_reqif_metadata",
        "AuthoritativeReqifScope",
    ],
)
require(
    "apps/desktop/src-tauri/src/workspace.rs",
    ["reqif_exchange", "save_reqif_metadata", "load_reqif_metadata"],
)
require(
    "apps/desktop/src-tauri/src/workspace/model_script.rs",
    ["reqif_exchange", "ReqIF exchange lock poisoned"],
)
require(
    "apps/desktop/src-tauri/src/main.rs",
    [
        "preview_reqif_import",
        "apply_reqif_import",
        "export_reqif",
        "stage_reqif_upload",
        "discard_staged_reqif",
    ],
)
require(
    "apps/desktop/frontend/reqif-ui.js",
    [
        ".reqif,.reqifz",
        "source_namespace",
        "target_scope",
        "preview_reqif_import",
        "apply_reqif_import",
        "export_reqif",
        "stage_reqif_upload",
        "discard_staged_reqif",
    ],
)
require("apps/desktop/frontend/index.html", ['<script src="reqif-ui.js"></script>'])

rust_reqif = text("apps/desktop/src-tauri/src/workspace/reqif_runtime.rs") + text(
    "apps/desktop/src-tauri/src/workspace/reqif_interchange.rs"
)
for forbidden in ["mod xmi", "XmiImporter", "ProfileRepository"]:
    if forbidden in rust_reqif:
        raise SystemExit(f"PR52 scope violation detected in ReqIF Rust: {forbidden}")

print("ReqIF integration contract passed")
