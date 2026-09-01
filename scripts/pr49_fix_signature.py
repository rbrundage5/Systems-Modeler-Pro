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
spreadsheet_test_path.write_text(test_text)
