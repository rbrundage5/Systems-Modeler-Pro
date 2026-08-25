use crate::diagram_family::{
    DiagramCapability as C, DiagramFamilyDescriptor, DiagramFamilyId, DiagramFamilyRegistry,
    PreferredFlowDirection,
};

/// Returns the product-level diagram family registry.
///
/// Package Diagram is layered onto the established family contract instead of
/// duplicating workspace behavior. The base registry remains independently
/// extensible and its tests continue to exercise third-party registration.
pub fn supported_diagram_families() -> DiagramFamilyRegistry {
    let mut registry = crate::diagram_family::supported_diagram_families();
    registry
        .register(DiagramFamilyDescriptor {
            id: DiagramFamilyId::new("package").expect("static package family id is valid"),
            display_name: "Package Diagram".into(),
            frame_abbreviation: "pkg".into(),
            frame_model_element_type: "Package".into(),
            renderer_id: "package".into(),
            permitted_owner_kinds: vec!["Model".into(), "Package".into()],
            capabilities: [
                C::NodePlacement,
                C::Frames,
                C::Move,
                C::Resize,
                C::Delete,
                C::Clipboard,
                C::DrillDown,
            ]
            .into_iter()
            .collect(),
            preferred_flow: PreferredFlowDirection::TopToBottom,
            accessibility_name: "Package Diagram engineering workspace".into(),
            empty_message: "This Package Diagram has no presented packages.".into(),
        })
        .expect("built-in Package Diagram family must be valid");
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_registry_exposes_all_nine_sysml_1x_diagram_families() {
        let registry = supported_diagram_families();
        assert_eq!(registry.descriptors().len(), 9);
        let package = registry
            .get(&DiagramFamilyId::new("package").unwrap())
            .expect("Package Diagram is registered");
        assert_eq!(package.frame_abbreviation, "pkg");
        assert_eq!(package.frame_model_element_type, "Package");
        assert_eq!(package.renderer_id, "package");
        assert!(package.supports(C::NodePlacement));
        assert!(package.supports(C::Move));
        assert!(package.supports(C::Resize));
        assert!(package.supports(C::Clipboard));
        assert!(!package.supports(C::Relationships));
        assert!(!package.supports(C::Routing));
        assert!(!package.supports(C::CleanLayout));
    }
}
