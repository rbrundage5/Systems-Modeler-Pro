from pathlib import Path

path = Path("apps/desktop/src-tauri/src/workspace/reqif_runtime.rs")
text = path.read_text(encoding="utf-8")
old = """        if !visited.insert(id) {\n            break;\n        }\n"""
new = """        if !visited.insert(id.to_string()) {\n            break;\n        }\n"""
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("native scope cycle-detection source not found")
old = """    let mut binding_lookup: BTreeMap<String, (&str, &ReqifSourceState)> = BTreeMap::new();\n    for (namespace, source) in &exchange.sources {\n"""
new = """    let mut binding_lookup: BTreeMap<String, (&str, &ReqifSourceState)> = BTreeMap::new();\n    for source in exchange.sources.values() {\n"""
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("unused export namespace source not found")
path.write_text(text, encoding="utf-8")
