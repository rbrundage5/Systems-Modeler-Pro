use systems_modeler_core::{ElementKind, Project, RelationshipKind, VisibilityKind};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn pr47_persistence_round_trip_preserves_all_five_relationships_and_metadata() {
    let mut p = Project::new("PR47 Persistence");
    let root = p.root_id;
    let uc_pkg = p.create_element(ElementKind::Package, "UC", root).unwrap();
    let base = p
        .create_element(ElementKind::UseCase, "Base", uc_pkg)
        .unwrap();
    p.element_mut(base).unwrap().extension_points = vec!["point".into()];
    let included = p
        .create_element(ElementKind::UseCase, "Included", uc_pkg)
        .unwrap();
    let ext = p
        .create_element(ElementKind::UseCase, "Ext", uc_pkg)
        .unwrap();
    let include = p
        .create_relationship(RelationshipKind::Include, base, included, Some(uc_pkg))
        .unwrap();
    let extend = p
        .create_relationship(RelationshipKind::Extend, ext, base, Some(uc_pkg))
        .unwrap();
    p.update_extend_relationship(extend, Some("guard".into()), Some("point".into()))
        .unwrap();
    let a = p.create_element(ElementKind::Package, "A", root).unwrap();
    let b = p.create_element(ElementKind::Package, "B", root).unwrap();
    let sig = p.create_element(ElementKind::Signal, "Signal", b).unwrap();
    let pi = p
        .create_package_import(a, b, VisibilityKind::Private)
        .unwrap();
    let ei = p
        .create_element_import(a, sig, VisibilityKind::Public, Some("Alias".into()))
        .unwrap();
    let pm = p
        .create_relationship(RelationshipKind::PackageMerge, a, b, Some(a))
        .unwrap();
    for (id, key) in [
        (include, "INC"),
        (extend, "EXT"),
        (pi, "PI"),
        (ei, "EI"),
        (pm, "PM"),
    ] {
        let r = p.relationships.get_mut(&id).unwrap();
        r.external_id = format!("catia:pr47::{key}");
        r.name = format!("name-{key}");
        r.documentation = format!("doc-{key}");
    }
    p.validate().unwrap();
    let mut db = ProjectDatabase::open_in_memory().unwrap();
    db.save_project(&p).unwrap();
    let r = db.load_project(p.id).unwrap();
    for (id, kind) in [
        (include, RelationshipKind::Include),
        (extend, RelationshipKind::Extend),
        (pi, RelationshipKind::PackageImport),
        (ei, RelationshipKind::ElementImport),
        (pm, RelationshipKind::PackageMerge),
    ] {
        assert_eq!(r.relationship(id).unwrap().kind, kind);
    }
    assert_eq!(r.relationship(ei).unwrap().alias.as_deref(), Some("Alias"));
    assert_eq!(
        r.relationship(extend)
            .unwrap()
            .extension_condition
            .as_deref(),
        Some("guard")
    );
    assert_eq!(
        r.relationship(extend)
            .unwrap()
            .extension_location
            .as_deref(),
        Some("point")
    );
    assert_eq!(
        r.relationship(pi).unwrap().visibility,
        VisibilityKind::Private
    );
    r.validate().unwrap();
}
