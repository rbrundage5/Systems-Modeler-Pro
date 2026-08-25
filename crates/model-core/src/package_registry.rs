use crate::diagram_family::{
    DiagramCapability, DiagramFamilyDescriptor, DiagramFamilyRegistry,
};

const SHARED_WORKSPACE_CAPABILITIES: [DiagramCapability; 9] = [
    DiagramCapability::NodePlacement,
    DiagramCapability::Relationships,
    DiagramCapability::Frames,
    DiagramCapability::Move,
    DiagramCapability::Resize,
    DiagramCapability::Delete,
    DiagramCapability::Clipboard,
    DiagramCapability::Routing,
    DiagramCapability::CleanLayout,
];

/// Returns the product-level registry for the nine qualified SysML 1.x diagram
/// families.
///
/// The base registry owns family-specific semantics such as frame notation,
/// permitted owners, preferred flow, evaluation, and most navigation support.
/// This product boundary closes only the shared workspace contract that every
/// currently qualified built-in family is known to implement. Keeping that
/// closure in one place prevents one-off Package/BDD capability patches from
/// drifting apart while execution-specific capabilities remain family-owned.
pub fn supported_diagram_families() -> DiagramFamilyRegistry {
    let base = crate::diagram_family::supported_diagram_families();
    let mut registry = DiagramFamilyRegistry::default();

    for mut descriptor in base.descriptors() {
        close_shared_workspace_contract(&mut descriptor);
        registry
            .register(descriptor)
            .expect("product diagram family registry must remain valid");
    }

    registry
}

fn close_shared_workspace_contract(descriptor: &mut DiagramFamilyDescriptor) {
    for capability in SHARED_WORKSPACE_CAPABILITIES {
        descriptor.capabilities.insert(capability);
    }

    // BDD already has qualified semantic Block/AssociationBlock -> IBD
    // navigation in the desktop workspace. Advertise that existing behavior
    // through the same shared DrillDown capability used by the other families.
    if descriptor.id.0 == "bdd" {
        descriptor.capabilities.insert(DiagramCapability::DrillDown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram_family::{DiagramCapability as C, DiagramFamilyId};

    #[test]
    fn product_registry_exposes_all_nine_sysml_1x_diagram_families() {
        let registry = supported_diagram_families();
        assert_eq!(registry.descriptors().len(), 9);

        let expected = [
            ("bdd", "bdd", "Package"),
            ("ibd", "ibd", "Block"),
            ("state-machine", "stm", "StateMachine"),
            ("sequence", "seq", "Interaction"),
            ("activity", "act", "Activity"),
            ("requirement", "req", "Package"),
            ("use-case", "uc", "Package"),
            ("package", "pkg", "Package"),
            ("parametric", "par", "Block"),
        ];

        for (id, abbreviation, context_kind) in expected {
            let family = registry
                .get(&DiagramFamilyId::new(id).unwrap())
                .expect("qualified diagram family is registered");
            assert_eq!(family.frame_abbreviation, abbreviation);
            assert_eq!(family.frame_model_element_type, context_kind);
        }
    }

    #[test]
    fn every_qualified_family_exposes_the_shared_workspace_contract() {
        let registry = supported_diagram_families();

        for family in registry.descriptors() {
            for capability in SHARED_WORKSPACE_CAPABILITIES {
                assert!(
                    family.supports(capability),
                    "{} is missing shared workspace capability {capability:?}",
                    family.id.0
                );
            }
        }
    }

    #[test]
    fn bdd_and_package_expose_qualified_frame_and_navigation_support() {
        let registry = supported_diagram_families();

        for id in ["bdd", "package"] {
            let family = registry
                .get(&DiagramFamilyId::new(id).unwrap())
                .expect("qualified family is registered");
            assert!(family.supports(C::Frames));
            assert!(family.supports(C::DrillDown));
        }
    }

    #[test]
    fn evaluation_remains_parametric_specific() {
        let registry = supported_diagram_families();

        for family in registry.descriptors() {
            assert_eq!(
                family.supports(C::Evaluation),
                family.id.0 == "parametric",
                "Evaluation capability drifted for {}",
                family.id.0
            );
        }
    }
}
