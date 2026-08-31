from pathlib import Path

path = Path(__file__).resolve().parents[1] / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
text = path.read_text(encoding="utf-8")
old = 'temp_csv("ID,Kind,Source,Target,Owner\\nREL-2,Satisfy,VEH,ENG,Structure\\n"),'
new = 'temp_csv("ID,Kind,Source,Target,Owner\\nREL-2,Include,VEH,ENG,Structure\\n"),'
if old not in text:
    raise SystemExit("missing PR40 unsupported-kind test anchor")
path.write_text(text.replace(old, new, 1), encoding="utf-8")
