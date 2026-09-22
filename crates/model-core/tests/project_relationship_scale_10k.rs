#[path = "support/relationship_scale.rs"]
mod relationship_scale;

#[test]
fn validates_ten_thousand_mixed_relationships() {
    let project = relationship_scale::mixed_relationship_project(100, 100);
    assert_eq!(project.elements.len(), 201);
    assert_eq!(project.relationships.len(), 10_000);
    assert_eq!(project.validate(), Ok(()));
}
