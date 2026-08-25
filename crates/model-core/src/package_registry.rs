use crate::diagram_family::DiagramFamilyRegistry;

/// Returns the product-level diagram family registry.
///
/// Package Diagram is a built-in ninth family in the shared registry.
pub fn supported_diagram_families() -> DiagramFamilyRegistry {
    crate::diagram_family::supported_diagram_families()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram_family::{DiagramCapability as C, DiagramFamilyId};

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
        assert!(package.supports(C::Relationships));
        assert!(package.supports(C::Routing));
        assert!(package.supports(C::CleanLayout));
    }
}
