use systems_modeler_core::{
    ElementKind, Project, SemanticTarget, StereotypeTargetKind, TagValue, TagValueType,
};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn pr53_profiles_applications_and_typed_tags_round_trip_transactionally() {
    let mut project = Project::new("Profile Persistence");
    let block = project
        .create_element(ElementKind::Block, "Controller", project.root_id)
        .unwrap();
    let profile = project
        .create_profile(
            "profile:safety",
            "Safety",
            Some("https://example.test/safety".into()),
        )
        .unwrap();
    let stereotype = project
        .create_stereotype(
            profile,
            "stereotype:safety-critical",
            "SafetyCritical",
            vec![StereotypeTargetKind::Element(ElementKind::Block)],
        )
        .unwrap();
    let tag = project
        .create_tag_definition(
            stereotype,
            "tag:sil",
            "sil",
            TagValueType::Integer,
            (0, Some(1)),
            None,
        )
        .unwrap();
    project
        .apply_profile(profile, project.root_id, "profile-application:safety")
        .unwrap();
    let application = project
        .apply_stereotype(
            stereotype,
            SemanticTarget::Element(block),
            "stereotype-application:controller-safety",
        )
        .unwrap();
    project
        .set_tagged_values(application, tag, vec![TagValue::Integer(3)])
        .unwrap();

    let mut database = ProjectDatabase::open_in_memory().unwrap();
    database.save_project(&project).unwrap();
    let restored = database.load_project(project.id).unwrap();

    assert_eq!(restored.profiles, project.profiles);
    assert_eq!(
        restored.element(block).unwrap().applied_stereotypes,
        ["SafetyCritical"]
    );
    restored.validate().unwrap();
}
