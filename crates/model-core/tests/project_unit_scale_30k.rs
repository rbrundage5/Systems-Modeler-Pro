#[path = "support/unit_scale.rs"]
mod unit_scale;

#[test]
fn validates_thirty_thousand_distinct_unit_reference_sets() {
    let project = unit_scale::unit_project(30_000);
    assert_eq!(project.elements.len(), 90_001);
    assert_eq!(project.validate(), Ok(()));
}
