from pathlib import Path

path = Path("apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs")
text = path.read_text()
validation = '''    if !matches!(map.element_kind, ElementKind::ProxyPort | ElementKind::FullPort)
        && has_property(SpreadsheetSemanticProperty::Conjugated)
    {
        return Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, SpreadsheetSemanticProperty::Conjugated),
            Some(SpreadsheetSemanticProperty::Conjugated),
            None,
            "SEMANTIC_PROPERTY_INVALID",
            "Conjugated can be mapped only for ProxyPort or FullPort",
        ));
    }

'''
if text.count(validation) != 1:
    raise SystemExit(f"expected one misplaced conjugation validation block, found {text.count(validation)}")
text = text.replace(validation, "", 1)
flow_anchor = "    } else if has_property(SpreadsheetSemanticProperty::FlowDirection) {"
start = text.find(flow_anchor)
if start < 0:
    raise SystemExit("flow-direction validation anchor not found")
target_anchor = "\n    let target = project.element(map.target_scope).map_err(|_| {"
pos = text.find(target_anchor, start)
if pos < 0:
    raise SystemExit("non-relationship target validation anchor not found")
text = text[:pos] + "\n" + validation + text[pos:]
path.write_text(text)
