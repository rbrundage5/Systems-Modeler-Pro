from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def patch(path_str: str, replacements):
    path = ROOT / path_str
    text = path.read_text(encoding="utf-8")
    for label, old, new in replacements:
        if old not in text:
            raise SystemExit(f"{path_str}: missing anchor: {label}")
        text = text.replace(old, new, 1)
    path.write_text(text, encoding="utf-8")


patch(
    "crates/model-core/src/model.rs",
    [
        (
            "Allocate enum variant",
            "    Generalization,\n    Realization,\n    Connector,\n",
            "    Generalization,\n    Realization,\n    /// Explicit SysML allocation from a source semantic element to a target semantic element.\n    Allocate,\n    Connector,\n",
        ),
        (
            "allocatable semantic classification",
            "    pub fn is_feature(&self) -> bool {\n        self.is_property()\n            || self.is_port()\n            || matches!(\n                self.kind,\n                ElementKind::Operation\n                    | ElementKind::Parameter\n                    | ElementKind::ConstraintParameter\n                    | ElementKind::Reception\n                    | ElementKind::Slot\n            )\n    }\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct AssociationEnd",
            "    pub fn is_feature(&self) -> bool {\n        self.is_property()\n            || self.is_port()\n            || matches!(\n                self.kind,\n                ElementKind::Operation\n                    | ElementKind::Parameter\n                    | ElementKind::ConstraintParameter\n                    | ElementKind::Reception\n                    | ElementKind::Slot\n            )\n    }\n\n    /// Common semantic endpoint boundary for explicit SysML Allocation.\n    /// Presentation/admin namespaces and Comments are not allocation endpoints;\n    /// all other stable model-core Elements may participate.\n    pub fn is_allocatable(&self) -> bool {\n        !self.is_namespace() && self.kind != ElementKind::Comment\n    }\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct AssociationEnd",
        ),
        (
            "allocation errors",
            "    #[error(\"Requirement traceability relationships must be owned by a Model or Package: {0}\")]\n    InvalidTraceabilityOwner(ElementId),\n    #[error(\"{diagnostic}\")]\n    InvalidPackageRelationshipEndpoints",
            "    #[error(\"Requirement traceability relationships must be owned by a Model or Package: {0}\")]\n    InvalidTraceabilityOwner(ElementId),\n    #[error(\"Allocation cannot connect an element to itself\")]\n    AllocationSelfReference,\n    #[error(\"invalid Allocation endpoints: {source_kind:?} -> {target_kind:?}\")]\n    InvalidAllocationEndpoints {\n        source_kind: ElementKind,\n        target_kind: ElementKind,\n    },\n    #[error(\"duplicate Allocation relationship: {source_id} -> {target_id}\")]\n    DuplicateAllocationRelationship {\n        source_id: ElementId,\n        target_id: ElementId,\n    },\n    #[error(\"Allocation relationships require a Model or Package owner\")]\n    MissingAllocationOwner,\n    #[error(\"Allocation relationships must be owned by a Model or Package: {0}\")]\n    InvalidAllocationOwner(ElementId),\n    #[error(\"{diagnostic}\")]\n    InvalidPackageRelationshipEndpoints",
        ),
        (
            "allocation creation validation",
            "        let source = self.element(source_id)?;\n        let target = self.element(target_id)?;\n        let package_relationship =\n",
            "        let source = self.element(source_id)?;\n        let target = self.element(target_id)?;\n        if kind == RelationshipKind::Allocate {\n            validate_allocation_endpoints(source, target)?;\n            if source_id == target_id {\n                return Err(ModelError::AllocationSelfReference);\n            }\n            if self.relationships.values().any(|relationship| {\n                relationship.kind == RelationshipKind::Allocate\n                    && relationship.source_id == source_id\n                    && relationship.target_id == target_id\n            }) {\n                return Err(ModelError::DuplicateAllocationRelationship {\n                    source_id,\n                    target_id,\n                });\n            }\n        }\n        let package_relationship =\n",
        ),
        (
            "allocation owner required",
            "        if traceability && owner_id.is_none() {\n            return Err(ModelError::MissingTraceabilityOwner);\n        }\n        if let Some(owner_id) = owner_id {\n",
            "        if traceability && owner_id.is_none() {\n            return Err(ModelError::MissingTraceabilityOwner);\n        }\n        if kind == RelationshipKind::Allocate && owner_id.is_none() {\n            return Err(ModelError::MissingAllocationOwner);\n        }\n        if let Some(owner_id) = owner_id {\n",
        ),
        (
            "allocation owner kind",
            "            if traceability && !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {\n                return Err(ModelError::InvalidTraceabilityOwner(owner_id));\n            }\n            if !owner.is_namespace() && !owner.is_classifier() {\n",
            "            if traceability && !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {\n                return Err(ModelError::InvalidTraceabilityOwner(owner_id));\n            }\n            if kind == RelationshipKind::Allocate\n                && !matches!(owner.kind, ElementKind::Model | ElementKind::Package)\n            {\n                return Err(ModelError::InvalidAllocationOwner(owner_id));\n            }\n            if !owner.is_namespace() && !owner.is_classifier() {\n",
        ),
        (
            "allocation project validation",
            "            self.element(relationship.source_id)?;\n            self.element(relationship.target_id)?;\n            if is_traceability_relationship(&relationship.kind) {\n",
            "            self.element(relationship.source_id)?;\n            self.element(relationship.target_id)?;\n            if relationship.kind == RelationshipKind::Allocate {\n                let source = self.element(relationship.source_id)?;\n                let target = self.element(relationship.target_id)?;\n                validate_allocation_endpoints(source, target)?;\n                if relationship.source_id == relationship.target_id {\n                    return Err(ModelError::AllocationSelfReference);\n                }\n                let owner_id = relationship.owner_id.ok_or(ModelError::MissingAllocationOwner)?;\n                if !matches!(\n                    self.element(owner_id)?.kind,\n                    ElementKind::Model | ElementKind::Package\n                ) {\n                    return Err(ModelError::InvalidAllocationOwner(owner_id));\n                }\n                if self.relationships.values().any(|candidate| {\n                    candidate.id != relationship.id\n                        && candidate.kind == RelationshipKind::Allocate\n                        && candidate.source_id == relationship.source_id\n                        && candidate.target_id == relationship.target_id\n                }) {\n                    return Err(ModelError::DuplicateAllocationRelationship {\n                        source_id: relationship.source_id,\n                        target_id: relationship.target_id,\n                    });\n                }\n            }\n            if is_traceability_relationship(&relationship.kind) {\n",
        ),
        (
            "allocation endpoint validator",
            "fn validate_traceability_endpoints(\n    relationship: &RelationshipKind,\n    source: &ElementKind,\n    target: &ElementKind,\n) -> Result<(), ModelError> {\n",
            "fn validate_allocation_endpoints(source: &Element, target: &Element) -> Result<(), ModelError> {\n    if source.is_allocatable() && target.is_allocatable() {\n        Ok(())\n    } else {\n        Err(ModelError::InvalidAllocationEndpoints {\n            source_kind: source.kind.clone(),\n            target_kind: target.kind.clone(),\n        })\n    }\n}\n\nfn validate_traceability_endpoints(\n    relationship: &RelationshipKind,\n    source: &ElementKind,\n    target: &ElementKind,\n) -> Result<(), ModelError> {\n",
        ),
        (
            "allocation generic notation coverage",
            "            RelationshipKind::DeriveRequirement\n            | RelationshipKind::Satisfy\n",
            "            RelationshipKind::Allocate\n            | RelationshipKind::DeriveRequirement\n            | RelationshipKind::Satisfy\n",
        ),
    ],
)

