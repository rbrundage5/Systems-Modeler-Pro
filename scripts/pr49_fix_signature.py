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

bulk_path = Path("apps/desktop/src-tauri/src/workspace/bulk_model.rs")
bulk = bulk_path.read_text()
needle = "#[cfg(test)]\nmod pr48_tests;\n"
addition = "#[cfg(test)]\nmod pr48_tests;\n#[cfg(test)]\nmod pr49_tests;\n"
if "mod pr49_tests;" not in bulk:
    if needle not in bulk:
        raise SystemExit("PR48 bulk-model test declaration not found")
    bulk = bulk.replace(needle, addition, 1)
bulk_path.write_text(bulk)
