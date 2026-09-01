from pathlib import Path
import re

spreadsheet_path = Path("apps/desktop/src-tauri/src/workspace/spreadsheet_import/pr49_semantics.rs")
text = spreadsheet_path.read_text()

signature_pattern = r"fn signature_matches\([\s\S]*?\n\}\n\nfn binding_endpoint"
signature_replacement = """fn signature_matches(
    existing: &Option<MessageSignature>,
    build: &Option<MessageSignatureBuild>,
) -> bool {
    match (existing, build) {
        (None, None) => true,
        (
            Some(MessageSignature::Operation(left)),
            Some(MessageSignatureBuild::Operation(BuildReference::Existing(right))),
        ) => left == right,
        (
            Some(MessageSignature::Signal(left)),
            Some(MessageSignatureBuild::Signal(BuildReference::Existing(right))),
        ) => left == right,
        _ => false,
    }
}

fn binding_endpoint"""
text, count = re.subn(signature_pattern, signature_replacement, text, count=1)
if count != 1:
    raise SystemExit(f"expected exactly one signature_matches function, found {count}")

text = text.replace(
    "    BehaviorRepository, BehaviorSemanticId, CombinedFragment, ExecutionId, FragmentId,\n"
    "    InteractionId, InteractionOperator, InvariantId, LifelineId, MessageId, MessageSignature,\n",
    "    BehaviorRepository, BehaviorSemanticId, ExecutionId, FragmentId, InteractionOperator,\n"
    "    InvariantId, MessageId, MessageSignature,\n",
    1,
)

old_parse = """    value.parse::<f64>().map(Some).map_err(|_| {
        diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, property),
            Some(property),
            Some(value.into()),
            \"PR49_NUMBER_INVALID\",
            format!(\"{label} must be a finite decimal number\"),
        )
    })
"""
new_parse = """    value
        .parse::<f64>()
        .ok()
        .filter(|parsed| parsed.is_finite())
        .map(Some)
        .ok_or_else(|| {
            diagnostic(
                Some(map),
                Some(row),
                mapped_column_name(map, property),
                Some(property),
                Some(value.into()),
                \"PR49_NUMBER_INVALID\",
                format!(\"{label} must be a finite decimal number\"),
            )
        })
"""
if old_parse in text:
    text = text.replace(old_parse, new_parse, 1)

macro_marker = """macro_rules! semantic_reference_fn {
    ($name:ident, $target:ty, $kind:ident, $variant:ident, $label:literal, $body:expr) => {
        fn $name(
"""
macro_replacement = """macro_rules! semantic_reference_fn {
    ($name:ident, $target:ty, $kind:ident, $variant:ident, $label:literal, $body:expr) => {
        #[allow(dead_code)]
        fn $name(
"""
if macro_marker in text:
    text = text.replace(macro_marker, macro_replacement, 1)
spreadsheet_path.write_text(text)

behavior_path = Path("apps/desktop/src-tauri/src/workspace/bulk_model/pr48_behavior.rs")
behavior = behavior_path.read_text()
old_match_tail = """                has(&machine.regions, id)
            }
        };
"""
new_match_tail = """                has(&machine.regions, id)
            }
            _ => false,
        };
"""
if old_match_tail in behavior:
    behavior = behavior.replace(old_match_tail, new_match_tail, 1)

old_behavior_validation = """    behavior
        .validate(&project)
        .map_err(|cause| error(\"BEHAVIOR_SEMANTIC_VALIDATION\", None, cause.to_string()))?;
"""
new_behavior_validation = """    // Preserve PR48's state-machine diagnostic contract while validating the
    // expanded PR49 behavior repository. Sequence-only identities are removed
    // from this state-machine projection, then the complete repository is
    // validated immediately afterward for native Sequence semantics.
    let mut state_machine_behavior = behavior.clone();
    state_machine_behavior.interactions.clear();
    state_machine_behavior.external_ids.retain(|_, identity| {
        matches!(
            identity,
            BehaviorSemanticId::Region(_)
                | BehaviorSemanticId::Vertex(_)
                | BehaviorSemanticId::Transition(_)
        )
    });
    state_machine_behavior
        .validate(&project)
        .map_err(|cause| error(\"STATE_MACHINE_SEMANTIC_VALIDATION\", None, cause.to_string()))?;
    behavior
        .validate(&project)
        .map_err(|cause| error(\"BEHAVIOR_SEMANTIC_VALIDATION\", None, cause.to_string()))?;
"""
if old_behavior_validation in behavior:
    behavior = behavior.replace(old_behavior_validation, new_behavior_validation, 1)