patch(
    "apps/desktop/src-tauri/src/workspace.rs",
    [
        (
            "allocation snapshot display kind",
            "        RelationshipKind::Realization => \"Realization\",\n        RelationshipKind::Connector => \"Connector\",\n",
            "        RelationshipKind::Realization => \"Realization\",\n        RelationshipKind::Allocate => \"Allocate\",\n        RelationshipKind::Connector => \"Connector\",\n",
        ),
    ],
)

patch(
    "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs",
    [
        (
            "Allocate supported relationship map",
            "            | RelationshipKind::Realization\n            | RelationshipKind::DeriveRequirement\n",
            "            | RelationshipKind::Realization\n            | RelationshipKind::Allocate\n            | RelationshipKind::DeriveRequirement\n",
        ),
        (
            "relationship scope diagnostic",
            "format!(\"{:?} is outside the PR40/PR41 relationship scope\", kind),",
            "format!(\"{:?} is outside the PR40/PR41/PR42 relationship scope\", kind),",
        ),
        (
            "Allocate relationship keyword",
            "        \"realization\" => RelationshipKind::Realization,\n        \"deriverequirement\" | \"derivereqt\" => RelationshipKind::DeriveRequirement,\n",
            "        \"realization\" => RelationshipKind::Realization,\n        \"allocate\" => RelationshipKind::Allocate,\n        \"deriverequirement\" | \"derivereqt\" => RelationshipKind::DeriveRequirement,\n",
        ),
        (
            "relationship keyword diagnostic",
            "relationship kind '{}' is outside PR40/PR41; expected Association, Generalization, Dependency, Realization, DeriveRequirement/deriveReqt, Satisfy, Verify, Refine, Trace, or Copy",
            "relationship kind '{}' is outside PR40/PR41/PR42; expected Association, Generalization, Dependency, Realization, Allocate, DeriveRequirement/deriveReqt, Satisfy, Verify, Refine, Trace, or Copy",
        ),
        (
            "allocation self reference diagnostic",
            "    let owner = if let Some(owner_text) =\n        non_empty_value(values, SpreadsheetSemanticProperty::Owner)\n    {\n",
            "    if kind == RelationshipKind::Allocate && source.reference == target.reference {\n        return Err(diagnostic(\n            Some(map),\n            Some(row),\n            None,\n            None,\n            non_empty_value(values, SpreadsheetSemanticProperty::ExternalId)\n                .map(ToOwned::to_owned),\n            \"ALLOCATION_SELF_REFERENCE\",\n            format!(\n                \"Allocate source '{}' and target '{}' resolve to the same semantic element\",\n                non_empty_value(values, SpreadsheetSemanticProperty::Source).unwrap_or_default(),\n                non_empty_value(values, SpreadsheetSemanticProperty::Target).unwrap_or_default()\n            ),\n        ));\n    }\n    let owner = if let Some(owner_text) =\n        non_empty_value(values, SpreadsheetSemanticProperty::Owner)\n    {\n",
        ),
        (
            "allocation target-scope owner reuse",
            "    } else if is_pr41_traceability_kind(&kind) {\n        let inferred = resolve_owner(map, project, planned, None)?;\n",
            "    } else if is_pr41_traceability_kind(&kind) || kind == RelationshipKind::Allocate {\n        let inferred = resolve_owner(map, project, planned, None)?;\n",
        ),
        (
            "allocation no loose root owner diagnostic",
            "                \"PR41 does not infer a loose root owner; map Owner explicitly or configure a package target scope that contains the relationship endpoints\",\n",
            "                \"PR41/PR42 does not infer a loose root owner; map Owner explicitly or configure a package target scope that contains the relationship endpoints\",\n",
        ),
    ],
)
