use systems_modeler_core::{ElementKind, Project};

#[test]
fn children_are_returned_in_stable_external_id_order() {
    let mut project = Project::new("deterministic children");
    let root = project.root_id;
    let z = project
        .create_element(ElementKind::Package, "Zulu", root)
        .unwrap();
    let a = project
        .create_element(ElementKind::Package, "Alpha", root)
        .unwrap();
    let m = project
        .create_element(ElementKind::Package, "Mike", root)
        .unwrap();

    project.elements.get_mut(&z).unwrap().external_id = "Z".into();
    project.elements.get_mut(&a).unwrap().external_id = "A".into();
    project.elements.get_mut(&m).unwrap().external_id = "M".into();

    let ordered = project
        .children(root)
        .map(|element| element.external_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ordered, vec!["A", "M", "Z"]);
}
