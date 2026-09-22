#[path = "support/unit_scale.rs"]
mod unit_scale;

#[test]
fn validates_three_thousand_distinct_unit_reference_sets() {
    let project = unit_scale::unit_project(3_000);
    assert_eq!(project.elements.len(), 9_001);
    assert_eq!(project.validate(), Ok(()));
}
