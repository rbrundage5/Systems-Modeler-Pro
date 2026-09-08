from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODEL_SCRIPT = ROOT / "apps/desktop/src-tauri/src/workspace/model_script.rs"
MODEL_SCRIPT_UI = ROOT / "apps/desktop/frontend/model-script-ui.js"
QUALIFICATION = ROOT / "scripts/validate_model_script_import_qualification.py"


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


# ---------------------------------------------------------------------------
# Native model-script candidate isolation.
# A semantically blank Project is authoritative: specialized repositories
# cannot legitimately contain semantics owned by another Project.  Preview and
# apply must therefore compile/build against clean Activity/Behavior stores at
# the native Rust boundary rather than trusting frontend reset ordering.
# ---------------------------------------------------------------------------
text = MODEL_SCRIPT.read_text(encoding="utf-8")

helper_anchor = "fn build_candidate(\n    script_name: &str,\n"
helper = r'''fn project_is_semantically_blank(project: &Project) -> bool {
    project.elements.len() == 1
        && project.elements.contains_key(&project.root_id)
        && project.relationships.is_empty()
}

fn reset_blank_project_specialized_candidate(
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> Result<(), String> {
    *activity
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")? =
        systems_modeler_core::ActivityRepository::default();
    activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?
        .clear();
    *workspace
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")? =
        systems_modeler_core::BehaviorRepository::default();
    workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .clear();
    Ok(())
}

'''
if "fn project_is_semantically_blank(project: &Project)" not in text:
    text = replace_once(text, helper_anchor, helper + helper_anchor, "insert blank-project helpers")

build_start = text.index("fn build_candidate(\n")
build_end = text.index("\nfn commit_candidate(", build_start)
prefix, build, suffix = text[:build_start], text[build_start:build_end], text[build_end:]

project_to_activities = '''        })?;\n    let activities = activity\n        .repository\n'''
project_to_activities_new = '''        })?;\n    let blank_project = project_is_semantically_blank(&project);\n    let mut activities = activity\n        .repository\n'''
build = replace_once(build, project_to_activities, project_to_activities_new, "mark blank project / mutable activities")

behavior_to_compile = '''        })?\n        .clone();\n    let compiled = compile_script(script_name, source, &project, &activities, &behavior).map_err(\n'''
behavior_to_compile_new = '''        })?\n        .clone();\n    let mut behavior = behavior;\n    if blank_project {\n        // A root-only Project cannot legitimately own pre-existing Activity or\n        // Behavior semantics. Compile as a fresh specialized import so stale\n        // External IDs cannot turn Create operations into Updates.\n        activities = systems_modeler_core::ActivityRepository::default();\n        behavior = systems_modeler_core::BehaviorRepository::default();\n    }\n    let compiled = compile_script(script_name, source, &project, &activities, &behavior).map_err(\n'''
build = replace_once(build, behavior_to_compile, behavior_to_compile_new, "compile against clean specialized stores")

candidate_to_apply = '''        })?;\n    if let Err(preview) =\n        apply_unified_model_build(&compiled.plan, &candidate_workspace, &candidate_activity)\n'''
candidate_to_apply_new = '''        })?;\n    if blank_project {\n        reset_blank_project_specialized_candidate(&candidate_workspace, &candidate_activity)\n            .map_err(|reason| ModelScriptPreview {\n                host: SCRIPT_HOST,\n                applied: false,\n                source_namespace: compiled.document.source_namespace.clone(),\n                items: compiled.items.clone(),\n                diagnostics: vec![diag(\n                    script_name,\n                    None,\n                    Some("candidate".into()),\n                    None,\n                    "BLANK_PROJECT_SPECIALIZED_RESET_FAILED",\n                    reason,\n                )],\n            })?;\n    }\n    if let Err(preview) =\n        apply_unified_model_build(&compiled.plan, &candidate_workspace, &candidate_activity)\n'''
build = replace_once(build, candidate_to_apply, candidate_to_apply_new, "reset candidate specialized stores")
text = prefix + build + suffix