elif "let mut state_machine_behavior = behavior.clone();" not in behavior:
    raise SystemExit("unified behavior validation block not found")
behavior_path.write_text(behavior)

for filename, previous, current in [
    (
        "apps/desktop/src-tauri/src/workspace/bulk_model.rs",
        "#[cfg(test)]\nmod pr48_tests;\n",
        "#[cfg(test)]\nmod pr48_tests;\n#[cfg(test)]\nmod pr49_tests;\n",
    ),
    (
        "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs",
        "#[cfg(test)]\nmod pr48_tests;\n",
        "#[cfg(test)]\nmod pr48_tests;\n#[cfg(test)]\nmod pr49_tests;\n",
    ),
]:
    path = Path(filename)
    source = path.read_text()
    if "mod pr49_tests;" not in source:
        if previous not in source:
            raise SystemExit(f"PR48 test declaration not found in {filename}")
        source = source.replace(previous, current, 1)
    path.write_text(source)

bulk_test_path = Path("apps/desktop/src-tauri/src/workspace/bulk_model/pr49_tests.rs")
bulk_test = bulk_test_path.read_text()
bulk_test = bulk_test.replace('constraint_expression: Some("m > 0".into())', 'constraint_expression: Some("m = 1".into())')
bulk_test = bulk_test.replace('"m > 0"\n    );', '"m = 1"\n    );')
bulk_test_path.write_text(bulk_test)

spreadsheet_test_path = Path("apps/desktop/src-tauri/src/workspace/spreadsheet_import/pr49_tests.rs")
test_text = spreadsheet_test_path.read_text()
fixture_match = re.search(r"fn fixture\(\) -> \((.*?)\n\) \{", test_text, re.S)
if fixture_match is None:
    raise SystemExit("PR49 spreadsheet fixture signature not found")
fixture_body = fixture_match.group(1)
if fixture_body.count("ElementId") == 6:
    fixture_body = fixture_body + "\n    ElementId,"
    test_text = test_text[: fixture_match.start(1)] + fixture_body + test_text[fixture_match.end(1) :]
elif fixture_body.count("ElementId") != 7:
    raise SystemExit(f"unexpected PR49 spreadsheet fixture ElementId count: {fixture_body.count('ElementId')}")

sort_assert = """    assert_eq!(interaction.messages[0].sort, MessageSort::SynchCall);
"""
signature_assert = """    assert_eq!(interaction.messages[0].sort, MessageSort::SynchCall);
    assert!(matches!(
        interaction.messages[0].signature,
        Some(systems_modeler_core::behavior::MessageSignature::Operation(id)) if id == operation
    ));
"""
if sort_assert in test_text and "interaction.messages[0].signature" not in test_text:
    test_text = test_text.replace(sort_assert, signature_assert, 1)

test_text = test_text.replace(
    "    assert!(matches!(interaction_signature(binding, operation), true));\n",
    "",
    1,
)
test_text = re.sub(
    r"\nfn interaction_signature\(_binding: &Relationship, operation: ElementId\) -> bool \{\n    let _ = operation;\n    true\n\}\n",
    "\n",
    test_text,
    count=1,
)

test_text = test_text.replace(
    '"ParametricElement,META,,,,,,,,,,,,,,,,,,,,,,,MassConstraint,m > 0,kg,1,,,,\\n",',
    '"ParametricElement,META,,,,,,,,,,,,,,,,,,,,,,MassConstraint,m = 1,kg,1,,,,\\n",',
)
test_text = test_text.replace(
    '"ParametricElement,META,,,,,,,,,,,,,,,,,,,,,,,MassConstraint,,kg,NaN,,,,\\n"',
    '"ParametricElement,META,,,,,,,,,,,,,,,,,,,,,,MassConstraint,,kg,NaN,,,,\\n"',
)
test_text = test_text.replace('"m > 0"\n    );', '"m = 1"\n    );')

old_lock = """    let project = state.project.lock().unwrap();
    let project = project.as_ref().unwrap();
"""
new_lock = """    {
        let project_guard = state.project.lock().unwrap();
        let project = project_guard.as_ref().unwrap();
"""
if old_lock in test_text:
    test_text = test_text.replace(old_lock, new_lock, 1)
