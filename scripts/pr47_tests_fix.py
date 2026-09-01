from pathlib import Path

p = Path('apps/desktop/src-tauri/src/workspace/spreadsheet_import/pr47_tests.rs')
text = p.read_text()

text = text.replace(
    'let bad=temp_csv("bad","ID,Kind,From,To,Owner,Alias,Point,Condition\\nI1,Include,A,Block,P,,,\\nI2,Include,A,A,P,,,\\nE1,Extend,B,A,P,,Missing,x\\nEI,ElementImport,P,Signal,P,not-valid!,,\\n");',
    'let bad=temp_csv("bad","ID,Kind,From,To,Owner,Alias,Point,Condition\\nI1,Include,P::A,P::Block,P,,,\\nI2,Include,P::A,P::A,P,,,\\nE1,Extend,P::B,P::A,P,,Missing,x\\nEI,ElementImport,P,P::Signal,P,not-valid!,,\\n");'
)
text = text.replace(
    '    assert!(preview.diagnostics.iter().any(|d| d.reason.contains("extension point") || d.reason.contains("Missing")));\n',
    ''
)
text = text.replace(
    'let update=temp_csv("update","ID,Kind,From,To,Owner,Alias,Visibility,Point,Condition,Description\\nEXT-1,Extend,UC-EMERG,UC-OPERATE,PKG-UC,,,AlternateHandling,updated,changed\\nEI-1,ElementImport,PKG-VEH,SIG-CMD,PKG-VEH,Cmd,Private,,,changed\\n");',
    'let update=temp_csv("update","ID,Kind,From,To,Owner,Alias,Visibility,Point,Condition,Description\\nEXT-1,Extend,UC-EMERG,UC-OPERATE,PKG-UC,,Public,AlternateHandling,updated,changed\\nEI-1,ElementImport,PKG-VEH,SIG-CMD,PKG-VEH,Cmd,Private,,,changed\\n");'
)
old_setup = '{let mut g=state.project.lock().unwrap();let p=g.as_mut().unwrap();let p1=p.create_element(ElementKind::Package,"One",root).unwrap();let p2=p.create_element(ElementKind::Package,"Two",root).unwrap();p.create_element(ElementKind::UseCase,"Same",p1).unwrap();p.create_element(ElementKind::UseCase,"Same",p2).unwrap();p.create_element(ElementKind::UseCase,"Target",p1).unwrap();}'
new_setup = '{let mut g=state.project.lock().unwrap();let p=g.as_mut().unwrap();let p1=p.create_element(ElementKind::Package,"One",root).unwrap();p.create_element(ElementKind::Package,"Two",root).unwrap();p.create_element(ElementKind::UseCase,"Same",p1).unwrap();p.create_element(ElementKind::UseCase,"Same",p1).unwrap();p.create_element(ElementKind::UseCase,"Target",p1).unwrap();}'
if old_setup not in text:
    raise SystemExit('resolution setup anchor missing')
text = text.replace(old_setup, new_setup, 1)
old_csv = 'let csv=temp_csv("resolution","ID,Kind,From,To,Owner\\nA,Include,Same,Target,One\\nB,Include,Missing,Target,One\\nC,PackageImport,One,Two,Two\\nC,PackageImport,One,Two,One\\n");'
new_csv = 'let csv=temp_csv("resolution","ID,Kind,From,To,Owner\\nA,Include,One::Same,One::Target,One\\nB,Include,One::Missing,One::Target,One\\nC,PackageImport,One,Two,Two\\nD,PackageImport,One,Two,One\\nD,PackageImport,One,Two,One\\n");'
if old_csv not in text:
    raise SystemExit('resolution CSV anchor missing')
text = text.replace(old_csv, new_csv, 1)
old_late = 'let preview=preview_spreadsheet_import_group(&group,&state);assert!(!preview.is_valid());assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(),1);assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(),0);assert!(apply_spreadsheet_import_group(&group,&state).is_err());assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(),1);'
new_late = 'let preview=preview_spreadsheet_import_group(&group,&state);assert!(!preview.is_valid());assert!(preview.diagnostics.iter().any(|d| d.reason.contains("extension point") || d.reason.contains("Missing")), "{:?}", preview.diagnostics);assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(),1);assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(),0);assert!(apply_spreadsheet_import_group(&group,&state).is_err());assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(),1);'
if old_late not in text:
    raise SystemExit('late invalid assertion anchor missing')
text = text.replace(old_late, new_late, 1)
p.write_text(text)

# PR40 used Include as the representative unsupported relationship kind. PR47 now
# intentionally supports Include, so keep the old regression's original purpose by
# using BindingConnector, which remains outside the spreadsheet-import scope.
spreadsheet = Path('apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs')
source = spreadsheet.read_text()
old = 'temp_csv("ID,Kind,Source,Target,Owner\\nREL-2,Include,VEH,ENG,Structure\\n"),'
new = 'temp_csv("ID,Kind,Source,Target,Owner\\nREL-2,BindingConnector,VEH,ENG,Structure\\n"),'
if old not in source:
    raise SystemExit('PR40 unsupported-kind compatibility anchor missing')
spreadsheet.write_text(source.replace(old, new, 1))