# Regression: reproduce the observed orphan-Project-UUID failure natively, then
# prove a blank model-script preview discards it without mutating the live store.
test_anchor = '''    #[test]\n    fn dry_run_is_non_mutating_and_apply_is_atomic() {\n'''
test = r'''    #[test]
    fn blank_model_script_import_discards_orphaned_specialized_state_before_native_preview() {
        let (workspace, activity) = states();
        let current_project = workspace.project.lock().unwrap().clone().unwrap();

        // Simulate the exact production failure class: Activity state from an
        // older Project survives while the current Project is freshly blank.
        // Native Activity validation then reports ModelError::ElementNotFound
        // for the old Project root UUID.
        let old_project = Project::new("Old Project");
        let mut stale = systems_modeler_core::ActivityRepository::default();
        stale
            .create_activity(&old_project, old_project.root_id, None, "Stale Activity")
            .unwrap();
        assert!(stale.validate(&current_project).is_err());
        *activity.repository.lock().unwrap() = stale;

        let source = r#"{
          "source_namespace":"blank-stale-regression",
          "operations":[
            {"op":"activity","external_id":"ACT","name":"Fresh Activity","owner":"$root"},
            {"op":"activity_node","external_id":"I","activity":"handle:ACT","name":"Initial","node":{"kind":"initial"}},
            {"op":"activity_node","external_id":"A","activity":"handle:ACT","name":"Work","node":{"kind":"opaque_action","body":"work()"}},
            {"op":"activity_node","external_id":"F","activity":"handle:ACT","name":"Final","node":{"kind":"activity_final"}},
            {"op":"activity_edge","external_id":"E1","activity":"handle:ACT","name":"","kind":"ControlFlow","source":"handle:I","target":"handle:A"},
            {"op":"activity_edge","external_id":"E2","activity":"handle:ACT","name":"","kind":"ControlFlow","source":"handle:A","target":"handle:F"}
          ],
          "diagrams":[]
        }"#;

        let preview = preview_impl("blank-stale.groovy", source, &workspace, &activity);
        assert!(preview.valid(), "{:?}", preview.diagnostics);

        // Dry run remains non-mutating: the live stale repository is untouched.
        assert_eq!(activity.repository.lock().unwrap().activities.len(), 1);
        assert_eq!(
            activity
                .repository
                .lock()
                .unwrap()
                .activities
                .values()
                .next()
                .unwrap()
                .name,
            "Stale Activity"
        );

        // The native candidate itself contains only the newly scripted Activity
        // and validates against the current blank Project.
        let (_, candidate_workspace, candidate_activity) =
            build_candidate("blank-stale.groovy", source, &workspace, &activity).unwrap();
        let candidate_project = candidate_workspace.project.lock().unwrap().clone().unwrap();
        let candidate_repository = candidate_activity.repository.lock().unwrap();
        assert_eq!(candidate_repository.activities.len(), 1);
        assert_eq!(
            candidate_repository.activities.values().next().unwrap().name,
            "Fresh Activity"
        );
        candidate_repository.validate(&candidate_project).unwrap();
    }

'''
if "blank_model_script_import_discards_orphaned_specialized_state_before_native_preview" not in text:
    text = replace_once(text, test_anchor, test + test_anchor, "insert orphaned Activity regression")

MODEL_SCRIPT.write_text(text, encoding="utf-8")

# ---------------------------------------------------------------------------
# Frontend: blankness is a semantic Project property. Stale presentation rows
# must not prevent the frontend from requesting the authoritative native reset.
# ---------------------------------------------------------------------------
ui = MODEL_SCRIPT_UI.read_text(encoding="utf-8")
old_blank = '''    return elements.length === 1\n      && String(elements[0]?.id || '') === String(project.root_id || '')\n      && relationships.length === 0\n      && (state.snapshot?.diagrams || []).length === 0\n      && (state.snapshot?.ibd_diagrams || []).length === 0;\n'''
new_blank = '''    return elements.length === 1\n      && String(elements[0]?.id || '') === String(project.root_id || '')\n      && relationships.length === 0;\n'''
if old_blank in ui:
    ui = replace_once(ui, old_blank, new_blank, "semantic blank frontend predicate")
elif new_blank not in ui:
    raise RuntimeError("frontend blank-project predicate did not match expected PR65 form")
MODEL_SCRIPT_UI.write_text(ui, encoding="utf-8")

# ---------------------------------------------------------------------------
# CI/static qualification must protect the native boundary, not only JS wiring.
# ---------------------------------------------------------------------------
qual = QUALIFICATION.read_text(encoding="utf-8")
needle = '''assert "second_diagram_count, 9," in model_script_rs\n\n'''
insert = '''assert "second_diagram_count, 9," in model_script_rs\nassert "fn project_is_semantically_blank(project: &Project)" in model_script_rs\nassert "reset_blank_project_specialized_candidate" in model_script_rs\nassert "blank_model_script_import_discards_orphaned_specialized_state_before_native_preview" in model_script_rs\nassert "BLANK_PROJECT_SPECIALIZED_RESET_FAILED" in model_script_rs\n\n'''
if "blank_model_script_import_discards_orphaned_specialized_state_before_native_preview" not in qual:
    qual = replace_once(qual, needle, insert, "native blank-state qualification assertions")

# Presentation remnants must not cause a root-only Project to bypass reset.
qual_anchor = '''for required in [\n'''
extra = '''assert "&& (state.snapshot?.diagrams || []).length === 0" not in model_script_ui\nassert "&& (state.snapshot?.ibd_diagrams || []).length === 0" not in model_script_ui\n\n'''
if extra.strip() not in qual:
    qual = replace_once(qual, qual_anchor, extra + qual_anchor, "frontend semantic-blank qualification")
QUALIFICATION.write_text(qual, encoding="utf-8")

print("PR66 native blank-project model-script isolation patch applied")
