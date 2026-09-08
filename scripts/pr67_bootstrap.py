from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(path: str, old: str, new: str) -> None:
    target = ROOT / path
    text = target.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"expected PR67 patch anchor not found in {path}: {old!r}")
    target.write_text(text.replace(old, new, 1), encoding="utf-8")


app = "apps/desktop/frontend/app.js"
replace_once(
    app,
    "const $ = (id) => document.getElementById(id);\n",
    "const $ = (id) => document.getElementById(id);\n"
    "function setOptionalText(id, value) {\n"
    "  const node = $(id);\n"
    "  if (node) node.textContent = value;\n"
    "}\n",
)
replace_once(
    app,
    "  $('active-diagram-summary').textContent = diagram ? `${diagram.name} · ${label}` : 'No diagram selected';\n"
    "  $('palette-title').textContent = diagram ? `Elements (${label})` : 'Elements';",
    "  setOptionalText('active-diagram-summary', diagram ? `${diagram.name} · ${label}` : 'No diagram selected');\n"
    "  setOptionalText('palette-title', diagram ? `Elements (${label})` : 'Elements');",
)

for path, old, new in [
    (
        "apps/desktop/frontend/ibd-ui.js",
        "  $('active-diagram-summary').textContent = `${ibd.name} · IBD`;\n  $('palette-title').textContent = 'Elements (IBD)';",
        "  setOptionalText('active-diagram-summary', `${ibd.name} · IBD`);\n  setOptionalText('palette-title', 'Elements (IBD)');",
    ),
    (
        "apps/desktop/frontend/use-case-ui.js",
        "    $('active-diagram-summary').textContent = `${diagram.name} · Use Case Diagram`;\n    $('palette-title').textContent = 'Elements (Use Case)';",
        "    setOptionalText('active-diagram-summary', `${diagram.name} · Use Case Diagram`);\n    setOptionalText('palette-title', 'Elements (Use Case)');",
    ),
    (
        "apps/desktop/frontend/workspace-ux.js",
        "    $('active-diagram-summary').textContent = `${diagram.name} · Package Diagram`;\n    $('palette-title').textContent = 'Package Diagram';",
        "    setOptionalText('active-diagram-summary', `${diagram.name} · Package Diagram`);\n    setOptionalText('palette-title', 'Package Diagram');",
    ),
    (
        "apps/desktop/frontend/parametric-ui.js",
        "    $('active-diagram-summary').textContent = `${diagram.name} · Parametric Diagram`;\n    $('palette-title').textContent = 'Elements (Parametric)';",
        "    setOptionalText('active-diagram-summary', `${diagram.name} · Parametric Diagram`);\n    setOptionalText('palette-title', 'Elements (Parametric)');",
    ),
]:
    replace_once(path, old, new)

validator = ROOT / "scripts/validate_model_script_import_qualification.py"
text = validator.read_text(encoding="utf-8")
anchor = 'model_script_ui = (frontend / "model-script-ui.js").read_text(encoding="utf-8")\n'
if anchor not in text:
    raise SystemExit("model-script qualification frontend anchor not found")
addition = anchor + (
    'app_frontend = (frontend / "app.js").read_text(encoding="utf-8")\n'
    'ibd_frontend = (frontend / "ibd-ui.js").read_text(encoding="utf-8")\n'
    'use_case_frontend = (frontend / "use-case-ui.js").read_text(encoding="utf-8")\n'
    'package_frontend = (frontend / "workspace-ux.js").read_text(encoding="utf-8")\n'
    'parametric_frontend = (frontend / "parametric-ui.js").read_text(encoding="utf-8")\n'
    'shell_frontend = (frontend / "ui-shell.js").read_text(encoding="utf-8")\n'
)
text = text.replace(anchor, addition, 1)

append = '''\n\n# Model Script is launched from the File ribbon. ui-shell replaces the ribbon\n# contents for that tab, so active-diagram-summary is intentionally absent while\n# model-script refresh/qualification renders the workspace. All family context\n# renderers must therefore treat ribbon context targets as optional.\nassert "File:" in shell_frontend and "ribbon.innerHTML = panels[name]" in shell_frontend, "dynamic ribbon contract changed"\nassert "function setOptionalText(id, value)" in app_frontend, "shared optional DOM text helper is missing"\nfor name, source in {\n    "app": app_frontend,\n    "ibd": ibd_frontend,\n    "use-case": use_case_frontend,\n    "package": package_frontend,\n    "parametric": parametric_frontend,\n}.items():\n    assert "$('active-diagram-summary').textContent" not in source, f"{name} context renderer can crash when File/Arrange/View/Help removes active-diagram-summary"\n    assert "setOptionalText('active-diagram-summary'" in source, f"{name} context renderer does not use optional ribbon context writes"\n'''
if "dynamic ribbon contract changed" in text:
    raise SystemExit("PR67 validator contract is already present")
text += append
validator.write_text(text, encoding="utf-8")

print("PR67 File-tab model-script render null-safety patch applied")
