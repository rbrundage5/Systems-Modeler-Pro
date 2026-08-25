use crate::diagram_family::{DiagramCapability, DiagramFamilyRegistry};

/// Returns the product-level diagram family registry.
///
/// Package Diagram is the ninth built-in SysML 1.x family. Product-level
/// capability closure is applied here so downstream workspace consumers see
/// the same shared frame/navigation contract as the other qualified families
/// without duplicating renderer or interaction infrastructure.
pub fn supported_diagram_families() -> DiagramFamilyRegistry {
    let base = crate::diagram_family::supported_diagram_families();
    let mut registry = DiagramFamilyRegistry::default();

    for mut descriptor in base.descriptors() {
        if descriptor.id.0 == "package" {
            descriptor.capabilities.insert(DiagramCapability::Frames);
            descriptor.capabilities.insert(DiagramCapability::DrillDown);
        }
        registry
            .register(descriptor)
            .expect("product diagram family registry must remain valid");
    }

    registry
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
        assert!(package.supports(C::Frames));
        assert!(package.supports(C::DrillDown));
    }
}
