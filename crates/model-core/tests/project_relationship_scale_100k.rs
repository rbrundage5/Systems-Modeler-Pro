#[path = "support/relationship_scale.rs"]
mod relationship_scale;

#[test]
fn validates_one_hundred_thousand_mixed_relationships() {
    let project = relationship_scale::mixed_relationship_project(100, 1_000);
    assert_eq!(project.elements.len(), 1_101);
    assert_eq!(project.relationships.len(), 100_000);
    assert_eq!(project.validate(), Ok(()));
}