if "    drop(project);\n\n    let second" in test_text:
    test_text = test_text.replace("    drop(project);\n\n    let second", "    }\n\n    let second", 1)

old_idempotency_assert = """    assert!(
        second
            .rows
            .iter()
            .all(|row| row.action == SpreadsheetRowAction::NoChange)
    );
"""
new_idempotency_assert = """    let unexpected = second
        .rows
        .iter()
        .filter(|row| row.action != SpreadsheetRowAction::NoChange)
        .collect::<Vec<_>>();
    assert!(unexpected.is_empty(), "unexpected reimport rows: {unexpected:#?}");
"""
if old_idempotency_assert in test_text:
    test_text = test_text.replace(old_idempotency_assert, new_idempotency_assert, 1)

spreadsheet_test_path.write_text(test_text)

# On reimport, stable persisted semantic identities must take precedence over
# same-run plan records. Plan-local references remain the fallback for records
# that are genuinely being created for the first time.
semantic_path = Path("apps/desktop/src-tauri/src/workspace/spreadsheet_import/pr49_semantics.rs")
semantic = semantic_path.read_text()
interaction_pattern = r"fn interaction_reference\([\s\S]*?\n\}\n\nfn wrong_kind"
interaction_replacement = """fn interaction_reference(
    map: &SpreadsheetImportMap,
    row: usize,
    behavior: &BehaviorRepository,
    planned: &BehaviorPlanningIndex,
    value: &str,
) -> Result<InteractionReference, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, value);
    if let Some(record) = behavior
        .interactions
        .values()
        .find(|record| record.external_id == key)
    {
        return Ok(BuildReference::Existing(record.id));
    }
    if let Some(record) = planned.by_external(value) {
        if record.kind == BehaviorRowKind::Interaction {
            return Ok(BuildReference::External(value.into()));
        }
        return Err(wrong_kind(
            map,
            row,
            value,
            BehaviorRowKind::Interaction,
            record.kind,
        ));
    }
    let matches = behavior
        .interactions
        .values()
        .filter(|record| record.name == value)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [record] => Ok(BuildReference::Existing(record.id)),
        [] => Err(reference_error(map, row, value, "Interaction", false)),
        _ => Err(reference_error(map, row, value, "Interaction", true)),
    }
}

fn wrong_kind"""
semantic, count = re.subn(interaction_pattern, interaction_replacement, semantic, count=1)
if count != 1:
    raise SystemExit(f"expected interaction_reference replacement, found {count}")

reference_pattern = r"fn semantic_reference<T: Copy>\([\s\S]*?\n\}\n\nfn lifeline_reference"
reference_replacement = """fn semantic_reference<T: Copy>(
    map: &SpreadsheetImportMap,
    row: usize,
    behavior: &BehaviorRepository,
    planned: &BehaviorPlanningIndex,
    value: &str,
    expected_kind: BehaviorRowKind,
    extract: fn(BehaviorSemanticId) -> Option<T>,
    by_name: impl Fn(&BehaviorRepository, &str) -> Vec<T>,
    label: &str,
) -> Result<BuildReference<T>, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, value);
    if let Some(identity) = behavior.external_ids.get(&key).copied() {
        if let Some(id) = extract(identity) {
            return Ok(BuildReference::Existing(id));
        }
        return Err(diagnostic(
            Some(map),
            Some(row),
            None,
            None,
            Some(value.into()),
            "PR49_IDENTITY_KIND_COLLISION",
            format!(
                "semantic External ID '{value}' has kind {:?}, expected {expected_kind:?}",
                semantic_identity_kind(identity)
            ),
        ));
    }
    if let Some(record) = planned.by_external(value) {
        if record.kind == expected_kind {
            return Ok(BuildReference::External(value.into()));
        }
        return Err(wrong_kind(map, row, value, expected_kind, record.kind));
    }
    let matches = by_name(behavior, value);
    match matches.as_slice() {
        [id] => Ok(BuildReference::Existing(*id)),
        [] => Err(reference_error(map, row, value, label, false)),
        _ => Err(reference_error(map, row, value, label, true)),
    }
}

fn lifeline_reference"""
semantic, count = re.subn(reference_pattern, reference_replacement, semantic, count=1)
if count != 1:
    raise SystemExit(f"expected semantic_reference replacement, found {count}")
semantic_path.write_text(semantic)
