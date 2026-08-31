#![allow(clippy::result_large_err)]

use super::{
    WorkspaceState,
    bulk_model::{
        AssociationEndBuildFields, BuildDiagnostic, BuildDiagnosticSeverity, BuildReference,
        ElementReference, ModelBuildOperation, ModelBuildPlan, ModelBuildResult,
        RelationshipReference, apply_model_build, external_key, preview_model_build,
    },
};
use calamine::{Reader, open_workbook_auto};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use systems_modeler_core::{
    AggregationKind, Element, ElementId, ElementKind, FlowDirection, Multiplicity, Project,
    Relationship, RelationshipKind, VisibilityKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SpreadsheetSemanticProperty {
    Name,
    Documentation,
    Owner,
    Type,
    Multiplicity,
    DefaultValue,
    FlowDirection,
    ExternalId,
    Visibility,
    RequirementId,
    RequirementText,
    RelationshipKind,
    Source,
    Target,
    SourceEndRole,
    TargetEndRole,
    SourceMultiplicity,
    TargetMultiplicity,
    SourceNavigable,
    TargetNavigable,
    SourceAggregation,
    TargetAggregation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpreadsheetIdentificationProperty {
    ExternalId,
    Name,
    RequirementId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpreadsheetSearchScope {
    TargetOnly,
    TargetRecursive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SpreadsheetRelationshipIdentityPolicy {
    #[default]
    ExternalId,
    KindSourceTarget,
    KindSourceTargetAssociationEnds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetColumnMapping {
    pub source_column: String,
    pub property: SpreadsheetSemanticProperty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadsheetImportMap {
    pub name: String,
    /// File path used for this import run. Source identity is deliberately separate
    /// from `source_namespace`; renaming a file never changes semantic identity.
    pub source: String,
    pub worksheet: Option<String>,
    /// One-based physical row number containing headers.
    pub header_row: usize,
    pub element_kind: ElementKind,
    /// PR40 relationship mappings set this to a supported relationship kind, or map
    /// the controlled `RelationshipKind` property for mixed relationship rows. The
    /// legacy `element_kind` field remains for backwards-compatible PR38/39 maps.
    #[serde(default)]
    pub relationship_kind: Option<RelationshipKind>,
    #[serde(default)]
    pub relationship_identity: SpreadsheetRelationshipIdentityPolicy,
    /// Stable semantic target. Display names are never persisted as target identity.
    pub target_scope: ElementId,
    pub identification_property: SpreadsheetIdentificationProperty,
    pub search_scope: SpreadsheetSearchScope,
    pub source_namespace: String,
    pub mapping_version: String,
    pub column_mappings: Vec<SpreadsheetColumnMapping>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpreadsheetImportMapGroup {
    /// Mappings are executed in this order so later mappings can resolve owners
    /// explicitly created by earlier mappings.
    pub mappings: Vec<SpreadsheetImportMap>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpreadsheetRowAction {
    Create,
    Update,
    NoChange,
    Warning,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpreadsheetDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetImportDiagnostic {
    pub severity: SpreadsheetDiagnosticSeverity,
    pub code: String,
    pub source: Option<String>,
    pub worksheet: Option<String>,
    pub row: Option<usize>,
    pub column: Option<String>,
    pub import_map: Option<String>,
    pub element_kind: Option<ElementKind>,
    pub relationship_kind: Option<RelationshipKind>,
    pub source_endpoint: Option<String>,
    pub target_endpoint: Option<String>,
    pub source_namespace: Option<String>,
    pub identification_value: Option<String>,
    pub semantic_property: Option<SpreadsheetSemanticProperty>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetRowPreview {
    pub import_map: String,
    pub source: String,
    pub worksheet: Option<String>,
    pub row: usize,
    pub element_kind: ElementKind,
    pub relationship_kind: Option<RelationshipKind>,
    pub identification_value: Option<String>,
    pub action: SpreadsheetRowAction,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetImportTotals {
    pub create: usize,
    pub update: usize,
    pub no_change: usize,
    pub warnings: usize,
    pub blocked: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpreadsheetImportPreview {
    pub applied: bool,
    pub rows: Vec<SpreadsheetRowPreview>,
    pub diagnostics: Vec<SpreadsheetImportDiagnostic>,
    pub totals: SpreadsheetImportTotals,
}

impl SpreadsheetImportPreview {
    pub fn is_valid(&self) -> bool {
        self.totals.blocked == 0
            && !self
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == SpreadsheetDiagnosticSeverity::Error)
    }

    fn recount(&mut self) {
        self.totals = SpreadsheetImportTotals::default();
        for row in &self.rows {
            match row.action {
                SpreadsheetRowAction::Create => self.totals.create += 1,
                SpreadsheetRowAction::Update => self.totals.update += 1,
                SpreadsheetRowAction::NoChange => self.totals.no_change += 1,
                SpreadsheetRowAction::Warning => self.totals.warnings += 1,
                SpreadsheetRowAction::Blocked => self.totals.blocked += 1,
            }
        }
        self.totals.warnings += self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == SpreadsheetDiagnosticSeverity::Warning)
            .count();
        if self.rows.is_empty()
            && self
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == SpreadsheetDiagnosticSeverity::Error)
        {
            self.totals.blocked = 1;
        }
    }
}

#[derive(Debug, Clone)]
struct SpreadsheetRow {
    row_number: usize,
    values: Vec<String>,
}

#[derive(Debug, Clone)]
struct SpreadsheetTable {
    headers: Vec<String>,
    rows: Vec<SpreadsheetRow>,
}

#[derive(Debug, Clone)]
struct RowContext {
    import_map: String,
    source: String,
    worksheet: Option<String>,
    row: usize,
    element_kind: ElementKind,
    relationship_kind: Option<RelationshipKind>,
    source_endpoint: Option<String>,
    target_endpoint: Option<String>,
    source_namespace: String,
    identification_value: Option<String>,
}

#[derive(Debug, Clone)]
struct PlannedElement {
    external_id: String,
    kind: ElementKind,
    qualified_name: String,
    requirement_id: Option<String>,
    depth_from_target: usize,
}

#[derive(Debug, Clone)]
struct ResolvedOwner {
    reference: ElementReference,
    qualified_name: String,
    kind: ElementKind,
    depth_from_target: usize,
}

struct PreparedSpreadsheetImport {
    plan: ModelBuildPlan,
    preview: SpreadsheetImportPreview,
    operation_contexts: Vec<RowContext>,
}

fn supported_kind(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Package
            | ElementKind::ModelLibrary
            | ElementKind::Block
            | ElementKind::AssociationBlock
            | ElementKind::InterfaceBlock
            | ElementKind::ConstraintBlock
            | ElementKind::ValueType
            | ElementKind::DataType
            | ElementKind::PrimitiveType
            | ElementKind::Enumeration
            | ElementKind::Signal
            | ElementKind::Actor
            | ElementKind::UseCase
            | ElementKind::Requirement
            | ElementKind::TestCase
            | ElementKind::PartProperty
            | ElementKind::ReferenceProperty
            | ElementKind::ValueProperty
            | ElementKind::FlowProperty
            | ElementKind::ConstraintProperty
            | ElementKind::ConstraintParameter
    )
}

fn is_namespace_kind(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Model | ElementKind::Package | ElementKind::ModelLibrary
    )
}

fn is_feature_kind(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::PartProperty
            | ElementKind::ReferenceProperty
            | ElementKind::ValueProperty
            | ElementKind::FlowProperty
            | ElementKind::ConstraintProperty
            | ElementKind::ConstraintParameter
    )
}

fn supported_relationship_kind(kind: &RelationshipKind) -> bool {
    matches!(
        kind,
        RelationshipKind::Association
            | RelationshipKind::Generalization
            | RelationshipKind::Dependency
            | RelationshipKind::Realization
            | RelationshipKind::Allocate
            | RelationshipKind::DeriveRequirement
            | RelationshipKind::Satisfy
            | RelationshipKind::Verify
            | RelationshipKind::Refine
            | RelationshipKind::Trace
            | RelationshipKind::Copy
    )
}

fn is_pr41_traceability_kind(kind: &RelationshipKind) -> bool {
    matches!(
        kind,
        RelationshipKind::DeriveRequirement
            | RelationshipKind::Satisfy
            | RelationshipKind::Verify
            | RelationshipKind::Refine
            | RelationshipKind::Trace
            | RelationshipKind::Copy
    )
}

fn is_relationship_map(map: &SpreadsheetImportMap) -> bool {
    map.relationship_kind.is_some()
        || map
            .column_mappings
            .iter()
            .any(|mapping| mapping.property == SpreadsheetSemanticProperty::RelationshipKind)
}

fn reference_error_code(property: SpreadsheetSemanticProperty, ambiguous: bool) -> &'static str {
    match (property, ambiguous) {
        (SpreadsheetSemanticProperty::Owner, true) => "OWNER_AMBIGUOUS",
        (SpreadsheetSemanticProperty::Owner, false) => "OWNER_UNRESOLVED",
        (SpreadsheetSemanticProperty::Type, true) => "TYPE_AMBIGUOUS",
        (SpreadsheetSemanticProperty::Type, false) => "TYPE_UNRESOLVED",
        (SpreadsheetSemanticProperty::Source, true) => "SOURCE_AMBIGUOUS",
        (SpreadsheetSemanticProperty::Source, false) => "SOURCE_UNRESOLVED",
        (SpreadsheetSemanticProperty::Target, true) => "TARGET_AMBIGUOUS",
        (SpreadsheetSemanticProperty::Target, false) => "TARGET_UNRESOLVED",
        (_, true) => "REFERENCE_AMBIGUOUS",
        (_, false) => "REFERENCE_UNRESOLVED",
    }
}

fn diagnostic(
    map: Option<&SpreadsheetImportMap>,
    row: Option<usize>,
    column: Option<String>,
    property: Option<SpreadsheetSemanticProperty>,
    identification_value: Option<String>,
    code: &str,
    reason: impl Into<String>,
) -> SpreadsheetImportDiagnostic {
    SpreadsheetImportDiagnostic {
        severity: SpreadsheetDiagnosticSeverity::Error,
        code: code.into(),
        source: map.map(|mapping| mapping.source.clone()),
        worksheet: map.and_then(|mapping| mapping.worksheet.clone()),
        row,
        column,
        import_map: map.map(|mapping| mapping.name.clone()),
        element_kind: map.map(|mapping| mapping.element_kind.clone()),
        relationship_kind: map.and_then(|mapping| mapping.relationship_kind.clone()),
        source_endpoint: None,
        target_endpoint: None,
        source_namespace: map.map(|mapping| mapping.source_namespace.clone()),
        identification_value,
        semantic_property: property,
        reason: reason.into(),
    }
}

fn read_spreadsheet_table(
    map: &SpreadsheetImportMap,
) -> Result<SpreadsheetTable, SpreadsheetImportDiagnostic> {
    if map.header_row == 0 {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "HEADER_ROW_INVALID",
            "header_row is one-based and must be at least 1",
        ));
    }
    let extension = Path::new(&map.source)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "csv" => read_csv_table(map),
        "xlsx" => read_xlsx_table(map),
        _ => Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "SPREADSHEET_FORMAT_UNSUPPORTED",
            format!("PR38 supports .xlsx and .csv; received .{extension}"),
        )),
    }
}

fn read_csv_table(
    map: &SpreadsheetImportMap,
) -> Result<SpreadsheetTable, SpreadsheetImportDiagnostic> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(&map.source)
        .map_err(|error| {
            diagnostic(
                Some(map),
                None,
                None,
                None,
                None,
                "CSV_READ_FAILED",
                error.to_string(),
            )
        })?;
    let mut records = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|error| {
            diagnostic(
                Some(map),
                None,
                None,
                None,
                None,
                "CSV_READ_FAILED",
                error.to_string(),
            )
        })?;
        records.push(record.iter().map(ToOwned::to_owned).collect::<Vec<_>>());
    }
    table_from_rows(map, 1, records)
}

fn read_xlsx_table(
    map: &SpreadsheetImportMap,
) -> Result<SpreadsheetTable, SpreadsheetImportDiagnostic> {
    let worksheet = map
        .worksheet
        .as_deref()
        .map(str::trim)
        .filter(|worksheet| !worksheet.is_empty())
        .ok_or_else(|| {
            diagnostic(
                Some(map),
                None,
                None,
                None,
                None,
                "WORKSHEET_REQUIRED",
                "XLSX mappings require an explicit worksheet name",
            )
        })?;
    let mut workbook = open_workbook_auto(&map.source).map_err(|error| {
        diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "XLSX_READ_FAILED",
            error.to_string(),
        )
    })?;
    let range = workbook.worksheet_range(worksheet).map_err(|error| {
        diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "WORKSHEET_NOT_FOUND",
            format!("worksheet '{worksheet}' could not be read: {error}"),
        )
    })?;
    let first_physical_row = range.start().map(|(row, _)| row as usize + 1).unwrap_or(1);
    let rows = range
        .rows()
        .map(|row| row.iter().map(ToString::to_string).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    table_from_rows(map, first_physical_row, rows)
}

fn table_from_rows(
    map: &SpreadsheetImportMap,
    first_physical_row: usize,
    rows: Vec<Vec<String>>,
) -> Result<SpreadsheetTable, SpreadsheetImportDiagnostic> {
    if map.header_row < first_physical_row {
        return Err(diagnostic(
            Some(map),
            Some(map.header_row),
            None,
            None,
            None,
            "HEADER_ROW_NOT_FOUND",
            format!(
                "header row {} is before the first populated row {}",
                map.header_row, first_physical_row
            ),
        ));
    }
    let header_index = map.header_row - first_physical_row;
    let headers = rows.get(header_index).cloned().ok_or_else(|| {
        diagnostic(
            Some(map),
            Some(map.header_row),
            None,
            None,
            None,
            "HEADER_ROW_NOT_FOUND",
            format!("header row {} does not exist", map.header_row),
        )
    })?;
    let headers = headers
        .into_iter()
        .map(|header| header.trim().to_string())
        .collect::<Vec<_>>();
    let data_rows = rows
        .into_iter()
        .enumerate()
        .skip(header_index + 1)
        .filter_map(|(index, values)| {
            let values = values
                .into_iter()
                .map(|value| value.trim().to_string())
                .collect::<Vec<_>>();
            (!values.iter().all(String::is_empty)).then_some(SpreadsheetRow {
                row_number: first_physical_row + index,
                values,
            })
        })
        .collect();
    Ok(SpreadsheetTable {
        headers,
        rows: data_rows,
    })
}

fn column_indexes(
    map: &SpreadsheetImportMap,
    table: &SpreadsheetTable,
) -> Result<BTreeMap<SpreadsheetSemanticProperty, usize>, SpreadsheetImportDiagnostic> {
    let mut header_indexes: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, header) in table.headers.iter().enumerate() {
        header_indexes
            .entry(header.as_str())
            .or_default()
            .push(index);
    }
    let mut result = BTreeMap::new();
    for column_mapping in &map.column_mappings {
        let source_column = column_mapping.source_column.trim();
        let matches = header_indexes
            .get(source_column)
            .cloned()
            .unwrap_or_default();
        match matches.as_slice() {
            [] => {
                return Err(diagnostic(
                    Some(map),
                    Some(map.header_row),
                    Some(source_column.into()),
                    Some(column_mapping.property),
                    None,
                    "MAPPED_COLUMN_MISSING",
                    format!(
                        "mapped column '{source_column}' is not present in the configured header row"
                    ),
                ));
            }
            [_] => {}
            _ => {
                return Err(diagnostic(
                    Some(map),
                    Some(map.header_row),
                    Some(source_column.into()),
                    Some(column_mapping.property),
                    None,
                    "MAPPED_COLUMN_AMBIGUOUS",
                    format!("mapped column '{source_column}' appears more than once"),
                ));
            }
        }
        if result.insert(column_mapping.property, matches[0]).is_some() {
            return Err(diagnostic(
                Some(map),
                Some(map.header_row),
                Some(source_column.into()),
                Some(column_mapping.property),
                None,
                "SEMANTIC_PROPERTY_DUPLICATE",
                format!(
                    "semantic property {:?} is mapped more than once",
                    column_mapping.property
                ),
            ));
        }
    }
    Ok(result)
}

fn mapped_values(
    row: &SpreadsheetRow,
    indexes: &BTreeMap<SpreadsheetSemanticProperty, usize>,
) -> BTreeMap<SpreadsheetSemanticProperty, String> {
    indexes
        .iter()
        .map(|(property, index)| {
            (
                *property,
                row.values
                    .get(*index)
                    .cloned()
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            )
        })
        .collect()
}

fn non_empty_value(
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    property: SpreadsheetSemanticProperty,
) -> Option<&str> {
    values
        .get(&property)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn mapped_column_name(
    map: &SpreadsheetImportMap,
    property: SpreadsheetSemanticProperty,
) -> Option<String> {
    map.column_mappings
        .iter()
        .find(|mapping| mapping.property == property)
        .map(|mapping| mapping.source_column.clone())
}

fn validate_map(
    map: &SpreadsheetImportMap,
    project: &Project,
) -> Result<(), SpreadsheetImportDiagnostic> {
    if map.name.trim().is_empty() {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "IMPORT_MAP_NAME_REQUIRED",
            "import map name is required",
        ));
    }
    if map.source_namespace.trim().is_empty() {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "SOURCE_NAMESPACE_REQUIRED",
            "source namespace is required and is not derived from the file name",
        ));
    }
    if map.mapping_version.trim().is_empty() {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "MAPPING_VERSION_REQUIRED",
            "mapping version is required",
        ));
    }
    let has_property = |property| {
        map.column_mappings
            .iter()
            .any(|mapping| mapping.property == property)
    };
    if is_relationship_map(map) {
        if let Some(kind) = &map.relationship_kind
            && !supported_relationship_kind(kind)
        {
            return Err(diagnostic(
                Some(map),
                None,
                mapped_column_name(map, SpreadsheetSemanticProperty::RelationshipKind),
                Some(SpreadsheetSemanticProperty::RelationshipKind),
                None,
                "RELATIONSHIP_KIND_UNSUPPORTED",
                format!(
                    "{:?} is outside the PR40/PR41/PR42 relationship scope",
                    kind
                ),
            ));
        }
        if map.relationship_kind.is_none()
            && !has_property(SpreadsheetSemanticProperty::RelationshipKind)
        {
            return Err(diagnostic(
                Some(map),
                None,
                None,
                Some(SpreadsheetSemanticProperty::RelationshipKind),
                None,
                "RELATIONSHIP_KIND_REQUIRED",
                "relationship mappings require a configured relationship_kind or mapped RelationshipKind column",
            ));
        }
        for property in [
            SpreadsheetSemanticProperty::Source,
            SpreadsheetSemanticProperty::Target,
        ] {
            if !has_property(property) {
                return Err(diagnostic(
                    Some(map),
                    None,
                    None,
                    Some(property),
                    None,
                    "RELATIONSHIP_COLUMN_REQUIRED",
                    format!(
                        "PR40 relationship mappings require a mapped {:?} column",
                        property
                    ),
                ));
            }
        }
        if map.relationship_identity == SpreadsheetRelationshipIdentityPolicy::ExternalId
            && !has_property(SpreadsheetSemanticProperty::ExternalId)
        {
            return Err(diagnostic(
                Some(map),
                None,
                None,
                Some(SpreadsheetSemanticProperty::ExternalId),
                None,
                "RELATIONSHIP_EXTERNAL_ID_REQUIRED",
                "ExternalId relationship identity requires a mapped External ID column",
            ));
        }
        if [
            SpreadsheetSemanticProperty::Type,
            SpreadsheetSemanticProperty::Multiplicity,
            SpreadsheetSemanticProperty::DefaultValue,
            SpreadsheetSemanticProperty::FlowDirection,
            SpreadsheetSemanticProperty::RequirementId,
            SpreadsheetSemanticProperty::RequirementText,
        ]
        .into_iter()
        .any(has_property)
        {
            return Err(diagnostic(
                Some(map),
                None,
                None,
                None,
                None,
                "SEMANTIC_PROPERTY_INVALID",
                "element/feature-only mapped fields cannot be used by PR40 relationship mappings",
            ));
        }
        let association_fields = [
            SpreadsheetSemanticProperty::SourceEndRole,
            SpreadsheetSemanticProperty::TargetEndRole,
            SpreadsheetSemanticProperty::SourceMultiplicity,
            SpreadsheetSemanticProperty::TargetMultiplicity,
            SpreadsheetSemanticProperty::SourceNavigable,
            SpreadsheetSemanticProperty::TargetNavigable,
            SpreadsheetSemanticProperty::SourceAggregation,
            SpreadsheetSemanticProperty::TargetAggregation,
        ];
        if map
            .relationship_kind
            .as_ref()
            .is_some_and(|kind| *kind != RelationshipKind::Association)
            && association_fields.into_iter().any(has_property)
        {
            return Err(diagnostic(
                Some(map),
                None,
                None,
                None,
                None,
                "ASSOCIATION_FIELD_INVALID",
                "Association-end fields can be mapped only for Association rows",
            ));
        }
        let target = project.element(map.target_scope).map_err(|_| {
            diagnostic(
                Some(map),
                None,
                None,
                None,
                None,
                "TARGET_SCOPE_UNRESOLVED",
                format!("target scope {} does not resolve", map.target_scope),
            )
        })?;
        if !target.is_namespace() {
            return Err(diagnostic(
                Some(map),
                None,
                None,
                None,
                None,
                "TARGET_SCOPE_INVALID",
                format!(
                    "target '{}' ({:?}) is not a semantic namespace",
                    target.name, target.kind
                ),
            ));
        }
        return Ok(());
    }

    if !supported_kind(&map.element_kind) {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "ELEMENT_KIND_UNSUPPORTED",
            format!(
                "{:?} is outside the PR39 package/basic-element/owned-feature scope",
                map.element_kind
            ),
        ));
    }
    if map.element_kind != ElementKind::Requirement
        && (has_property(SpreadsheetSemanticProperty::RequirementId)
            || has_property(SpreadsheetSemanticProperty::RequirementText))
    {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "SEMANTIC_PROPERTY_INVALID",
            "Requirement ID/Text columns can be mapped only for Requirement elements",
        ));
    }
    if is_feature_kind(&map.element_kind) && !has_property(SpreadsheetSemanticProperty::Type) {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            Some(SpreadsheetSemanticProperty::Type),
            None,
            "FEATURE_TYPE_COLUMN_REQUIRED",
            format!(
                "{:?} mappings require an explicit Type column",
                map.element_kind
            ),
        ));
    }
    if !is_feature_kind(&map.element_kind)
        && (has_property(SpreadsheetSemanticProperty::Type)
            || has_property(SpreadsheetSemanticProperty::Multiplicity)
            || has_property(SpreadsheetSemanticProperty::DefaultValue)
            || has_property(SpreadsheetSemanticProperty::FlowDirection))
    {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "SEMANTIC_PROPERTY_INVALID",
            "Type/Multiplicity/Default Value/Flow Direction mappings are reserved for PR39 owned features",
        ));
    }
    if map.element_kind != ElementKind::ValueProperty
        && has_property(SpreadsheetSemanticProperty::DefaultValue)
    {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            Some(SpreadsheetSemanticProperty::DefaultValue),
            None,
            "SEMANTIC_PROPERTY_INVALID",
            "Default Value can be mapped only for ValueProperty",
        ));
    }
    if map.element_kind == ElementKind::FlowProperty {
        if !has_property(SpreadsheetSemanticProperty::FlowDirection) {
            return Err(diagnostic(
                Some(map),
                None,
                None,
                Some(SpreadsheetSemanticProperty::FlowDirection),
                None,
                "FLOW_DIRECTION_COLUMN_REQUIRED",
                "FlowProperty mappings require an explicit Flow Direction column",
            ));
        }
    } else if has_property(SpreadsheetSemanticProperty::FlowDirection) {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            Some(SpreadsheetSemanticProperty::FlowDirection),
            None,
            "SEMANTIC_PROPERTY_INVALID",
            "Flow Direction can be mapped only for FlowProperty",
        ));
    }

    let target = project.element(map.target_scope).map_err(|_| {
        diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "TARGET_SCOPE_UNRESOLVED",
            format!("target scope {} does not resolve", map.target_scope),
        )
    })?;
    if !target.is_namespace() {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "TARGET_SCOPE_INVALID",
            format!(
                "target '{}' ({:?}) is not a semantic namespace",
                target.name, target.kind
            ),
        ));
    }
    let required_identification_property = match map.identification_property {
        SpreadsheetIdentificationProperty::ExternalId => SpreadsheetSemanticProperty::ExternalId,
        SpreadsheetIdentificationProperty::Name => SpreadsheetSemanticProperty::Name,
        SpreadsheetIdentificationProperty::RequirementId => {
            SpreadsheetSemanticProperty::RequirementId
        }
    };
    if map.identification_property == SpreadsheetIdentificationProperty::RequirementId
        && map.element_kind != ElementKind::Requirement
    {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            Some(SpreadsheetSemanticProperty::RequirementId),
            None,
            "IDENTIFICATION_PROPERTY_INVALID",
            "RequirementId identification is valid only for Requirement mappings",
        ));
    }
    if !map
        .column_mappings
        .iter()
        .any(|mapping| mapping.property == required_identification_property)
    {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            Some(required_identification_property),
            None,
            "IDENTIFICATION_COLUMN_REQUIRED",
            format!(
                "{:?} identification requires a mapped {:?} column",
                map.identification_property, required_identification_property
            ),
        ));
    }
    Ok(())
}

fn distance_from_target(
    project: &Project,
    element_id: ElementId,
    target: ElementId,
) -> Option<usize> {
    if element_id == target {
        return Some(0);
    }
    let mut current = project.element(element_id).ok()?.owner_id;
    let mut distance = 1usize;
    let mut visited = HashSet::new();
    while let Some(id) = current {
        if !visited.insert(id) {
            return None;
        }
        if id == target {
            return Some(distance);
        }
        current = project.element(id).ok()?.owner_id;
        distance += 1;
    }
    None
}

fn existing_match_in_scope(
    project: &Project,
    element: &Element,
    target: ElementId,
    search_scope: SpreadsheetSearchScope,
) -> bool {
    match search_scope {
        SpreadsheetSearchScope::TargetOnly => element.owner_id == Some(target),
        SpreadsheetSearchScope::TargetRecursive => {
            element.id != target && distance_from_target(project, element.id, target).is_some()
        }
    }
}

fn reference_in_scope(
    project: &Project,
    element_id: ElementId,
    target: ElementId,
    search_scope: SpreadsheetSearchScope,
) -> bool {
    if element_id == target {
        return true;
    }
    match search_scope {
        SpreadsheetSearchScope::TargetOnly => project
            .element(element_id)
            .ok()
            .is_some_and(|element| element.owner_id == Some(target)),
        SpreadsheetSearchScope::TargetRecursive => {
            distance_from_target(project, element_id, target).is_some()
        }
    }
}

fn qname_aliases(canonical: &str, root_name: &str, target_qname: &str) -> Vec<String> {
    let mut aliases = vec![canonical.to_string()];
    if let Some(relative) = canonical.strip_prefix(&format!("{root_name}::")) {
        aliases.push(relative.to_string());
    }
    if canonical == target_qname {
        aliases.push(
            target_qname
                .rsplit("::")
                .next()
                .unwrap_or(target_qname)
                .to_string(),
        );
    } else if let Some(relative) = canonical.strip_prefix(&format!("{target_qname}::")) {
        aliases.push(relative.to_string());
    }
    aliases.sort();
    aliases.dedup();
    aliases
}

fn resolve_semantic_reference(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned: &[PlannedElement],
    requested: &str,
    property: SpreadsheetSemanticProperty,
    label: &str,
    allow_requirement_id: bool,
) -> Result<ResolvedOwner, SpreadsheetImportDiagnostic> {
    let target_qname = project.qualified_name(map.target_scope).map_err(|error| {
        diagnostic(
            Some(map),
            None,
            None,
            Some(property),
            None,
            "TARGET_SCOPE_UNRESOLVED",
            error.to_string(),
        )
    })?;
    let root_name = project
        .element(project.root_id)
        .map(|root| root.name.as_str())
        .unwrap_or_default();
    let requested = requested.trim();

    let mut existing_external = project
        .elements
        .values()
        .filter(|element| {
            reference_in_scope(project, element.id, map.target_scope, map.search_scope)
        })
        .filter(|element| element.external_id == external_key(&map.source_namespace, requested))
        .collect::<Vec<_>>();
    existing_external.sort_by_key(|element| element.id.to_string());
    existing_external.dedup_by_key(|element| element.id);
    let mut pending_external = planned
        .iter()
        .filter(|element| match map.search_scope {
            SpreadsheetSearchScope::TargetOnly => element.depth_from_target == 1,
            SpreadsheetSearchScope::TargetRecursive => element.depth_from_target >= 1,
        })
        .filter(|element| element.external_id == requested)
        .collect::<Vec<_>>();
    pending_external.sort_by(|left, right| left.external_id.cmp(&right.external_id));
    pending_external.dedup_by(|left, right| left.external_id == right.external_id);
    match (existing_external.as_slice(), pending_external.as_slice()) {
        ([element], []) => {
            let qualified_name = project
                .qualified_name(element.id)
                .unwrap_or_else(|_| element.name.clone());
            return Ok(ResolvedOwner {
                reference: BuildReference::Existing(element.id),
                qualified_name,
                kind: element.kind.clone(),
                depth_from_target: distance_from_target(project, element.id, map.target_scope)
                    .unwrap_or(0),
            });
        }
        ([], [element]) => {
            return Ok(ResolvedOwner {
                reference: BuildReference::External(element.external_id.clone()),
                qualified_name: element.qualified_name.clone(),
                kind: element.kind.clone(),
                depth_from_target: element.depth_from_target,
            });
        }
        ([], []) => {}
        _ => {
            return Err(diagnostic(
                Some(map),
                None,
                mapped_column_name(map, property),
                Some(property),
                Some(requested.into()),
                reference_error_code(property, true),
                format!(
                    "{label} '{requested}' resolves to more than one element by source external identity"
                ),
            ));
        }
    }

    if allow_requirement_id {
        let mut existing_requirement = project
            .elements
            .values()
            .filter(|element| {
                reference_in_scope(project, element.id, map.target_scope, map.search_scope)
            })
            .filter(|element| {
                element.kind == ElementKind::Requirement
                    && element.requirement_id.as_deref() == Some(requested)
            })
            .collect::<Vec<_>>();
        existing_requirement.sort_by_key(|element| element.id.to_string());
        existing_requirement.dedup_by_key(|element| element.id);
        let mut pending_requirement = planned
            .iter()
            .filter(|element| match map.search_scope {
                SpreadsheetSearchScope::TargetOnly => element.depth_from_target == 1,
                SpreadsheetSearchScope::TargetRecursive => element.depth_from_target >= 1,
            })
            .filter(|element| {
                element.kind == ElementKind::Requirement
                    && element.requirement_id.as_deref() == Some(requested)
            })
            .collect::<Vec<_>>();
        pending_requirement.sort_by(|left, right| left.external_id.cmp(&right.external_id));
        pending_requirement.dedup_by(|left, right| left.external_id == right.external_id);
        match (
            existing_requirement.as_slice(),
            pending_requirement.as_slice(),
        ) {
            ([element], []) => {
                let qualified_name = project
                    .qualified_name(element.id)
                    .unwrap_or_else(|_| element.name.clone());
                return Ok(ResolvedOwner {
                    reference: BuildReference::Existing(element.id),
                    qualified_name,
                    kind: element.kind.clone(),
                    depth_from_target: distance_from_target(project, element.id, map.target_scope)
                        .unwrap_or(0),
                });
            }
            ([], [element]) => {
                return Ok(ResolvedOwner {
                    reference: BuildReference::External(element.external_id.clone()),
                    qualified_name: element.qualified_name.clone(),
                    kind: element.kind.clone(),
                    depth_from_target: element.depth_from_target,
                });
            }
            ([], []) => {}
            _ => {
                return Err(diagnostic(
                    Some(map),
                    None,
                    mapped_column_name(map, property),
                    Some(property),
                    Some(requested.into()),
                    "AMBIGUOUS_REQUIREMENT_ID",
                    format!(
                        "Requirement ID '{requested}' resolves to {} Requirements within the configured search scope; use External ID or a more specific identifier",
                        existing_requirement.len() + pending_requirement.len()
                    ),
                ));
            }
        }
    }

    let mut existing = project
        .elements
        .values()
        .filter(|element| {
            reference_in_scope(project, element.id, map.target_scope, map.search_scope)
        })
        .filter_map(|element| {
            let canonical = project.qualified_name(element.id).ok()?;
            qname_aliases(&canonical, root_name, &target_qname)
                .iter()
                .any(|alias| alias == requested)
                .then_some((element, canonical))
        })
        .collect::<Vec<_>>();
    existing.sort_by_key(|(element, _)| element.id.to_string());
    existing.dedup_by_key(|(element, _)| element.id);
    let mut pending = planned
        .iter()
        .filter(|element| match map.search_scope {
            SpreadsheetSearchScope::TargetOnly => element.depth_from_target == 1,
            SpreadsheetSearchScope::TargetRecursive => element.depth_from_target >= 1,
        })
        .filter(|element| {
            qname_aliases(&element.qualified_name, root_name, &target_qname)
                .iter()
                .any(|alias| alias == requested)
        })
        .collect::<Vec<_>>();
    pending.sort_by(|left, right| left.external_id.cmp(&right.external_id));
    pending.dedup_by(|left, right| left.external_id == right.external_id);
    match (existing.as_slice(), pending.as_slice()) {
        ([(element, qualified_name)], []) => Ok(ResolvedOwner {
            reference: BuildReference::Existing(element.id),
            qualified_name: qualified_name.clone(),
            kind: element.kind.clone(),
            depth_from_target: distance_from_target(project, element.id, map.target_scope)
                .unwrap_or(0),
        }),
        ([], [element]) => Ok(ResolvedOwner {
            reference: BuildReference::External(element.external_id.clone()),
            qualified_name: element.qualified_name.clone(),
            kind: element.kind.clone(),
            depth_from_target: element.depth_from_target,
        }),
        ([], []) => Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, property),
            Some(property),
            Some(requested.into()),
            reference_error_code(property, false),
            format!(
                "{label} '{requested}' could not be resolved by {} within {:?} search scope",
                if allow_requirement_id {
                    "namespaced External ID, exact Requirement ID, or exact qualified name"
                } else {
                    "namespaced External ID or exact qualified name"
                },
                map.search_scope
            ),
        )),
        _ => Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, property),
            Some(property),
            Some(requested.into()),
            reference_error_code(property, true),
            format!("{label} '{requested}' resolves to more than one semantic element"),
        )),
    }
}

fn resolve_owner(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned: &[PlannedElement],
    value: Option<&str>,
) -> Result<ResolvedOwner, SpreadsheetImportDiagnostic> {
    let target = project.element(map.target_scope).expect("validated target");
    let target_qname = project.qualified_name(map.target_scope).map_err(|error| {
        diagnostic(
            Some(map),
            None,
            None,
            Some(SpreadsheetSemanticProperty::Owner),
            None,
            "TARGET_SCOPE_UNRESOLVED",
            error.to_string(),
        )
    })?;
    let Some(requested) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(ResolvedOwner {
            reference: BuildReference::Existing(map.target_scope),
            qualified_name: target_qname,
            kind: target.kind.clone(),
            depth_from_target: 0,
        });
    };
    resolve_semantic_reference(
        map,
        project,
        planned,
        requested,
        SpreadsheetSemanticProperty::Owner,
        "Owner",
        false,
    )
}

fn resolve_type(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned: &[PlannedElement],
    value: Option<&str>,
) -> Result<ResolvedOwner, SpreadsheetImportDiagnostic> {
    let requested = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            diagnostic(
                Some(map),
                None,
                mapped_column_name(map, SpreadsheetSemanticProperty::Type),
                Some(SpreadsheetSemanticProperty::Type),
                None,
                "TYPE_REQUIRED",
                format!("{:?} requires a non-empty Type", map.element_kind),
            )
        })?;
    resolve_semantic_reference(
        map,
        project,
        planned,
        requested,
        SpreadsheetSemanticProperty::Type,
        "Type",
        false,
    )
}

fn find_by_external_id<'a>(
    map: &SpreadsheetImportMap,
    project: &'a Project,
    external_id: &str,
) -> Result<Option<&'a Element>, SpreadsheetImportDiagnostic> {
    let expected = external_key(&map.source_namespace, external_id);
    let matches = project
        .elements
        .values()
        .filter(|element| element.external_id == expected)
        .filter(|element| {
            existing_match_in_scope(project, element, map.target_scope, map.search_scope)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [element] if element.kind == map.element_kind => Ok(Some(*element)),
        [element] => Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId),
            Some(external_id.into()),
            "IDENTIFICATION_KIND_MISMATCH",
            format!(
                "external ID identifies {:?}, not {:?}",
                element.kind, map.element_kind
            ),
        )),
        _ => Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId),
            Some(external_id.into()),
            "AMBIGUOUS_IDENTIFICATION",
            format!("external ID '{external_id}' identifies more than one element"),
        )),
    }
}

fn find_existing<'a>(
    map: &SpreadsheetImportMap,
    project: &'a Project,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<Option<&'a Element>, SpreadsheetImportDiagnostic> {
    if let Some(external_id) = non_empty_value(values, SpreadsheetSemanticProperty::ExternalId) {
        if let Some(element) = find_by_external_id(map, project, external_id)? {
            return Ok(Some(element));
        }
        if map.identification_property == SpreadsheetIdentificationProperty::ExternalId {
            return Ok(None);
        }
    }

    let (property, identification_value) = match map.identification_property {
        SpreadsheetIdentificationProperty::ExternalId => (
            SpreadsheetSemanticProperty::ExternalId,
            non_empty_value(values, SpreadsheetSemanticProperty::ExternalId),
        ),
        SpreadsheetIdentificationProperty::Name => (
            SpreadsheetSemanticProperty::Name,
            non_empty_value(values, SpreadsheetSemanticProperty::Name),
        ),
        SpreadsheetIdentificationProperty::RequirementId => (
            SpreadsheetSemanticProperty::RequirementId,
            non_empty_value(values, SpreadsheetSemanticProperty::RequirementId),
        ),
    };
    let identification_value = identification_value.ok_or_else(|| {
        diagnostic(
            Some(map),
            None,
            mapped_column_name(map, property),
            Some(property),
            None,
            "IDENTIFICATION_VALUE_REQUIRED",
            format!(
                "{:?} identification value is blank",
                map.identification_property
            ),
        )
    })?;
    let matches = project
        .elements
        .values()
        .filter(|element| element.kind == map.element_kind)
        .filter(|element| {
            existing_match_in_scope(project, element, map.target_scope, map.search_scope)
        })
        .filter(|element| match map.identification_property {
            SpreadsheetIdentificationProperty::ExternalId => {
                element.external_id == external_key(&map.source_namespace, identification_value)
            }
            SpreadsheetIdentificationProperty::Name => element.name == identification_value,
            SpreadsheetIdentificationProperty::RequirementId => {
                element.requirement_id.as_deref() == Some(identification_value)
            }
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [element] => Ok(Some(*element)),
        _ => Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, property),
            Some(property),
            Some(identification_value.into()),
            "AMBIGUOUS_IDENTIFICATION",
            format!(
                "{:?} '{}' identifies {} {:?} elements in the configured search scope",
                map.identification_property,
                identification_value,
                matches.len(),
                map.element_kind
            ),
        )),
    }
}

fn parse_visibility(
    map: &SpreadsheetImportMap,
    row: usize,
    value: &str,
) -> Result<VisibilityKind, SpreadsheetImportDiagnostic> {
    match value.trim().to_ascii_lowercase().as_str() {
        "public" | "+" => Ok(VisibilityKind::Public),
        "private" | "-" => Ok(VisibilityKind::Private),
        _ => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::Visibility),
            Some(SpreadsheetSemanticProperty::Visibility),
            None,
            "VISIBILITY_INVALID",
            format!("visibility '{value}' must be Public or Private"),
        )),
    }
}

fn identification_value(
    map: &SpreadsheetImportMap,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Option<String> {
    if is_relationship_map(map) {
        return non_empty_value(values, SpreadsheetSemanticProperty::ExternalId)
            .map(ToOwned::to_owned)
            .or_else(|| {
                let source = non_empty_value(values, SpreadsheetSemanticProperty::Source)?;
                let target = non_empty_value(values, SpreadsheetSemanticProperty::Target)?;
                Some(format!("{source} -> {target}"))
            });
    }
    let property = match map.identification_property {
        SpreadsheetIdentificationProperty::ExternalId => SpreadsheetSemanticProperty::ExternalId,
        SpreadsheetIdentificationProperty::Name => SpreadsheetSemanticProperty::Name,
        SpreadsheetIdentificationProperty::RequirementId => {
            SpreadsheetSemanticProperty::RequirementId
        }
    };
    non_empty_value(values, property).map(ToOwned::to_owned)
}

fn row_preview(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    action: SpreadsheetRowAction,
) -> SpreadsheetRowPreview {
    SpreadsheetRowPreview {
        import_map: map.name.clone(),
        source: map.source.clone(),
        worksheet: map.worksheet.clone(),
        row,
        element_kind: map.element_kind.clone(),
        relationship_kind: if is_relationship_map(map) {
            relationship_kind_for_row(map, row, values).ok()
        } else {
            None
        },
        identification_value: identification_value(map, values),
        action,
    }
}

fn row_context(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> RowContext {
    RowContext {
        import_map: map.name.clone(),
        source: map.source.clone(),
        worksheet: map.worksheet.clone(),
        row,
        element_kind: map.element_kind.clone(),
        relationship_kind: if is_relationship_map(map) {
            relationship_kind_for_row(map, row, values).ok()
        } else {
            None
        },
        source_endpoint: non_empty_value(values, SpreadsheetSemanticProperty::Source)
            .map(ToOwned::to_owned),
        target_endpoint: non_empty_value(values, SpreadsheetSemanticProperty::Target)
            .map(ToOwned::to_owned),
        source_namespace: map.source_namespace.clone(),
        identification_value: identification_value(map, values),
    }
}

fn parse_relationship_kind_value(
    map: &SpreadsheetImportMap,
    row: usize,
    value: &str,
) -> Result<RelationshipKind, SpreadsheetImportDiagnostic> {
    let kind = match value.trim().to_ascii_lowercase().as_str() {
        "association" => RelationshipKind::Association,
        "generalization" => RelationshipKind::Generalization,
        "dependency" => RelationshipKind::Dependency,
        "realization" => RelationshipKind::Realization,
        "allocate" => RelationshipKind::Allocate,
        "deriverequirement" | "derivereqt" => RelationshipKind::DeriveRequirement,
        "satisfy" => RelationshipKind::Satisfy,
        "verify" => RelationshipKind::Verify,
        "refine" => RelationshipKind::Refine,
        "trace" => RelationshipKind::Trace,
        "copy" => RelationshipKind::Copy,
        _ => {
            return Err(diagnostic(
                Some(map),
                Some(row),
                mapped_column_name(map, SpreadsheetSemanticProperty::RelationshipKind),
                Some(SpreadsheetSemanticProperty::RelationshipKind),
                Some(value.trim().to_string()),
                "RELATIONSHIP_KIND_UNSUPPORTED",
                format!(
                    "relationship kind '{}' is outside PR40/PR41/PR42; expected Association, Generalization, Dependency, Realization, Allocate, DeriveRequirement/deriveReqt, Satisfy, Verify, Refine, Trace, or Copy",
                    value.trim()
                ),
            ));
        }
    };
    Ok(kind)
}

fn relationship_kind_for_row(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<RelationshipKind, SpreadsheetImportDiagnostic> {
    let mapped = non_empty_value(values, SpreadsheetSemanticProperty::RelationshipKind)
        .map(|value| parse_relationship_kind_value(map, row, value))
        .transpose()?;
    match (map.relationship_kind.clone(), mapped) {
        (Some(configured), Some(mapped)) if configured != mapped => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::RelationshipKind),
            Some(SpreadsheetSemanticProperty::RelationshipKind),
            Some(format!("{:?}", mapped)),
            "RELATIONSHIP_KIND_MISMATCH",
            format!(
                "row relationship kind {:?} does not match configured {:?}",
                mapped, configured
            ),
        )),
        (Some(configured), _) => Ok(configured),
        (None, Some(mapped)) => Ok(mapped),
        (None, None) => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::RelationshipKind),
            Some(SpreadsheetSemanticProperty::RelationshipKind),
            None,
            "RELATIONSHIP_KIND_REQUIRED",
            "relationship kind is blank",
        )),
    }
}

fn parse_navigable(
    map: &SpreadsheetImportMap,
    row: usize,
    property: SpreadsheetSemanticProperty,
    value: &str,
) -> Result<bool, SpreadsheetImportDiagnostic> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" => Ok(true),
        "false" | "no" | "0" => Ok(false),
        _ => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, property),
            Some(property),
            None,
            "NAVIGABILITY_INVALID",
            format!(
                "navigability '{}' must be true/false, yes/no, or 1/0",
                value
            ),
        )),
    }
}

fn parse_aggregation(
    map: &SpreadsheetImportMap,
    row: usize,
    property: SpreadsheetSemanticProperty,
    value: &str,
) -> Result<AggregationKind, SpreadsheetImportDiagnostic> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(AggregationKind::None),
        "shared" | "aggregation" => Ok(AggregationKind::Shared),
        "composite" | "composition" => Ok(AggregationKind::Composite),
        _ => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, property),
            Some(property),
            None,
            "AGGREGATION_INVALID",
            format!("aggregation '{}' must be none, shared, or composite", value),
        )),
    }
}

fn parse_end_fields(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    role_property: SpreadsheetSemanticProperty,
    multiplicity_property: SpreadsheetSemanticProperty,
    navigable_property: SpreadsheetSemanticProperty,
    aggregation_property: SpreadsheetSemanticProperty,
) -> Result<Option<AssociationEndBuildFields>, SpreadsheetImportDiagnostic> {
    if ![
        role_property,
        multiplicity_property,
        navigable_property,
        aggregation_property,
    ]
    .into_iter()
    .any(|property| non_empty_value(values, property).is_some())
    {
        return Ok(None);
    }
    let role_name = values.get(&role_property).cloned();
    let multiplicity = match non_empty_value(values, multiplicity_property) {
        Some(value) => Some(
            super::parametrics::parse_multiplicity(value).map_err(|reason| {
                diagnostic(
                    Some(map),
                    Some(row),
                    mapped_column_name(map, multiplicity_property),
                    Some(multiplicity_property),
                    None,
                    "MULTIPLICITY_INVALID",
                    format!(
                        "association-end multiplicity '{}' is invalid: {}",
                        value, reason
                    ),
                )
            })?,
        ),
        None => None,
    };
    let navigable = match non_empty_value(values, navigable_property) {
        Some(value) => Some(parse_navigable(map, row, navigable_property, value)?),
        None => None,
    };
    let aggregation = match non_empty_value(values, aggregation_property) {
        Some(value) => Some(parse_aggregation(map, row, aggregation_property, value)?),
        None => None,
    };
    Ok(Some(AssociationEndBuildFields {
        role_name,
        multiplicity,
        navigable,
        aggregation,
    }))
}

fn resolve_relationship_endpoint(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned: &[PlannedElement],
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    kind: &RelationshipKind,
    property: SpreadsheetSemanticProperty,
    label: &str,
) -> Result<ResolvedOwner, SpreadsheetImportDiagnostic> {
    let requested = non_empty_value(values, property).ok_or_else(|| {
        diagnostic(
            Some(map),
            None,
            mapped_column_name(map, property),
            Some(property),
            None,
            if property == SpreadsheetSemanticProperty::Source {
                "SOURCE_REQUIRED"
            } else {
                "TARGET_REQUIRED"
            },
            format!("{label} endpoint is blank"),
        )
    })?;
    let allow_requirement_id = matches!(
        (kind, property),
        (
            RelationshipKind::DeriveRequirement | RelationshipKind::Copy,
            SpreadsheetSemanticProperty::Source | SpreadsheetSemanticProperty::Target,
        ) | (
            RelationshipKind::Satisfy | RelationshipKind::Verify | RelationshipKind::Refine,
            SpreadsheetSemanticProperty::Target,
        ) | (
            RelationshipKind::Trace,
            SpreadsheetSemanticProperty::Source | SpreadsheetSemanticProperty::Target,
        )
    );
    resolve_semantic_reference(
        map,
        project,
        planned,
        requested,
        property,
        label,
        allow_requirement_id,
    )
    .map_err(|mut error| {
        if property == SpreadsheetSemanticProperty::Source {
            error.source_endpoint = Some(requested.to_string());
        } else {
            error.target_endpoint = Some(requested.to_string());
        }
        error
    })
}

fn relationship_in_scope(
    map: &SpreadsheetImportMap,
    project: &Project,
    relationship: &Relationship,
) -> bool {
    let Some(owner_id) = relationship.owner_id else {
        return false;
    };
    match map.search_scope {
        SpreadsheetSearchScope::TargetOnly => owner_id == map.target_scope,
        SpreadsheetSearchScope::TargetRecursive => {
            owner_id == map.target_scope
                || distance_from_target(project, owner_id, map.target_scope).is_some()
        }
    }
}

fn find_relationship_by_external_id<'a>(
    map: &SpreadsheetImportMap,
    project: &'a Project,
    external_id: &str,
    kind: &RelationshipKind,
) -> Result<Option<&'a Relationship>, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, external_id);
    if project
        .elements
        .values()
        .any(|element| element.external_id == key)
    {
        return Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId),
            Some(external_id.to_string()),
            "RELATIONSHIP_IDENTITY_KIND_MISMATCH",
            "relationship external ID is already used by an element",
        ));
    }
    let matches = project
        .relationships
        .values()
        .filter(|relationship| relationship.external_id == key)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [relationship] if &relationship.kind != kind => Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId),
            Some(external_id.to_string()),
            "RELATIONSHIP_IDENTITY_KIND_MISMATCH",
            format!(
                "external ID identifies {:?}, not {:?}",
                relationship.kind, kind
            ),
        )),
        [relationship] if !relationship_in_scope(map, project, relationship) => Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId),
            Some(external_id.to_string()),
            "RELATIONSHIP_OUTSIDE_SCOPE",
            "relationship external ID exists outside the configured target/search scope",
        )),
        [relationship] => Ok(Some(*relationship)),
        _ => Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId),
            Some(external_id.to_string()),
            "AMBIGUOUS_RELATIONSHIP",
            "relationship external ID resolves to more than one relationship",
        )),
    }
}

fn end_fields_match(
    end: &systems_modeler_core::AssociationEnd,
    fields: &Option<AssociationEndBuildFields>,
) -> bool {
    let Some(fields) = fields else {
        return true;
    };
    fields
        .role_name
        .as_ref()
        .is_none_or(|value| end.role_name == *value)
        && fields
            .multiplicity
            .is_none_or(|value| end.multiplicity == value)
        && fields.navigable.is_none_or(|value| end.navigable == value)
        && fields
            .aggregation
            .is_none_or(|value| end.aggregation == value)
}

fn find_relationship_by_fallback<'a>(
    map: &SpreadsheetImportMap,
    project: &'a Project,
    kind: &RelationshipKind,
    source: &ResolvedOwner,
    target: &ResolvedOwner,
    source_end: &Option<AssociationEndBuildFields>,
    target_end: &Option<AssociationEndBuildFields>,
) -> Result<Option<&'a Relationship>, SpreadsheetImportDiagnostic> {
    let (BuildReference::Existing(source_id), BuildReference::Existing(target_id)) =
        (&source.reference, &target.reference)
    else {
        return Ok(None);
    };
    let matches = project
        .relationships
        .values()
        .filter(|relationship| relationship_in_scope(map, project, relationship))
        .filter(|relationship| {
            relationship.kind == *kind
                && relationship.source_id == *source_id
                && relationship.target_id == *target_id
        })
        .filter(|relationship| {
            if map.relationship_identity
                != SpreadsheetRelationshipIdentityPolicy::KindSourceTargetAssociationEnds
                || *kind != RelationshipKind::Association
            {
                return true;
            }
            relationship.association_ends.len() >= 2
                && end_fields_match(&relationship.association_ends[0], source_end)
                && end_fields_match(&relationship.association_ends[1], target_end)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [relationship] => Ok(Some(*relationship)),
        _ => Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "AMBIGUOUS_RELATIONSHIP",
            format!(
                "configured fallback identity matches {} {:?} relationships",
                matches.len(),
                kind
            ),
        )),
    }
}

fn fallback_relationship_external_id(
    policy: SpreadsheetRelationshipIdentityPolicy,
    kind: &RelationshipKind,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> String {
    let source = non_empty_value(values, SpreadsheetSemanticProperty::Source).unwrap_or_default();
    let target = non_empty_value(values, SpreadsheetSemanticProperty::Target).unwrap_or_default();
    let mut identity = format!("fallback::{kind:?}::{source}=>{target}");
    if policy == SpreadsheetRelationshipIdentityPolicy::KindSourceTargetAssociationEnds
        && *kind == RelationshipKind::Association
    {
        for property in [
            SpreadsheetSemanticProperty::SourceEndRole,
            SpreadsheetSemanticProperty::TargetEndRole,
            SpreadsheetSemanticProperty::SourceMultiplicity,
            SpreadsheetSemanticProperty::TargetMultiplicity,
            SpreadsheetSemanticProperty::SourceNavigable,
            SpreadsheetSemanticProperty::TargetNavigable,
            SpreadsheetSemanticProperty::SourceAggregation,
            SpreadsheetSemanticProperty::TargetAggregation,
        ] {
            identity.push('|');
            identity.push_str(non_empty_value(values, property).unwrap_or_default());
        }
    }
    identity
}

fn relationship_reference_matches(reference: &ElementReference, existing: ElementId) -> bool {
    matches!(reference, BuildReference::Existing(id) if *id == existing)
}

fn association_end_changed(
    end: &systems_modeler_core::AssociationEnd,
    fields: &Option<AssociationEndBuildFields>,
) -> bool {
    !end_fields_match(end, fields)
}

#[allow(clippy::too_many_arguments)]
fn relationship_field_changes(
    map: &SpreadsheetImportMap,
    row: usize,
    relationship: &Relationship,
    effective_external_id: &str,
    source: &ResolvedOwner,
    target: &ResolvedOwner,
    owner: &ResolvedOwner,
    source_end: Option<AssociationEndBuildFields>,
    target_end: Option<AssociationEndBuildFields>,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<(bool, ModelBuildOperation), SpreadsheetImportDiagnostic> {
    let name = values.get(&SpreadsheetSemanticProperty::Name).cloned();
    let documentation = values
        .get(&SpreadsheetSemanticProperty::Documentation)
        .cloned();
    let visibility = values
        .get(&SpreadsheetSemanticProperty::Visibility)
        .map(|value| parse_visibility(map, row, value))
        .transpose()?;
    let external_id_explicit =
        non_empty_value(values, SpreadsheetSemanticProperty::ExternalId).is_some();
    let external_changed =
        relationship.external_id != external_key(&map.source_namespace, effective_external_id);
    let owner_changed = match (&owner.reference, relationship.owner_id) {
        (BuildReference::Existing(id), Some(existing)) => *id != existing,
        (BuildReference::Existing(_), None) => true,
        (BuildReference::External(_), _) => true,
    };
    let source_changed = !relationship_reference_matches(&source.reference, relationship.source_id);
    let target_changed = !relationship_reference_matches(&target.reference, relationship.target_id);
    let association_changed = if relationship.kind == RelationshipKind::Association {
        relationship.association_ends.len() < 2
            || relationship
                .association_ends
                .first()
                .is_some_and(|end| association_end_changed(end, &source_end))
            || relationship
                .association_ends
                .get(1)
                .is_some_and(|end| association_end_changed(end, &target_end))
    } else {
        false
    };
    let changed = source_changed
        || target_changed
        || owner_changed
        || association_changed
        || name
            .as_ref()
            .is_some_and(|value| relationship.name != *value)
        || documentation
            .as_ref()
            .is_some_and(|value| relationship.documentation != *value)
        || visibility.is_some_and(|value| relationship.visibility != value)
        || external_changed;
    Ok((
        changed,
        ModelBuildOperation::UpdateRelationshipFields {
            relationship: BuildReference::Existing(relationship.id),
            name,
            owner: Some(owner.reference.clone()),
            source: Some(source.reference.clone()),
            target: Some(target.reference.clone()),
            external_id: (external_changed || external_id_explicit)
                .then(|| effective_external_id.to_string()),
            documentation,
            visibility,
            source_end,
            target_end,
        },
    ))
}

struct RelationshipRowPlan {
    action: SpreadsheetRowAction,
    operations: Vec<ModelBuildOperation>,
}

fn plan_relationship_row(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    project: &Project,
    planned: &[PlannedElement],
    seen_source_external_ids: &mut HashSet<String>,
) -> Result<RelationshipRowPlan, SpreadsheetImportDiagnostic> {
    let kind = relationship_kind_for_row(map, row, values)?;
    let source = resolve_relationship_endpoint(
        map,
        project,
        planned,
        values,
        &kind,
        SpreadsheetSemanticProperty::Source,
        "Source",
    )?;
    let target = resolve_relationship_endpoint(
        map,
        project,
        planned,
        values,
        &kind,
        SpreadsheetSemanticProperty::Target,
        "Target",
    )?;
    if kind == RelationshipKind::Allocate && source.reference == target.reference {
        return Err(diagnostic(
            Some(map),
            Some(row),
            None,
            None,
            non_empty_value(values, SpreadsheetSemanticProperty::ExternalId).map(ToOwned::to_owned),
            "ALLOCATION_SELF_REFERENCE",
            format!(
                "Allocate source '{}' and target '{}' resolve to the same semantic element",
                non_empty_value(values, SpreadsheetSemanticProperty::Source).unwrap_or_default(),
                non_empty_value(values, SpreadsheetSemanticProperty::Target).unwrap_or_default()
            ),
        ));
    }
    let owner = if let Some(owner_text) =
        non_empty_value(values, SpreadsheetSemanticProperty::Owner)
    {
        resolve_owner(map, project, planned, Some(owner_text))?
    } else if is_pr41_traceability_kind(&kind) || kind == RelationshipKind::Allocate {
        let inferred = resolve_owner(map, project, planned, None)?;
        if map.target_scope == project.root_id {
            return Err(diagnostic(
                Some(map),
                Some(row),
                mapped_column_name(map, SpreadsheetSemanticProperty::Owner),
                Some(SpreadsheetSemanticProperty::Owner),
                None,
                "RELATIONSHIP_OWNER_REQUIRED",
                "PR41/PR42 does not infer a loose root owner; map Owner explicitly or configure a package target scope that contains the relationship endpoints",
            ));
        }
        inferred
    } else {
        return Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::Owner),
            Some(SpreadsheetSemanticProperty::Owner),
            None,
            "RELATIONSHIP_OWNER_REQUIRED",
            "PR40 core relationships still require an explicit semantic Owner",
        ));
    };

    let source_end = parse_end_fields(
        map,
        row,
        values,
        SpreadsheetSemanticProperty::SourceEndRole,
        SpreadsheetSemanticProperty::SourceMultiplicity,
        SpreadsheetSemanticProperty::SourceNavigable,
        SpreadsheetSemanticProperty::SourceAggregation,
    )?;
    let target_end = parse_end_fields(
        map,
        row,
        values,
        SpreadsheetSemanticProperty::TargetEndRole,
        SpreadsheetSemanticProperty::TargetMultiplicity,
        SpreadsheetSemanticProperty::TargetNavigable,
        SpreadsheetSemanticProperty::TargetAggregation,
    )?;
    if kind != RelationshipKind::Association {
        let association_value_present = [
            SpreadsheetSemanticProperty::SourceEndRole,
            SpreadsheetSemanticProperty::TargetEndRole,
            SpreadsheetSemanticProperty::SourceMultiplicity,
            SpreadsheetSemanticProperty::TargetMultiplicity,
            SpreadsheetSemanticProperty::SourceNavigable,
            SpreadsheetSemanticProperty::TargetNavigable,
            SpreadsheetSemanticProperty::SourceAggregation,
            SpreadsheetSemanticProperty::TargetAggregation,
        ]
        .into_iter()
        .any(|property| non_empty_value(values, property).is_some());
        if association_value_present {
            return Err(diagnostic(
                Some(map),
                Some(row),
                None,
                None,
                None,
                "ASSOCIATION_FIELD_INVALID",
                format!("Association-end values cannot be applied to {:?}", kind),
            ));
        }
    }

    if let Some(value) = non_empty_value(values, SpreadsheetSemanticProperty::RelationshipKind) {
        let parsed = parse_relationship_kind_value(map, row, value)?;
        if parsed != kind {
            return Err(diagnostic(
                Some(map),
                Some(row),
                mapped_column_name(map, SpreadsheetSemanticProperty::RelationshipKind),
                Some(SpreadsheetSemanticProperty::RelationshipKind),
                Some(value.to_string()),
                "RELATIONSHIP_KIND_MISMATCH",
                format!("mapped kind {:?} does not match row {:?}", kind, parsed),
            ));
        }
    }

    let explicit_external_id = non_empty_value(values, SpreadsheetSemanticProperty::ExternalId);
    if explicit_external_id.is_none()
        && map.relationship_identity == SpreadsheetRelationshipIdentityPolicy::ExternalId
    {
        return Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId),
            None,
            "RELATIONSHIP_EXTERNAL_ID_REQUIRED",
            "relationship External ID is blank and no fallback relationship identity policy was configured",
        ));
    }
    let effective_external_id = explicit_external_id
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            fallback_relationship_external_id(map.relationship_identity, &kind, values)
        });
    let key = external_key(&map.source_namespace, &effective_external_id);
    if !seen_source_external_ids.insert(key.clone()) {
        return Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId),
            Some(effective_external_id),
            "DUPLICATE_SOURCE_EXTERNAL_ID",
            format!(
                "source relationship identity '{key}' appears more than once in this import group"
            ),
        ));
    }

    let existing = if let Some(external_id) = explicit_external_id {
        find_relationship_by_external_id(map, project, external_id, &kind)?
    } else {
        let synthesized =
            find_relationship_by_external_id(map, project, &effective_external_id, &kind)?;
        if synthesized.is_some() {
            synthesized
        } else {
            find_relationship_by_fallback(
                map,
                project,
                &kind,
                &source,
                &target,
                &source_end,
                &target_end,
            )?
        }
    };

    if let Some(existing) = existing {
        let (changed, operation) = relationship_field_changes(
            map,
            row,
            existing,
            &effective_external_id,
            &source,
            &target,
            &owner,
            source_end,
            target_end,
            values,
        )?;
        return Ok(RelationshipRowPlan {
            action: if changed {
                SpreadsheetRowAction::Update
            } else {
                SpreadsheetRowAction::NoChange
            },
            operations: changed.then_some(operation).into_iter().collect(),
        });
    }

    let mut operations = vec![ModelBuildOperation::CreateRelationship {
        external_id: effective_external_id.clone(),
        kind: kind.clone(),
        source: source.reference.clone(),
        target: target.reference.clone(),
        owner: Some(owner.reference.clone()),
    }];
    let name = values.get(&SpreadsheetSemanticProperty::Name).cloned();
    let documentation = values
        .get(&SpreadsheetSemanticProperty::Documentation)
        .cloned();
    let visibility = values
        .get(&SpreadsheetSemanticProperty::Visibility)
        .map(|value| parse_visibility(map, row, value))
        .transpose()?;
    if name.is_some()
        || documentation.is_some()
        || visibility.is_some()
        || source_end.is_some()
        || target_end.is_some()
    {
        operations.push(ModelBuildOperation::UpdateRelationshipFields {
            relationship: RelationshipReference::External(effective_external_id),
            name,
            owner: None,
            source: None,
            target: None,
            external_id: None,
            documentation,
            visibility,
            source_end,
            target_end,
        });
    }
    Ok(RelationshipRowPlan {
        action: SpreadsheetRowAction::Create,
        operations,
    })
}

#[allow(clippy::too_many_arguments)]
fn mapped_field_changes(
    map: &SpreadsheetImportMap,
    row: usize,
    element: &Element,
    owner: &ResolvedOwner,
    type_ref: Option<ElementReference>,
    multiplicity: Option<Multiplicity>,
    flow_direction: Option<FlowDirection>,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<(bool, ModelBuildOperation), SpreadsheetImportDiagnostic> {
    let name = values.get(&SpreadsheetSemanticProperty::Name).cloned();
    let documentation = values
        .get(&SpreadsheetSemanticProperty::Documentation)
        .cloned();
    let external_id = values
        .get(&SpreadsheetSemanticProperty::ExternalId)
        .cloned()
        .filter(|value| !value.is_empty());
    let requirement_id = values
        .get(&SpreadsheetSemanticProperty::RequirementId)
        .cloned();
    let requirement_text = values
        .get(&SpreadsheetSemanticProperty::RequirementText)
        .cloned();
    let visibility = values
        .get(&SpreadsheetSemanticProperty::Visibility)
        .map(|value| parse_visibility(map, row, value))
        .transpose()?;
    let owner_mapped = values.contains_key(&SpreadsheetSemanticProperty::Owner);
    let default_value = values
        .get(&SpreadsheetSemanticProperty::DefaultValue)
        .cloned();
    let owner_changed = owner_mapped
        && match owner.reference {
            BuildReference::Existing(id) => element.owner_id != Some(id),
            BuildReference::External(_) => true,
        };
    let type_changed = match &type_ref {
        Some(BuildReference::Existing(id)) => element.type_id != Some(*id),
        Some(BuildReference::External(_)) => true,
        None => false,
    };
    let default_changed = default_value.as_ref().is_some_and(|value| {
        let normalized = (!value.trim().is_empty()).then_some(value.as_str());
        element.default_value.as_deref() != normalized
    });
    let changed = name.as_ref().is_some_and(|value| element.name != *value)
        || documentation
            .as_ref()
            .is_some_and(|value| element.documentation != *value)
        || external_id
            .as_ref()
            .is_some_and(|value| element.external_id != external_key(&map.source_namespace, value))
        || visibility.is_some_and(|value| element.visibility != value)
        || requirement_id
            .as_ref()
            .is_some_and(|value| element.requirement_id.as_deref() != Some(value.as_str()))
        || requirement_text
            .as_ref()
            .is_some_and(|value| element.requirement_text.as_deref() != Some(value.as_str()))
        || multiplicity.is_some_and(|value| element.multiplicity != Some(value))
        || flow_direction.is_some_and(|value| element.flow_direction != Some(value))
        || default_changed
        || owner_changed
        || type_changed;
    Ok((
        changed,
        ModelBuildOperation::UpdateElementFields {
            element: BuildReference::Existing(element.id),
            name,
            owner: owner_mapped.then(|| owner.reference.clone()),
            type_ref,
            external_id,
            documentation,
            visibility,
            requirement_id,
            requirement_text,
            multiplicity,
            default_value,
            flow_direction,
        },
    ))
}

fn prepare_spreadsheet_import(
    group: &SpreadsheetImportMapGroup,
    project: &Project,
) -> PreparedSpreadsheetImport {
    let mut preview = SpreadsheetImportPreview::default();
    if group.mappings.is_empty() {
        preview.diagnostics.push(diagnostic(
            None,
            None,
            None,
            None,
            None,
            "IMPORT_MAP_GROUP_EMPTY",
            "at least one spreadsheet import map is required",
        ));
        preview.recount();
        return PreparedSpreadsheetImport {
            plan: ModelBuildPlan {
                source_namespace: String::new(),
                operations: Vec::new(),
            },
            preview,
            operation_contexts: Vec::new(),
        };
    }

    let source_namespace = group.mappings[0].source_namespace.trim().to_string();
    if group
        .mappings
        .iter()
        .any(|map| map.source_namespace.trim() != source_namespace)
    {
        preview.diagnostics.push(diagnostic(
            group.mappings.first(),
            None,
            None,
            None,
            None,
            "MIXED_SOURCE_NAMESPACE",
            "an ordered mapping group must use one source namespace so it can become one atomic ModelBuildPlan",
        ));
        preview.recount();
        return PreparedSpreadsheetImport {
            plan: ModelBuildPlan {
                source_namespace,
                operations: Vec::new(),
            },
            preview,
            operation_contexts: Vec::new(),
        };
    }

    let mut operations = Vec::new();
    let mut operation_contexts = Vec::new();
    let mut planned = Vec::<PlannedElement>::new();
    let mut seen_source_external_ids = HashSet::<String>::new();

    for map in &group.mappings {
        if let Err(error) = validate_map(map, project) {
            preview.diagnostics.push(error);
            continue;
        }
        let table = match read_spreadsheet_table(map) {
            Ok(table) => table,
            Err(error) => {
                preview.diagnostics.push(error);
                continue;
            }
        };
        let indexes = match column_indexes(map, &table) {
            Ok(indexes) => indexes,
            Err(error) => {
                preview.diagnostics.push(error);
                continue;
            }
        };

        for row in &table.rows {
            let values = mapped_values(row, &indexes);
            let id_value = identification_value(map, &values);
            let mut block_row = |mut error: SpreadsheetImportDiagnostic| {
                error.row = Some(row.row_number);
                if error.identification_value.is_none() {
                    error.identification_value = id_value.clone();
                }
                preview.diagnostics.push(error);
                preview.rows.push(row_preview(
                    map,
                    row.row_number,
                    &values,
                    SpreadsheetRowAction::Blocked,
                ));
            };

            if is_relationship_map(map) {
                match plan_relationship_row(
                    map,
                    row.row_number,
                    &values,
                    project,
                    &planned,
                    &mut seen_source_external_ids,
                ) {
                    Ok(planned_relationship) => {
                        preview.rows.push(row_preview(
                            map,
                            row.row_number,
                            &values,
                            planned_relationship.action,
                        ));
                        let context = row_context(map, row.row_number, &values);
                        for operation in planned_relationship.operations {
                            operations.push(operation);
                            operation_contexts.push(context.clone());
                        }
                    }
                    Err(mut error) => {
                        error.row = Some(row.row_number);
                        error.relationship_kind =
                            relationship_kind_for_row(map, row.row_number, &values).ok();
                        error.source_endpoint =
                            non_empty_value(&values, SpreadsheetSemanticProperty::Source)
                                .map(ToOwned::to_owned);
                        error.target_endpoint =
                            non_empty_value(&values, SpreadsheetSemanticProperty::Target)
                                .map(ToOwned::to_owned);
                        block_row(error);
                    }
                }
                continue;
            }

            if let Some(external_id) =
                non_empty_value(&values, SpreadsheetSemanticProperty::ExternalId)
            {
                let key = external_key(&map.source_namespace, external_id);
                if !seen_source_external_ids.insert(key.clone()) {
                    block_row(diagnostic(
                        Some(map),
                        Some(row.row_number),
                        mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
                        Some(SpreadsheetSemanticProperty::ExternalId),
                        Some(external_id.into()),
                        "DUPLICATE_SOURCE_EXTERNAL_ID",
                        format!(
                            "source external ID '{key}' appears more than once in this import group"
                        ),
                    ));
                    continue;
                }
            }

            let owner_value = non_empty_value(&values, SpreadsheetSemanticProperty::Owner);
            let owner = match resolve_owner(map, project, &planned, owner_value) {
                Ok(owner) => owner,
                Err(error) => {
                    block_row(error);
                    continue;
                }
            };
            if !is_feature_kind(&map.element_kind) && !is_namespace_kind(&owner.kind) {
                block_row(diagnostic(
                    Some(map),
                    Some(row.row_number),
                    mapped_column_name(map, SpreadsheetSemanticProperty::Owner),
                    Some(SpreadsheetSemanticProperty::Owner),
                    id_value.clone(),
                    "INVALID_OWNERSHIP",
                    format!(
                        "{:?} cannot be owned by {:?} in the PR38 packageable-element scope",
                        map.element_kind, owner.kind
                    ),
                ));
                continue;
            }

            let type_resolution = if is_feature_kind(&map.element_kind) {
                match resolve_type(
                    map,
                    project,
                    &planned,
                    non_empty_value(&values, SpreadsheetSemanticProperty::Type),
                ) {
                    Ok(value) => Some(value),
                    Err(error) => {
                        block_row(error);
                        continue;
                    }
                }
            } else {
                None
            };
            let multiplicity =
                match non_empty_value(&values, SpreadsheetSemanticProperty::Multiplicity) {
                    Some(value) => match super::parametrics::parse_multiplicity(value) {
                        Ok(value) => Some(value),
                        Err(reason) => {
                            block_row(diagnostic(
                                Some(map),
                                Some(row.row_number),
                                mapped_column_name(map, SpreadsheetSemanticProperty::Multiplicity),
                                Some(SpreadsheetSemanticProperty::Multiplicity),
                                id_value.clone(),
                                "MULTIPLICITY_INVALID",
                                format!(
                                    "feature '{}' has invalid multiplicity '{}': {}",
                                    non_empty_value(&values, SpreadsheetSemanticProperty::Name)
                                        .unwrap_or("<unnamed>"),
                                    value,
                                    reason
                                ),
                            ));
                            continue;
                        }
                    },
                    None => None,
                };
            let flow_direction = if map.element_kind == ElementKind::FlowProperty {
                let Some(value) =
                    non_empty_value(&values, SpreadsheetSemanticProperty::FlowDirection)
                else {
                    block_row(diagnostic(
                        Some(map),
                        Some(row.row_number),
                        mapped_column_name(map, SpreadsheetSemanticProperty::FlowDirection),
                        Some(SpreadsheetSemanticProperty::FlowDirection),
                        id_value.clone(),
                        "FLOW_DIRECTION_INVALID",
                        "FlowProperty direction is blank; expected in, out, or inout",
                    ));
                    continue;
                };
                match super::feature_editing::parse_flow_direction(
                    &value.trim().to_ascii_lowercase(),
                ) {
                    Ok(value) => Some(value),
                    Err(_) => {
                        block_row(diagnostic(
                            Some(map),
                            Some(row.row_number),
                            mapped_column_name(map, SpreadsheetSemanticProperty::FlowDirection),
                            Some(SpreadsheetSemanticProperty::FlowDirection),
                            id_value.clone(),
                            "FLOW_DIRECTION_INVALID",
                            format!(
                                "FlowProperty '{}' direction '{}' is invalid; expected in, out, or inout",
                                non_empty_value(&values, SpreadsheetSemanticProperty::Name)
                                    .unwrap_or("<unnamed>"),
                                value
                            ),
                        ));
                        continue;
                    }
                }
            } else {
                None
            };

            let existing = match find_existing(map, project, &values) {
                Ok(existing) => existing,
                Err(error) => {
                    block_row(error);
                    continue;
                }
            };

            if let Some(existing) = existing {
                match mapped_field_changes(
                    map,
                    row.row_number,
                    existing,
                    &owner,
                    type_resolution
                        .as_ref()
                        .map(|value| value.reference.clone()),
                    multiplicity,
                    flow_direction,
                    &values,
                ) {
                    Ok((false, _)) => preview.rows.push(row_preview(
                        map,
                        row.row_number,
                        &values,
                        SpreadsheetRowAction::NoChange,
                    )),
                    Ok((true, operation)) => {
                        preview.rows.push(row_preview(
                            map,
                            row.row_number,
                            &values,
                            SpreadsheetRowAction::Update,
                        ));
                        operations.push(operation);
                        operation_contexts.push(row_context(map, row.row_number, &values));
                    }
                    Err(mut error) => {
                        error.row = Some(row.row_number);
                        block_row(error);
                    }
                }
                continue;
            }

            let Some(external_id) =
                non_empty_value(&values, SpreadsheetSemanticProperty::ExternalId)
            else {
                block_row(diagnostic(
                    Some(map),
                    Some(row.row_number),
                    mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
                    Some(SpreadsheetSemanticProperty::ExternalId),
                    id_value.clone(),
                    "CREATE_REQUIRES_EXTERNAL_ID",
                    "new spreadsheet-created elements require a mapped External ID so PR36 can provide stable namespaced identity",
                ));
                continue;
            };
            let Some(name) = non_empty_value(&values, SpreadsheetSemanticProperty::Name) else {
                block_row(diagnostic(
                    Some(map),
                    Some(row.row_number),
                    mapped_column_name(map, SpreadsheetSemanticProperty::Name),
                    Some(SpreadsheetSemanticProperty::Name),
                    id_value.clone(),
                    "CREATE_REQUIRES_NAME",
                    "new semantic elements require a non-empty Name",
                ));
                continue;
            };
            if map.element_kind == ElementKind::Requirement
                && (non_empty_value(&values, SpreadsheetSemanticProperty::RequirementId).is_none()
                    || non_empty_value(&values, SpreadsheetSemanticProperty::RequirementText)
                        .is_none())
            {
                block_row(diagnostic(
                    Some(map),
                    Some(row.row_number),
                    None,
                    None,
                    id_value.clone(),
                    "REQUIREMENT_FIELDS_REQUIRED",
                    "new Requirement rows require mapped, non-empty Requirement ID and Requirement Text",
                ));
                continue;
            }

            let context = row_context(map, row.row_number, &values);
            operations.push(ModelBuildOperation::CreateElement {
                external_id: external_id.to_string(),
                kind: map.element_kind.clone(),
                name: name.to_string(),
                owner: owner.reference.clone(),
                type_ref: type_resolution
                    .as_ref()
                    .map(|value| value.reference.clone()),
            });
            operation_contexts.push(context.clone());

            let documentation = values
                .get(&SpreadsheetSemanticProperty::Documentation)
                .cloned();
            let visibility = match values.get(&SpreadsheetSemanticProperty::Visibility) {
                Some(value) => match parse_visibility(map, row.row_number, value) {
                    Ok(value) => Some(value),
                    Err(error) => {
                        operations.pop();
                        operation_contexts.pop();
                        block_row(error);
                        continue;
                    }
                },
                None => None,
            };
            let requirement_id = values
                .get(&SpreadsheetSemanticProperty::RequirementId)
                .cloned();
            let requirement_text = values
                .get(&SpreadsheetSemanticProperty::RequirementText)
                .cloned();
            let default_value = values
                .get(&SpreadsheetSemanticProperty::DefaultValue)
                .cloned();
            if documentation.is_some()
                || visibility.is_some()
                || requirement_id.is_some()
                || requirement_text.is_some()
                || multiplicity.is_some()
                || default_value.is_some()
                || flow_direction.is_some()
            {
                operations.push(ModelBuildOperation::UpdateElementFields {
                    element: BuildReference::External(external_id.to_string()),
                    name: None,
                    owner: None,
                    type_ref: None,
                    external_id: None,
                    documentation,
                    visibility,
                    requirement_id,
                    requirement_text,
                    multiplicity,
                    default_value,
                    flow_direction,
                });
                operation_contexts.push(context);
            }

            let qualified_name = format!("{}::{name}", owner.qualified_name);
            planned.push(PlannedElement {
                external_id: external_id.to_string(),
                kind: map.element_kind.clone(),
                qualified_name,
                requirement_id: (map.element_kind == ElementKind::Requirement)
                    .then(|| {
                        non_empty_value(&values, SpreadsheetSemanticProperty::RequirementId)
                            .map(ToOwned::to_owned)
                    })
                    .flatten(),
                depth_from_target: owner.depth_from_target + 1,
            });
            preview.rows.push(row_preview(
                map,
                row.row_number,
                &values,
                SpreadsheetRowAction::Create,
            ));
        }
    }

    preview.recount();
    PreparedSpreadsheetImport {
        plan: ModelBuildPlan {
            source_namespace,
            operations,
        },
        preview,
        operation_contexts,
    }
}

fn convert_build_diagnostic(
    diagnostic: &BuildDiagnostic,
    contexts: &[RowContext],
) -> SpreadsheetImportDiagnostic {
    let context = diagnostic
        .operation
        .and_then(|operation| contexts.get(operation));
    SpreadsheetImportDiagnostic {
        severity: match diagnostic.severity {
            BuildDiagnosticSeverity::Error => SpreadsheetDiagnosticSeverity::Error,
            BuildDiagnosticSeverity::Warning => SpreadsheetDiagnosticSeverity::Warning,
        },
        code: diagnostic.code.into(),
        source: context.map(|context| context.source.clone()),
        worksheet: context.and_then(|context| context.worksheet.clone()),
        row: context.map(|context| context.row),
        column: None,
        import_map: context.map(|context| context.import_map.clone()),
        element_kind: context.map(|context| context.element_kind.clone()),
        relationship_kind: context.and_then(|context| context.relationship_kind.clone()),
        source_endpoint: context.and_then(|context| context.source_endpoint.clone()),
        target_endpoint: context.and_then(|context| context.target_endpoint.clone()),
        source_namespace: context.map(|context| context.source_namespace.clone()),
        identification_value: context.and_then(|context| context.identification_value.clone()),
        semantic_property: None,
        reason: diagnostic.message.clone(),
    }
}

fn attach_build_diagnostics(
    preview: &mut SpreadsheetImportPreview,
    diagnostics: &[BuildDiagnostic],
    contexts: &[RowContext],
) {
    for build_diagnostic in diagnostics {
        let spreadsheet_diagnostic = convert_build_diagnostic(build_diagnostic, contexts);
        if let Some(row) = spreadsheet_diagnostic.row
            && let Some(row_preview) = preview.rows.iter_mut().find(|candidate| {
                candidate.row == row
                    && spreadsheet_diagnostic
                        .import_map
                        .as_deref()
                        .is_some_and(|map| candidate.import_map == map)
            })
        {
            row_preview.action = match spreadsheet_diagnostic.severity {
                SpreadsheetDiagnosticSeverity::Error => SpreadsheetRowAction::Blocked,
                SpreadsheetDiagnosticSeverity::Warning => SpreadsheetRowAction::Warning,
            };
        }
        preview.diagnostics.push(spreadsheet_diagnostic);
    }
    preview.recount();
}

fn prepare_from_state(
    group: &SpreadsheetImportMapGroup,
    state: &WorkspaceState,
) -> Result<PreparedSpreadsheetImport, SpreadsheetImportPreview> {
    let project = state
        .project
        .lock()
        .map_err(|_| {
            let mut preview = SpreadsheetImportPreview::default();
            preview.diagnostics.push(diagnostic(
                group.mappings.first(),
                None,
                None,
                None,
                None,
                "LOCK_FAILURE",
                "project lock poisoned",
            ));
            preview.recount();
            preview
        })?
        .clone()
        .ok_or_else(|| {
            let mut preview = SpreadsheetImportPreview::default();
            preview.diagnostics.push(diagnostic(
                group.mappings.first(),
                None,
                None,
                None,
                None,
                "NO_PROJECT",
                "no project open",
            ));
            preview.recount();
            preview
        })?;
    Ok(prepare_spreadsheet_import(group, &project))
}

pub fn preview_spreadsheet_import_group(
    group: &SpreadsheetImportMapGroup,
    state: &WorkspaceState,
) -> SpreadsheetImportPreview {
    let mut prepared = match prepare_from_state(group, state) {
        Ok(prepared) => prepared,
        Err(preview) => return preview,
    };
    if !prepared.preview.is_valid() {
        return prepared.preview;
    }
    let build_preview = preview_model_build(&prepared.plan, state);
    attach_build_diagnostics(
        &mut prepared.preview,
        &build_preview.diagnostics,
        &prepared.operation_contexts,
    );
    prepared.preview
}

fn apply_spreadsheet_import_with_preview(
    group: &SpreadsheetImportMapGroup,
    state: &WorkspaceState,
) -> (Option<ModelBuildResult>, SpreadsheetImportPreview) {
    let mut prepared = match prepare_from_state(group, state) {
        Ok(prepared) => prepared,
        Err(preview) => return (None, preview),
    };
    if !prepared.preview.is_valid() {
        return (None, prepared.preview);
    }
    let build_preview = preview_model_build(&prepared.plan, state);
    attach_build_diagnostics(
        &mut prepared.preview,
        &build_preview.diagnostics,
        &prepared.operation_contexts,
    );
    if !prepared.preview.is_valid() {
        return (None, prepared.preview);
    }
    match apply_model_build(&prepared.plan, state) {
        Ok(result) => {
            prepared.preview.applied = true;
            (Some(result), prepared.preview)
        }
        Err(build_preview) => {
            attach_build_diagnostics(
                &mut prepared.preview,
                &build_preview.diagnostics,
                &prepared.operation_contexts,
            );
            (None, prepared.preview)
        }
    }
}

#[cfg(test)]
pub fn apply_spreadsheet_import_group(
    group: &SpreadsheetImportMapGroup,
    state: &WorkspaceState,
) -> Result<ModelBuildResult, SpreadsheetImportPreview> {
    let (result, preview) = apply_spreadsheet_import_with_preview(group, state);
    result.ok_or(preview)
}

#[tauri::command]
pub fn preview_spreadsheet_import(
    group: SpreadsheetImportMapGroup,
    workspace: tauri::State<'_, WorkspaceState>,
) -> SpreadsheetImportPreview {
    preview_spreadsheet_import_group(&group, &workspace)
}

#[tauri::command]
pub fn apply_spreadsheet_import(
    group: SpreadsheetImportMapGroup,
    workspace: tauri::State<'_, WorkspaceState>,
) -> SpreadsheetImportPreview {
    apply_spreadsheet_import_with_preview(&group, &workspace).1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn workspace(name: &str) -> (WorkspaceState, ElementId) {
        let state = WorkspaceState::default();
        let project = Project::new(name);
        let root = project.root_id;
        *state.project.lock().unwrap() = Some(project);
        (state, root)
    }

    fn fixture_path() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pr38_catia_style.xlsx")
            .to_string_lossy()
            .into_owned()
    }

    fn temp_csv(contents: &str) -> String {
        let path = std::env::temp_dir().join(format!("pr38-{}.csv", uuid::Uuid::new_v4()));
        fs::write(&path, contents).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[allow(clippy::too_many_arguments)]
    fn map(
        name: &str,
        source: String,
        worksheet: Option<&str>,
        header_row: usize,
        kind: ElementKind,
        target: ElementId,
        identification_property: SpreadsheetIdentificationProperty,
        search_scope: SpreadsheetSearchScope,
        columns: &[(&str, SpreadsheetSemanticProperty)],
    ) -> SpreadsheetImportMap {
        SpreadsheetImportMap {
            name: name.into(),
            source,
            worksheet: worksheet.map(ToOwned::to_owned),
            header_row,
            element_kind: kind,
            relationship_kind: None,
            relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,
            target_scope: target,
            identification_property,
            search_scope,
            source_namespace: "catia:pr38-fixture".into(),
            mapping_version: "1".into(),
            column_mappings: columns
                .iter()
                .map(|(source_column, property)| SpreadsheetColumnMapping {
                    source_column: (*source_column).into(),
                    property: *property,
                })
                .collect(),
        }
    }

    #[test]
    fn xlsx_package_import_supports_arbitrary_sheet_columns_and_header_row() {
        let (state, root) = workspace("PR38 XLSX");
        let mapping = map(
            "Packages",
            fixture_path(),
            Some("Package Definition"),
            3,
            ElementKind::Package,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                (
                    "Package Identifier",
                    SpreadsheetSemanticProperty::ExternalId,
                ),
                ("Package Name", SpreadsheetSemanticProperty::Name),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![mapping],
        };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 3);
        assert!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len()
                == 1
        );
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        assert!(project.elements.values().any(|element| {
            element.external_id == "catia:pr38-fixture::PKG-STRUCT" && element.name == "Structure"
        }));
    }

    #[test]
    fn ordered_xlsx_maps_allow_basic_elements_to_resolve_packages_created_first() {
        let (state, root) = workspace("PR38 Ordered");
        let packages = map(
            "Packages",
            fixture_path(),
            Some("Package Definition"),
            3,
            ElementKind::Package,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                (
                    "Package Identifier",
                    SpreadsheetSemanticProperty::ExternalId,
                ),
                ("Package Name", SpreadsheetSemanticProperty::Name),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let architecture = map(
            "Architecture Blocks",
            fixture_path(),
            Some("Architecture"),
            2,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("Element ID", SpreadsheetSemanticProperty::ExternalId),
                ("Component Name", SpreadsheetSemanticProperty::Name),
                ("Description", SpreadsheetSemanticProperty::Documentation),
                ("Parent", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![packages, architecture],
        };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 6);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        let engine = project
            .elements
            .values()
            .find(|element| element.external_id == "catia:pr38-fixture::BLK-ENG")
            .unwrap();
        assert_eq!(engine.documentation, "Power unit");
        assert_eq!(
            project.qualified_name(engine.id).unwrap(),
            "PR38 Ordered::Vehicle::Structure::Engine"
        );
    }

    #[test]
    fn csv_basic_element_import_uses_same_mapping_pipeline() {
        let (state, root) = workspace("PR38 CSV");
        let source = temp_csv(
            "Element ID,Component Name,Description,Parent\nBLK-1,Controller,Controller docs,\n",
        );
        let mapping = map(
            "CSV Blocks",
            source,
            None,
            1,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetOnly,
            &[
                ("Element ID", SpreadsheetSemanticProperty::ExternalId),
                ("Component Name", SpreadsheetSemanticProperty::Name),
                ("Description", SpreadsheetSemanticProperty::Documentation),
                ("Parent", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![mapping],
        };
        assert_eq!(
            preview_spreadsheet_import_group(&group, &state)
                .totals
                .create,
            1
        );
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let project = state.project.lock().unwrap();
        let controller = project
            .as_ref()
            .unwrap()
            .elements
            .values()
            .find(|element| element.name == "Controller")
            .unwrap();
        assert_eq!(controller.documentation, "Controller docs");
    }

    #[test]
    fn target_only_and_recursive_search_are_semantically_distinct() {
        let (state, root) = workspace("PR38 Scope");
        let nested;
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let package = project
                .create_element(ElementKind::Package, "Nested", root)
                .unwrap();
            nested = project
                .create_element(ElementKind::Block, "Controller", package)
                .unwrap();
        }
        let source = temp_csv("Element ID,Component Name,Description\nNEW-ID,Controller,Updated\n");
        let columns = [
            ("Element ID", SpreadsheetSemanticProperty::ExternalId),
            ("Component Name", SpreadsheetSemanticProperty::Name),
            ("Description", SpreadsheetSemanticProperty::Documentation),
        ];
        let target_only = map(
            "Target Only",
            source.clone(),
            None,
            1,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::Name,
            SpreadsheetSearchScope::TargetOnly,
            &columns,
        );
        let recursive = map(
            "Recursive",
            source,
            None,
            1,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::Name,
            SpreadsheetSearchScope::TargetRecursive,
            &columns,
        );
        assert_eq!(
            preview_spreadsheet_import_group(
                &SpreadsheetImportMapGroup {
                    mappings: vec![target_only]
                },
                &state,
            )
            .totals
            .create,
            1
        );
        assert_eq!(
            preview_spreadsheet_import_group(
                &SpreadsheetImportMapGroup {
                    mappings: vec![recursive]
                },
                &state,
            )
            .totals
            .update,
            1
        );
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .element(nested)
                .unwrap()
                .documentation,
            ""
        );
    }

    #[test]
    fn external_name_and_requirement_id_identification_classify_updates_and_no_change() {
        let (state, root) = workspace("PR38 Identity");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let block = project
                .create_element(ElementKind::Block, "Engine", root)
                .unwrap();
            project
                .set_external_id(block, "catia:pr38-fixture::BLK-ENGINE")
                .unwrap();
            project.element_mut(block).unwrap().documentation = "Old".into();
            let requirement = project
                .create_requirement("Safety", "REQ-7", "Old text", root)
                .unwrap();
            project
                .set_external_id(requirement, "legacy::REQ-7")
                .unwrap();
        }
        let ext_source = temp_csv("ID,Name,Description\nBLK-ENGINE,Engine,New\n");
        let ext_map = map(
            "External",
            ext_source,
            None,
            1,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetOnly,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Description", SpreadsheetSemanticProperty::Documentation),
            ],
        );
        let req_source = temp_csv(
            "New External,Requirement Key,Requirement Text,Name\nREQ-NEW,REQ-7,Updated text,Safety\n",
        );
        let req_map = map(
            "Requirement ID",
            req_source,
            None,
            1,
            ElementKind::Requirement,
            root,
            SpreadsheetIdentificationProperty::RequirementId,
            SpreadsheetSearchScope::TargetOnly,
            &[
                ("New External", SpreadsheetSemanticProperty::ExternalId),
                (
                    "Requirement Key",
                    SpreadsheetSemanticProperty::RequirementId,
                ),
                (
                    "Requirement Text",
                    SpreadsheetSemanticProperty::RequirementText,
                ),
                ("Name", SpreadsheetSemanticProperty::Name),
            ],
        );
        let preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![ext_map, req_map],
            },
            &state,
        );
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.update, 2);

        let no_change_source = temp_csv("ID,Name,Description\nBLK-ENGINE,Engine,Old\n");
        let no_change = map(
            "No change",
            no_change_source,
            None,
            1,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetOnly,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Description", SpreadsheetSemanticProperty::Documentation),
            ],
        );
        assert_eq!(
            preview_spreadsheet_import_group(
                &SpreadsheetImportMapGroup {
                    mappings: vec![no_change]
                },
                &state,
            )
            .totals
            .no_change,
            1
        );
    }

    #[test]
    fn ambiguous_name_is_blocked_instead_of_guessing() {
        let (state, root) = workspace("PR38 Ambiguous");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project
                .create_element(ElementKind::Block, "Controller", root)
                .unwrap();
            project
                .create_element(ElementKind::Block, "Controller", root)
                .unwrap();
        }
        let source = temp_csv("ID,Name\nNEW-ID,Controller\n");
        let mapping = map(
            "Ambiguous",
            source,
            None,
            1,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::Name,
            SpreadsheetSearchScope::TargetOnly,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
            ],
        );
        let preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![mapping],
            },
            &state,
        );
        assert_eq!(preview.totals.blocked, 1);
        assert!(
            preview
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "AMBIGUOUS_IDENTIFICATION")
        );
    }

    #[test]
    fn unresolved_owner_invalid_owner_missing_column_and_duplicate_external_id_are_blocked() {
        let (state, root) = workspace("PR38 Blocking");
        {
            let mut guard = state.project.lock().unwrap();
            guard
                .as_mut()
                .unwrap()
                .create_element(ElementKind::Block, "NotANamespace", root)
                .unwrap();
        }
        let unresolved_source = temp_csv("ID,Name,Parent\nA,A,Missing::Package\n");
        let unresolved = map(
            "Unresolved owner",
            unresolved_source,
            None,
            1,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Parent", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let invalid_source = temp_csv("ID,Name,Parent\nB,B,NotANamespace\n");
        let invalid = map(
            "Invalid owner",
            invalid_source,
            None,
            1,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Parent", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let missing_source = temp_csv("ID,Name\nC,C\n");
        let missing = map(
            "Missing column",
            missing_source,
            None,
            1,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetOnly,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Component Name", SpreadsheetSemanticProperty::Name),
            ],
        );
        let duplicate_source = temp_csv("ID,Name\nD,D1\nD,D2\n");
        let duplicate = map(
            "Duplicate external",
            duplicate_source,
            None,
            1,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetOnly,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
            ],
        );
        let preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![unresolved, invalid, missing, duplicate],
            },
            &state,
        );
        assert!(!preview.is_valid());
        for code in [
            "OWNER_UNRESOLVED",
            "INVALID_OWNERSHIP",
            "MAPPED_COLUMN_MISSING",
            "DUPLICATE_SOURCE_EXTERNAL_ID",
        ] {
            assert!(
                preview
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code),
                "missing {code}: {:?}",
                preview.diagnostics
            );
        }
    }

    #[test]
    fn unresolved_target_blocks_and_never_falls_back_to_root() {
        let (state, _root) = workspace("PR38 Target");
        let source = temp_csv("ID,Name\nA,A\n");
        let mapping = map(
            "Missing target",
            source,
            None,
            1,
            ElementKind::Block,
            ElementId::new(),
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetOnly,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
            ],
        );
        let preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![mapping],
            },
            &state,
        );
        assert!(!preview.is_valid());
        assert!(
            preview
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "TARGET_SCOPE_UNRESOLVED")
        );
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            1
        );
    }

    #[test]
    fn preview_and_blocked_apply_perform_zero_mutation() {
        let (state, root) = workspace("PR38 No Mutation");
        let valid_source = temp_csv("ID,Name\nA,A\n");
        let valid = map(
            "Preview",
            valid_source,
            None,
            1,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetOnly,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
            ],
        );
        let before = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len();
        let preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![valid],
            },
            &state,
        );
        assert!(preview.is_valid());
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            before
        );

        let blocked_source = temp_csv("ID,Name,Parent\nB,B,Missing\n");
        let blocked = map(
            "Blocked",
            blocked_source,
            None,
            1,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Parent", SpreadsheetSemanticProperty::Owner),
            ],
        );
        assert!(
            apply_spreadsheet_import_group(
                &SpreadsheetImportMapGroup {
                    mappings: vec![blocked]
                },
                &state,
            )
            .is_err()
        );
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            before
        );
    }

    #[test]
    fn spreadsheet_pipeline_emits_model_build_plan_and_pr36_rolls_back_atomically() {
        let (state, root) = workspace("PR38 Atomic");
        let source = temp_csv(
            "External,Name,Requirement ID,Requirement Text\nR1,First,REQ-DUP,First text\nR2,Second,REQ-DUP,Second text\n",
        );
        let mapping = map(
            "Requirements",
            source,
            None,
            1,
            ElementKind::Requirement,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetOnly,
            &[
                ("External", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Requirement ID", SpreadsheetSemanticProperty::RequirementId),
                (
                    "Requirement Text",
                    SpreadsheetSemanticProperty::RequirementText,
                ),
            ],
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![mapping],
        };
        let project = state.project.lock().unwrap().as_ref().unwrap().clone();
        let prepared = prepare_spreadsheet_import(&group, &project);
        assert!(prepared.preview.is_valid());
        assert!(prepared.plan.operations.iter().all(|operation| matches!(
            operation,
            ModelBuildOperation::CreateElement { .. }
                | ModelBuildOperation::UpdateElementFields { .. }
        )));
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            1
        );
        assert!(apply_model_build(&prepared.plan, &state).is_err());
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            1
        );
    }
    fn pr39_fixture_path() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pr39_owned_features.xlsx")
            .to_string_lossy()
            .into_owned()
    }

    fn set_pr39_external(project: &mut Project, id: ElementId, external_id: &str) {
        project
            .set_external_id(id, external_key("catia:pr38-fixture", external_id))
            .unwrap();
    }

    #[test]
    fn pr39_xlsx_business_columns_create_typed_owned_features() {
        let (state, root) = workspace("PR39 XLSX");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project
                .create_element(ElementKind::Block, "Vehicle", root)
                .unwrap();
            project
                .create_element(ElementKind::InterfaceBlock, "VehicleInterface", root)
                .unwrap();
            project
                .create_element(ElementKind::Block, "Engine", root)
                .unwrap();
            project
                .create_element(ElementKind::Block, "Controller", root)
                .unwrap();
            project
                .create_element(ElementKind::ValueType, "Mass", root)
                .unwrap();
            project
                .create_element(ElementKind::DataType, "Command", root)
                .unwrap();
        }
        let parts = map(
            "Part Properties",
            pr39_fixture_path(),
            Some("Component Parts"),
            1,
            ElementKind::PartProperty,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                (
                    "Feature Identifier",
                    SpreadsheetSemanticProperty::ExternalId,
                ),
                ("Owning Component", SpreadsheetSemanticProperty::Owner),
                ("Property Name", SpreadsheetSemanticProperty::Name),
                ("Classifier", SpreadsheetSemanticProperty::Type),
                ("Cardinality", SpreadsheetSemanticProperty::Multiplicity),
                ("Description", SpreadsheetSemanticProperty::Documentation),
            ],
        );
        let values = map(
            "Value Properties",
            pr39_fixture_path(),
            Some("Component Values"),
            1,
            ElementKind::ValueProperty,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                (
                    "Feature Identifier",
                    SpreadsheetSemanticProperty::ExternalId,
                ),
                ("Owning Component", SpreadsheetSemanticProperty::Owner),
                ("Property Name", SpreadsheetSemanticProperty::Name),
                ("Classifier", SpreadsheetSemanticProperty::Type),
                ("Cardinality", SpreadsheetSemanticProperty::Multiplicity),
                ("Initial Value", SpreadsheetSemanticProperty::DefaultValue),
                ("Description", SpreadsheetSemanticProperty::Documentation),
            ],
        );
        let flows = map(
            "Flow Properties",
            pr39_fixture_path(),
            Some("Interface Flows"),
            1,
            ElementKind::FlowProperty,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                (
                    "Feature Identifier",
                    SpreadsheetSemanticProperty::ExternalId,
                ),
                ("Owning Component", SpreadsheetSemanticProperty::Owner),
                ("Property Name", SpreadsheetSemanticProperty::Name),
                ("Classifier", SpreadsheetSemanticProperty::Type),
                ("Cardinality", SpreadsheetSemanticProperty::Multiplicity),
                ("Flow", SpreadsheetSemanticProperty::FlowDirection),
                ("Description", SpreadsheetSemanticProperty::Documentation),
            ],
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![parts, values, flows],
        };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 5);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        let engine = project
            .elements
            .values()
            .find(|element| element.name == "engine")
            .unwrap();
        assert_eq!(engine.kind, ElementKind::PartProperty);
        assert_eq!(engine.multiplicity.unwrap().notation(), "1");
        assert_eq!(
            project.element(engine.type_id.unwrap()).unwrap().name,
            "Engine"
        );
        let backup = project
            .elements
            .values()
            .find(|element| element.name == "backupController")
            .unwrap();
        assert_eq!(backup.multiplicity.unwrap().notation(), "0..1");
        let mass = project
            .elements
            .values()
            .find(|element| element.name == "mass")
            .unwrap();
        assert_eq!(mass.default_value.as_deref(), Some("1500"));
        let command = project
            .elements
            .values()
            .find(|element| element.name == "command")
            .unwrap();
        assert_eq!(command.flow_direction, Some(FlowDirection::In));
    }

    #[test]
    fn pr39_supports_all_six_feature_kinds() {
        let (state, root) = workspace("PR39 Six Kinds");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project
                .create_element(ElementKind::Block, "Vehicle", root)
                .unwrap();
            project
                .create_element(ElementKind::InterfaceBlock, "Iface", root)
                .unwrap();
            project
                .create_element(ElementKind::ConstraintBlock, "Equation", root)
                .unwrap();
            project
                .create_element(ElementKind::Block, "Engine", root)
                .unwrap();
            project
                .create_element(ElementKind::ValueType, "Scalar", root)
                .unwrap();
        }
        let cases = [
            (ElementKind::PartProperty, "Vehicle", "Engine", "part", None),
            (
                ElementKind::ReferenceProperty,
                "Vehicle",
                "Engine",
                "reference",
                None,
            ),
            (
                ElementKind::ValueProperty,
                "Vehicle",
                "Scalar",
                "value",
                None,
            ),
            (
                ElementKind::FlowProperty,
                "Iface",
                "Scalar",
                "flow",
                Some("out"),
            ),
            (
                ElementKind::ConstraintProperty,
                "Vehicle",
                "Equation",
                "constraint",
                None,
            ),
            (
                ElementKind::ConstraintParameter,
                "Equation",
                "Scalar",
                "parameter",
                None,
            ),
        ];
        let mut mappings = Vec::new();
        for (index, (kind, owner, type_name, name, direction)) in cases.into_iter().enumerate() {
            let id = format!("F-{index}");
            let source = if let Some(direction) = direction {
                temp_csv(&format!(
                    "ID,Owner,Name,Type,Multiplicity,Flow\n{id},{owner},{name},{type_name},1,{direction}\n"
                ))
            } else {
                temp_csv(&format!(
                    "ID,Owner,Name,Type,Multiplicity\n{id},{owner},{name},{type_name},1\n"
                ))
            };
            let mut columns = vec![
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Owner", SpreadsheetSemanticProperty::Owner),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Type", SpreadsheetSemanticProperty::Type),
                ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
            ];
            if direction.is_some() {
                columns.push(("Flow", SpreadsheetSemanticProperty::FlowDirection));
            }
            mappings.push(map(
                &format!("Feature {index}"),
                source,
                None,
                1,
                kind,
                root,
                SpreadsheetIdentificationProperty::ExternalId,
                SpreadsheetSearchScope::TargetRecursive,
                &columns,
            ));
        }
        let group = SpreadsheetImportMapGroup { mappings };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        for expected in [
            ElementKind::PartProperty,
            ElementKind::ReferenceProperty,
            ElementKind::ValueProperty,
            ElementKind::FlowProperty,
            ElementKind::ConstraintProperty,
            ElementKind::ConstraintParameter,
        ] {
            assert!(
                project
                    .elements
                    .values()
                    .any(|element| element.kind == expected)
            );
        }
    }

    #[test]
    fn pr39_resolves_owner_and_type_by_namespaced_external_identity() {
        let (state, root) = workspace("PR39 External References");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let owner = project
                .create_element(ElementKind::Block, "Renamed Vehicle", root)
                .unwrap();
            let ty = project
                .create_element(ElementKind::Block, "Renamed Engine", root)
                .unwrap();
            set_pr39_external(project, owner, "BLK-VEHICLE");
            set_pr39_external(project, ty, "BLK-ENGINE");
        }
        let source =
            temp_csv("ID,Owner,Name,Type,Multiplicity\nPART-1,BLK-VEHICLE,engine,BLK-ENGINE,1\n");
        let mapping = map(
            "External owner/type",
            source,
            None,
            1,
            ElementKind::PartProperty,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Owner", SpreadsheetSemanticProperty::Owner),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Type", SpreadsheetSemanticProperty::Type),
                ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
            ],
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![mapping],
        };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        let feature = project
            .elements
            .values()
            .find(|element| element.name == "engine")
            .unwrap();
        assert_eq!(
            project.element(feature.owner_id.unwrap()).unwrap().name,
            "Renamed Vehicle"
        );
        assert_eq!(
            project.element(feature.type_id.unwrap()).unwrap().name,
            "Renamed Engine"
        );
    }

    #[test]
    fn pr39_ordered_maps_resolve_plan_local_owner_and_type_without_early_commit() {
        let (state, root) = workspace("PR39 Ordered");
        let blocks = map(
            "Blocks",
            temp_csv("ID,Name\nBLK-VEHICLE,Vehicle\nBLK-ENGINE,Engine\n"),
            None,
            1,
            ElementKind::Block,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetOnly,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
            ],
        );
        let features = map(
            "Features",
            temp_csv("ID,Owner,Name,Type,Multiplicity\nPART-1,BLK-VEHICLE,engine,BLK-ENGINE,1\n"),
            None,
            1,
            ElementKind::PartProperty,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetOnly,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Owner", SpreadsheetSemanticProperty::Owner),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Type", SpreadsheetSemanticProperty::Type),
                ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
            ],
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![blocks, features],
        };
        let before = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len();
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            before
        );
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        let feature = project
            .elements
            .values()
            .find(|element| element.name == "engine")
            .unwrap();
        assert_eq!(
            project.element(feature.owner_id.unwrap()).unwrap().name,
            "Vehicle"
        );
        assert_eq!(
            project.element(feature.type_id.unwrap()).unwrap().name,
            "Engine"
        );
    }

    #[test]
    fn pr39_reimport_updates_same_feature_and_then_noops() {
        let (state, root) = workspace("PR39 Reimport");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project
                .create_element(ElementKind::Block, "Vehicle", root)
                .unwrap();
            project
                .create_element(ElementKind::Block, "Engine", root)
                .unwrap();
        }
        let mapping_for = |source: String| {
            map(
                "Part",
                source,
                None,
                1,
                ElementKind::PartProperty,
                root,
                SpreadsheetIdentificationProperty::ExternalId,
                SpreadsheetSearchScope::TargetRecursive,
                &[
                    ("ID", SpreadsheetSemanticProperty::ExternalId),
                    ("Owner", SpreadsheetSemanticProperty::Owner),
                    ("Name", SpreadsheetSemanticProperty::Name),
                    ("Type", SpreadsheetSemanticProperty::Type),
                    ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
                ],
            )
        };
        let first = SpreadsheetImportMapGroup {
            mappings: vec![mapping_for(temp_csv(
                "ID,Owner,Name,Type,Multiplicity\nPART-1,Vehicle,engine,Engine,1\n",
            ))],
        };
        apply_spreadsheet_import_group(&first, &state).unwrap();
        let update = SpreadsheetImportMapGroup {
            mappings: vec![mapping_for(temp_csv(
                "ID,Owner,Name,Type,Multiplicity\nPART-1,Vehicle,propulsionUnit,Engine,0..1\n",
            ))],
        };
        assert_eq!(
            preview_spreadsheet_import_group(&update, &state)
                .totals
                .update,
            1
        );
        apply_spreadsheet_import_group(&update, &state).unwrap();
        let noop = SpreadsheetImportMapGroup {
            mappings: vec![mapping_for(temp_csv(
                "ID,Owner,Name,Type,Multiplicity\nPART-1,Vehicle,propulsionUnit,Engine,0..1\n",
            ))],
        };
        assert_eq!(
            preview_spreadsheet_import_group(&noop, &state)
                .totals
                .no_change,
            1
        );
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(
            project
                .elements
                .values()
                .filter(|element| element.kind == ElementKind::PartProperty)
                .count(),
            1
        );
    }

    #[test]
    fn pr39_blocks_ambiguous_unresolved_illegal_multiplicity_and_flow_errors() {
        let (state, root) = workspace("PR39 Blocking");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project
                .create_element(ElementKind::Block, "Vehicle", root)
                .unwrap();
            project
                .create_element(ElementKind::Block, "Vehicle", root)
                .unwrap();
            project
                .create_element(ElementKind::ValueType, "Scalar", root)
                .unwrap();
            project
                .create_element(ElementKind::ValueType, "Scalar", root)
                .unwrap();
            project
                .create_element(ElementKind::Package, "WrongOwner", root)
                .unwrap();
            project
                .create_element(ElementKind::Block, "Engine", root)
                .unwrap();
            project
                .create_element(ElementKind::InterfaceBlock, "Iface", root)
                .unwrap();
        }
        let cases = [
            (
                "A,Vehicle,a,Engine,1\n",
                ElementKind::PartProperty,
                "OWNER_AMBIGUOUS",
            ),
            (
                "B,Missing,b,Engine,1\n",
                ElementKind::PartProperty,
                "OWNER_UNRESOLVED",
            ),
            (
                "C,WrongOwner,c,Engine,1\n",
                ElementKind::PartProperty,
                "SEMANTIC_VALIDATION",
            ),
            (
                "D,WrongOwner,d,Missing,1\n",
                ElementKind::PartProperty,
                "TYPE_UNRESOLVED",
            ),
            (
                "E,WrongOwner,e,Scalar,1\n",
                ElementKind::ValueProperty,
                "TYPE_AMBIGUOUS",
            ),
            (
                "F,WrongOwner,f,Engine,2..1\n",
                ElementKind::PartProperty,
                "MULTIPLICITY_INVALID",
            ),
        ];
        for (row, kind, code) in cases {
            let mapping = map(
                "Blocked feature",
                temp_csv(&format!("ID,Owner,Name,Type,Multiplicity\n{row}")),
                None,
                1,
                kind,
                root,
                SpreadsheetIdentificationProperty::ExternalId,
                SpreadsheetSearchScope::TargetRecursive,
                &[
                    ("ID", SpreadsheetSemanticProperty::ExternalId),
                    ("Owner", SpreadsheetSemanticProperty::Owner),
                    ("Name", SpreadsheetSemanticProperty::Name),
                    ("Type", SpreadsheetSemanticProperty::Type),
                    ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
                ],
            );
            let preview = preview_spreadsheet_import_group(
                &SpreadsheetImportMapGroup {
                    mappings: vec![mapping],
                },
                &state,
            );
            assert!(!preview.is_valid(), "expected {code}");
            assert!(
                preview
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code),
                "{:?}",
                preview.diagnostics
            );
        }
        let flow = map(
            "Bad flow",
            temp_csv(
                "ID,Owner,Name,Type,Multiplicity,Flow\nG,Iface,command,Engine,1,input/output\n",
            ),
            None,
            1,
            ElementKind::FlowProperty,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Owner", SpreadsheetSemanticProperty::Owner),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Type", SpreadsheetSemanticProperty::Type),
                ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
                ("Flow", SpreadsheetSemanticProperty::FlowDirection),
            ],
        );
        let preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![flow],
            },
            &state,
        );
        assert!(
            preview
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "FLOW_DIRECTION_INVALID")
        );
    }

    #[test]
    fn pr39_feature_apply_is_atomic_when_one_feature_is_invalid() {
        let (state, root) = workspace("PR39 Atomic");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project
                .create_element(ElementKind::Block, "Vehicle", root)
                .unwrap();
            project
                .create_element(ElementKind::Block, "Engine", root)
                .unwrap();
            project
                .create_element(ElementKind::Package, "WrongOwner", root)
                .unwrap();
        }
        let before = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len();
        let mapping = map(
            "Atomic features",
            temp_csv(
                "ID,Owner,Name,Type,Multiplicity\nGOOD,Vehicle,engine,Engine,1\nBAD,WrongOwner,bad,Engine,1\n",
            ),
            None,
            1,
            ElementKind::PartProperty,
            root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Owner", SpreadsheetSemanticProperty::Owner),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Type", SpreadsheetSemanticProperty::Type),
                ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
            ],
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![mapping],
        };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(!preview.is_valid());
        assert!(apply_spreadsheet_import_group(&group, &state).is_err());
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            before
        );
    }
}

#[cfg(test)]
mod pr40_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use systems_modeler_core::AggregationKind;

    const NS: &str = "catia:pr40-fixture";

    fn workspace(name: &str) -> (WorkspaceState, ElementId) {
        let state = WorkspaceState::default();
        let project = Project::new(name);
        let root = project.root_id;
        *state.project.lock().unwrap() = Some(project);
        (state, root)
    }

    fn fixture_path() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pr40_core_relationships.xlsx")
            .to_string_lossy()
            .into_owned()
    }

    fn temp_csv(contents: &str) -> String {
        let path = std::env::temp_dir().join(format!("pr40-{}.csv", uuid::Uuid::new_v4()));
        fs::write(&path, contents).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[allow(clippy::too_many_arguments)]
    fn relationship_map(
        name: &str,
        source: String,
        worksheet: Option<&str>,
        header_row: usize,
        target: ElementId,
        configured_kind: Option<RelationshipKind>,
        identity: SpreadsheetRelationshipIdentityPolicy,
        columns: &[(&str, SpreadsheetSemanticProperty)],
    ) -> SpreadsheetImportMap {
        SpreadsheetImportMap {
            name: name.into(),
            source,
            worksheet: worksheet.map(ToOwned::to_owned),
            header_row,
            element_kind: ElementKind::Block,
            relationship_kind: configured_kind,
            relationship_identity: identity,
            target_scope: target,
            identification_property: SpreadsheetIdentificationProperty::ExternalId,
            search_scope: SpreadsheetSearchScope::TargetRecursive,
            source_namespace: NS.into(),
            mapping_version: "1".into(),
            column_mappings: columns
                .iter()
                .map(|(source_column, property)| SpreadsheetColumnMapping {
                    source_column: (*source_column).into(),
                    property: *property,
                })
                .collect(),
        }
    }

    fn element_map(
        name: &str,
        source: String,
        kind: ElementKind,
        target: ElementId,
        columns: &[(&str, SpreadsheetSemanticProperty)],
    ) -> SpreadsheetImportMap {
        SpreadsheetImportMap {
            name: name.into(),
            source,
            worksheet: None,
            header_row: 1,
            element_kind: kind,
            relationship_kind: None,
            relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,
            target_scope: target,
            identification_property: SpreadsheetIdentificationProperty::ExternalId,
            search_scope: SpreadsheetSearchScope::TargetRecursive,
            source_namespace: NS.into(),
            mapping_version: "1".into(),
            column_mappings: columns
                .iter()
                .map(|(source_column, property)| SpreadsheetColumnMapping {
                    source_column: (*source_column).into(),
                    property: *property,
                })
                .collect(),
        }
    }

    fn seed(
        project: &mut Project,
        owner: ElementId,
        kind: ElementKind,
        name: &str,
        external_id: &str,
    ) -> ElementId {
        let id = project.create_element(kind, name, owner).unwrap();
        project
            .set_external_id(id, external_key(NS, external_id))
            .unwrap();
        id
    }

    fn seed_structure(
        state: &WorkspaceState,
        root: ElementId,
    ) -> (
        ElementId,
        ElementId,
        ElementId,
        ElementId,
        ElementId,
        ElementId,
    ) {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let structure = seed(
            project,
            root,
            ElementKind::Package,
            "Structure",
            "PKG-STRUCT",
        );
        let vehicle = seed(project, structure, ElementKind::Block, "Vehicle", "VEH");
        let engine = seed(project, structure, ElementKind::Block, "Engine", "ENG");
        let controller = seed(project, structure, ElementKind::Block, "Controller", "CTRL");
        let electric = seed(
            project,
            structure,
            ElementKind::Block,
            "ElectricVehicle",
            "EV",
        );
        let interface = seed(
            project,
            structure,
            ElementKind::InterfaceBlock,
            "PowertrainInterface",
            "IFACE",
        );
        (structure, vehicle, engine, controller, electric, interface)
    }

    fn fixture_columns() -> Vec<(&'static str, SpreadsheetSemanticProperty)> {
        vec![
            ("Connection ID", SpreadsheetSemanticProperty::ExternalId),
            (
                "Relationship",
                SpreadsheetSemanticProperty::RelationshipKind,
            ),
            ("Source Component", SpreadsheetSemanticProperty::Source),
            ("Target Component", SpreadsheetSemanticProperty::Target),
            ("Relationship Name", SpreadsheetSemanticProperty::Name),
            ("Source Role", SpreadsheetSemanticProperty::SourceEndRole),
            ("Target Role", SpreadsheetSemanticProperty::TargetEndRole),
            (
                "Source Cardinality",
                SpreadsheetSemanticProperty::SourceMultiplicity,
            ),
            (
                "Target Cardinality",
                SpreadsheetSemanticProperty::TargetMultiplicity,
            ),
            (
                "Source Navigable",
                SpreadsheetSemanticProperty::SourceNavigable,
            ),
            (
                "Target Navigable",
                SpreadsheetSemanticProperty::TargetNavigable,
            ),
            (
                "Source Aggregation",
                SpreadsheetSemanticProperty::SourceAggregation,
            ),
            (
                "Target Aggregation",
                SpreadsheetSemanticProperty::TargetAggregation,
            ),
            ("Description", SpreadsheetSemanticProperty::Documentation),
            ("Semantic Owner", SpreadsheetSemanticProperty::Owner),
        ]
    }

    fn basic_relationship_columns() -> Vec<(&'static str, SpreadsheetSemanticProperty)> {
        vec![
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Source", SpreadsheetSemanticProperty::Source),
            ("Target", SpreadsheetSemanticProperty::Target),
            ("Owner", SpreadsheetSemanticProperty::Owner),
        ]
    }

    #[test]
    fn pr40_xlsx_creates_all_four_kinds_and_preserves_association_end_semantics() {
        let (state, root) = workspace("PR40 XLSX");
        let (structure, vehicle, engine, controller, electric, interface) =
            seed_structure(&state, root);
        let map = relationship_map(
            "Architecture Connections",
            fixture_path(),
            Some("Architecture Connections"),
            1,
            root,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &fixture_columns(),
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![map],
        };

        let before = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .len();
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 5);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            before
        );

        let project_before = state.project.lock().unwrap().as_ref().unwrap().clone();
        let prepared = prepare_spreadsheet_import(&group, &project_before);
        assert!(
            prepared.plan.operations.iter().any(|operation| matches!(
                operation,
                ModelBuildOperation::CreateRelationship { .. }
            ))
        );
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            before
        );

        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.relationships.len(), 5);

        let association = project
            .relationships
            .values()
            .find(|relationship| relationship.external_id == external_key(NS, "ASSOC-VEH-ENGINE"))
            .unwrap();
        assert_eq!(association.kind, RelationshipKind::Association);
        assert_eq!(association.name, "VehicleEngine");
        assert_eq!(association.documentation, "Vehicle contains engines");
        assert_eq!(association.owner_id, Some(structure));
        assert_eq!(association.source_id, vehicle);
        assert_eq!(association.target_id, engine);
        assert_eq!(association.association_ends.len(), 2);
        assert_eq!(association.association_ends[0].role_name, "vehicle");
        assert_eq!(association.association_ends[1].role_name, "engine");
        assert_eq!(
            association.association_ends[0].multiplicity,
            Multiplicity::ONE
        );
        assert_eq!(
            association.association_ends[1].multiplicity,
            Multiplicity::new(1, None).unwrap()
        );
        assert!(!association.association_ends[0].navigable);
        assert!(association.association_ends[1].navigable);
        assert_eq!(
            association.association_ends[0].aggregation,
            AggregationKind::Composite
        );
        assert_eq!(
            association.association_ends[1].aggregation,
            AggregationKind::None
        );

        let generalization = project
            .relationships
            .values()
            .find(|relationship| relationship.external_id == external_key(NS, "GEN-EV-VEH"))
            .unwrap();
        assert_eq!(generalization.kind, RelationshipKind::Generalization);
        assert_eq!(
            (generalization.source_id, generalization.target_id),
            (electric, vehicle)
        );

        let dependency = project
            .relationships
            .values()
            .find(|relationship| relationship.external_id == external_key(NS, "DEP-CTRL-IF"))
            .unwrap();
        assert_eq!(dependency.kind, RelationshipKind::Dependency);
        assert_eq!(
            (dependency.source_id, dependency.target_id),
            (controller, interface)
        );

        let realization = project
            .relationships
            .values()
            .find(|relationship| relationship.external_id == external_key(NS, "REAL-CTRL-IF"))
            .unwrap();
        assert_eq!(realization.kind, RelationshipKind::Realization);
        assert_eq!(
            (realization.source_id, realization.target_id),
            (controller, interface)
        );
        drop(guard);

        let second = preview_spreadsheet_import_group(&group, &state);
        assert!(second.is_valid(), "{:?}", second.diagnostics);
        assert_eq!(second.totals.no_change, 5);
        assert_eq!(second.totals.create, 0);
        assert_eq!(second.totals.update, 0);
    }

    #[test]
    fn pr40_association_reimport_updates_endpoint_and_fields_without_duplication() {
        let (state, root) = workspace("PR40 Update");
        let (_structure, vehicle, engine, controller, _electric, _interface) =
            seed_structure(&state, root);
        let columns = vec![
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Source", SpreadsheetSemanticProperty::Source),
            ("Target", SpreadsheetSemanticProperty::Target),
            ("Owner", SpreadsheetSemanticProperty::Owner),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Source Role", SpreadsheetSemanticProperty::SourceEndRole),
            ("Target Role", SpreadsheetSemanticProperty::TargetEndRole),
            (
                "Source Mult",
                SpreadsheetSemanticProperty::SourceMultiplicity,
            ),
            (
                "Target Mult",
                SpreadsheetSemanticProperty::TargetMultiplicity,
            ),
            ("Source Nav", SpreadsheetSemanticProperty::SourceNavigable),
            ("Target Nav", SpreadsheetSemanticProperty::TargetNavigable),
            ("Source Agg", SpreadsheetSemanticProperty::SourceAggregation),
            ("Target Agg", SpreadsheetSemanticProperty::TargetAggregation),
            ("Description", SpreadsheetSemanticProperty::Documentation),
        ];
        let first_source = temp_csv(
            "ID,Source,Target,Owner,Name,Source Role,Target Role,Source Mult,Target Mult,Source Nav,Target Nav,Source Agg,Target Agg,Description\nASSOC-100,VEH,ENG,Structure,Powertrain,vehicle,engine,1,1,false,true,composite,none,First\n",
        );
        let first_map = relationship_map(
            "Association",
            first_source,
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        apply_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![first_map],
            },
            &state,
        )
        .unwrap();
        let original_id = {
            let guard = state.project.lock().unwrap();
            guard
                .as_ref()
                .unwrap()
                .relationships
                .values()
                .find(|relationship| relationship.external_id == external_key(NS, "ASSOC-100"))
                .unwrap()
                .id
        };

        let second_source = temp_csv(
            "ID,Source,Target,Owner,Name,Source Role,Target Role,Source Mult,Target Mult,Source Nav,Target Nav,Source Agg,Target Agg,Description\nASSOC-100,VEH,CTRL,Structure,Powertrain,system,controller,1,0..1,true,true,shared,none,Updated\n",
        );
        let second_map = relationship_map(
            "Association",
            second_source,
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        let second_group = SpreadsheetImportMapGroup {
            mappings: vec![second_map],
        };
        let preview = preview_spreadsheet_import_group(&second_group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.update, 1);
        apply_spreadsheet_import_group(&second_group, &state).unwrap();

        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.relationships.len(), 1);
        let association = project.relationship(original_id).unwrap();
        assert_eq!(association.source_id, vehicle);
        assert_eq!(association.target_id, controller);
        assert_ne!(association.target_id, engine);
        assert_eq!(association.documentation, "Updated");
        assert_eq!(association.association_ends[0].role_name, "system");
        assert_eq!(association.association_ends[1].role_name, "controller");
        assert_eq!(
            association.association_ends[1].multiplicity,
            Multiplicity::new(0, Some(1)).unwrap()
        );
        assert_eq!(
            association.association_ends[0].aggregation,
            AggregationKind::Shared
        );
        drop(guard);

        let third = preview_spreadsheet_import_group(&second_group, &state);
        assert_eq!(third.totals.no_change, 1);
    }

    #[test]
    fn pr40_resolves_external_and_exact_qualified_endpoints() {
        let (state, root) = workspace("PR40 Endpoint Identity");
        let (structure, a, b, _controller, _electric, _interface) = seed_structure(&state, root);
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project.rename_element(a, "A").unwrap();
            project.rename_element(b, "B").unwrap();
        }
        let source = temp_csv(
            "ID,Source,Target,Owner\nREL-EXT,VEH,ENG,Structure\nREL-QN,Structure::A,Structure::B,Structure\n",
        );
        let map = relationship_map(
            "Endpoint Identity",
            source,
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![map],
        };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 2);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.relationships.len(), 2);
        assert!(
            project
                .relationships
                .values()
                .all(|relationship| relationship.owner_id == Some(structure))
        );
    }

    #[test]
    fn pr40_ordered_mapgroup_resolves_plan_local_owner_and_endpoints_without_early_commit() {
        let (state, root) = workspace("PR40 Pending");
        let packages = element_map(
            "Packages",
            temp_csv("ID,Name\nPKG-STRUCT,Structure\n"),
            ElementKind::Package,
            root,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
            ],
        );
        let blocks = element_map(
            "Blocks",
            temp_csv("ID,Name,Owner\nA-ID,A,PKG-STRUCT\nB-ID,B,PKG-STRUCT\n"),
            ElementKind::Block,
            root,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Owner", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let relationship = relationship_map(
            "Relationships",
            temp_csv("ID,Source,Target,Owner\nASSOC-PENDING,A-ID,B-ID,PKG-STRUCT\n"),
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![packages, blocks, relationship],
        };
        let before = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len();
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 4);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            before
        );
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            0
        );

        let snapshot = state.project.lock().unwrap().as_ref().unwrap().clone();
        let prepared = prepare_spreadsheet_import(&group, &snapshot);
        assert_eq!(prepared.plan.source_namespace, NS);
        assert!(prepared.plan.operations.iter().any(|operation| matches!(
            operation,
            ModelBuildOperation::CreateRelationship {
                source: BuildReference::External(source),
                target: BuildReference::External(target),
                owner: Some(BuildReference::External(owner)),
                ..
            } if source == "A-ID" && target == "B-ID" && owner == "PKG-STRUCT"
        )));

        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.elements.len(), before + 3);
        assert_eq!(project.relationships.len(), 1);
    }

    #[test]
    fn pr40_blocks_duplicate_unsupported_ambiguous_and_unresolved_rows() {
        let (state, root) = workspace("PR40 Diagnostics");
        let (structure, _vehicle, _engine, _controller, _electric, _interface) =
            seed_structure(&state, root);

        let duplicate = relationship_map(
            "Duplicate",
            temp_csv("ID,Source,Target,Owner\nREL-1,VEH,ENG,Structure\nREL-1,VEH,CTRL,Structure\n"),
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let duplicate_preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![duplicate],
            },
            &state,
        );
        assert!(!duplicate_preview.is_valid());
        assert!(
            duplicate_preview
                .diagnostics
                .iter()
                .any(|item| item.code == "DUPLICATE_SOURCE_EXTERNAL_ID")
        );

        let unsupported = relationship_map(
            "Unsupported",
            temp_csv("ID,Kind,Source,Target,Owner\nREL-2,Include,VEH,ENG,Structure\n"),
            None,
            1,
            root,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Kind", SpreadsheetSemanticProperty::RelationshipKind),
                ("Source", SpreadsheetSemanticProperty::Source),
                ("Target", SpreadsheetSemanticProperty::Target),
                ("Owner", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let unsupported_preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![unsupported],
            },
            &state,
        );
        assert!(!unsupported_preview.is_valid());
        assert!(
            unsupported_preview
                .diagnostics
                .iter()
                .any(|item| item.code == "RELATIONSHIP_KIND_UNSUPPORTED")
        );

        let unresolved = relationship_map(
            "Unresolved",
            temp_csv("ID,Source,Target,Owner\nREL-3,MISSING,ALSO-MISSING,Structure\n"),
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let unresolved_preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![unresolved],
            },
            &state,
        );
        assert!(!unresolved_preview.is_valid());
        assert!(
            unresolved_preview
                .diagnostics
                .iter()
                .any(|item| item.code == "SOURCE_UNRESOLVED")
        );

        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed(project, structure, ElementKind::Block, "Duplicate", "DUP-A");
            seed(project, structure, ElementKind::Block, "Duplicate", "DUP-B");
            seed(project, structure, ElementKind::Block, "Unique", "UNIQUE");
        }
        let ambiguous = relationship_map(
            "Ambiguous",
            temp_csv("ID,Source,Target,Owner\nREL-4,Duplicate,Unique,Structure\n"),
            None,
            1,
            structure,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let ambiguous_preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![ambiguous],
            },
            &state,
        );
        assert!(!ambiguous_preview.is_valid());
        assert!(
            ambiguous_preview
                .diagnostics
                .iter()
                .any(|item| item.code == "SOURCE_AMBIGUOUS")
        );
    }

    #[test]
    fn pr40_explicit_fallback_identity_reuses_unique_match_and_blocks_ambiguity() {
        let (state, root) = workspace("PR40 Fallback");
        let (structure, vehicle, engine, _controller, _electric, _interface) =
            seed_structure(&state, root);
        let unique_id = {
            let mut guard = state.project.lock().unwrap();
            guard
                .as_mut()
                .unwrap()
                .create_association(
                    Some(structure),
                    vec![
                        Project::association_end(
                            vehicle,
                            "",
                            Multiplicity::ONE,
                            true,
                            AggregationKind::None,
                        ),
                        Project::association_end(
                            engine,
                            "",
                            Multiplicity::ONE,
                            true,
                            AggregationKind::None,
                        ),
                    ],
                )
                .unwrap()
        };
        let fallback_columns = [
            ("Source", SpreadsheetSemanticProperty::Source),
            ("Target", SpreadsheetSemanticProperty::Target),
            ("Owner", SpreadsheetSemanticProperty::Owner),
        ];
        let unique = relationship_map(
            "Fallback",
            temp_csv("Source,Target,Owner\nVEH,ENG,Structure\n"),
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::KindSourceTarget,
            &fallback_columns,
        );
        let unique_group = SpreadsheetImportMapGroup {
            mappings: vec![unique],
        };
        let preview = preview_spreadsheet_import_group(&unique_group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.update, 1);
        apply_spreadsheet_import_group(&unique_group, &state).unwrap();
        {
            let guard = state.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            assert_eq!(project.relationships.len(), 1);
            assert!(
                project
                    .relationship(unique_id)
                    .unwrap()
                    .external_id
                    .starts_with(&format!("{NS}::fallback::Association::"))
            );
        }
        let stable = preview_spreadsheet_import_group(&unique_group, &state);
        assert_eq!(stable.totals.no_change, 1);

        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project
                .relationships
                .get_mut(&unique_id)
                .unwrap()
                .external_id = "legacy::one".into();
            let second = project
                .create_association(
                    Some(structure),
                    vec![
                        Project::association_end(
                            vehicle,
                            "x",
                            Multiplicity::ONE,
                            true,
                            AggregationKind::None,
                        ),
                        Project::association_end(
                            engine,
                            "y",
                            Multiplicity::ONE,
                            true,
                            AggregationKind::None,
                        ),
                    ],
                )
                .unwrap();
            project.relationships.get_mut(&second).unwrap().external_id = "legacy::two".into();
        }
        let ambiguous = preview_spreadsheet_import_group(&unique_group, &state);
        assert!(!ambiguous.is_valid());
        assert!(
            ambiguous
                .diagnostics
                .iter()
                .any(|item| item.code == "AMBIGUOUS_RELATIONSHIP")
        );
    }

    #[test]
    fn pr40_reuses_model_core_validation_for_generalization_dependency_and_owner() {
        let (state, root) = workspace("PR40 Validation");
        let (structure, _vehicle, _engine, _controller, _electric, _interface) =
            seed_structure(&state, root);
        let note = {
            let mut guard = state.project.lock().unwrap();
            seed(
                guard.as_mut().unwrap(),
                structure,
                ElementKind::Comment,
                "Note",
                "NOTE",
            )
        };

        for (kind, csv) in [
            (
                RelationshipKind::Generalization,
                "ID,Source,Target,Owner\nBAD-GEN,NOTE,VEH,Structure\n",
            ),
            (
                RelationshipKind::Dependency,
                "ID,Source,Target,Owner\nBAD-DEP,NOTE,VEH,Structure\n",
            ),
        ] {
            let map = relationship_map(
                "Invalid endpoints",
                temp_csv(csv),
                None,
                1,
                root,
                Some(kind),
                SpreadsheetRelationshipIdentityPolicy::ExternalId,
                &basic_relationship_columns(),
            );
            let preview = preview_spreadsheet_import_group(
                &SpreadsheetImportMapGroup {
                    mappings: vec![map],
                },
                &state,
            );
            assert!(!preview.is_valid());
            assert!(
                preview
                    .diagnostics
                    .iter()
                    .any(|item| item.code == "SEMANTIC_VALIDATION")
            );
        }

        let illegal_owner = relationship_map(
            "Illegal owner",
            temp_csv("ID,Source,Target,Owner\nBAD-OWNER,VEH,ENG,NOTE\n"),
            None,
            1,
            root,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let owner_preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![illegal_owner],
            },
            &state,
        );
        assert!(!owner_preview.is_valid());
        assert!(
            owner_preview
                .diagnostics
                .iter()
                .any(|item| item.code == "SEMANTIC_VALIDATION")
        );
        assert!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .element(note)
                .is_ok()
        );
    }

    #[test]
    fn pr40_invalid_endpoint_update_and_blocked_mapgroup_leave_zero_mutation() {
        let (state, root) = workspace("PR40 Atomic");
        let (structure, vehicle, engine, _controller, _electric, _interface) =
            seed_structure(&state, root);
        let note = {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let note = seed(project, structure, ElementKind::Comment, "Note", "NOTE");
            let relationship = project
                .create_relationship(
                    RelationshipKind::Generalization,
                    vehicle,
                    engine,
                    Some(structure),
                )
                .unwrap();
            project
                .relationships
                .get_mut(&relationship)
                .unwrap()
                .external_id = external_key(NS, "GEN-UPDATE");
            note
        };
        let invalid_update = relationship_map(
            "Invalid update",
            temp_csv("ID,Source,Target,Owner\nGEN-UPDATE,VEH,NOTE,Structure\n"),
            None,
            1,
            root,
            Some(RelationshipKind::Generalization),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let update_group = SpreadsheetImportMapGroup {
            mappings: vec![invalid_update],
        };
        let before = state.project.lock().unwrap().as_ref().unwrap().clone();
        let preview = preview_spreadsheet_import_group(&update_group, &state);
        assert!(!preview.is_valid());
        assert!(apply_spreadsheet_import_group(&update_group, &state).is_err());
        let after = state.project.lock().unwrap().as_ref().unwrap().clone();
        let original = before
            .relationships
            .values()
            .find(|relationship| relationship.external_id == external_key(NS, "GEN-UPDATE"))
            .unwrap();
        let unchanged = after.relationship(original.id).unwrap();
        assert_eq!(
            (unchanged.source_id, unchanged.target_id),
            (vehicle, engine)
        );
        assert_eq!(after.elements.len(), before.elements.len());
        assert_eq!(after.relationships.len(), before.relationships.len());
        assert!(after.element(note).is_ok());

        let (state2, root2) = workspace("PR40 Whole Group");
        let packages = element_map(
            "Packages",
            temp_csv("ID,Name\nPKG,Structure\n"),
            ElementKind::Package,
            root2,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
            ],
        );
        let blocks = element_map(
            "Blocks",
            temp_csv("ID,Name,Owner\nA,A,PKG\n"),
            ElementKind::Block,
            root2,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Owner", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let broken_relationship = relationship_map(
            "Broken relationship",
            temp_csv("ID,Source,Target,Owner\nBROKEN,A,MISSING,PKG\n"),
            None,
            1,
            root2,
            Some(RelationshipKind::Association),
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_relationship_columns(),
        );
        let whole_group = SpreadsheetImportMapGroup {
            mappings: vec![packages, blocks, broken_relationship],
        };
        let before2 = state2.project.lock().unwrap().as_ref().unwrap().clone();
        let preview2 = preview_spreadsheet_import_group(&whole_group, &state2);
        assert!(!preview2.is_valid());
        assert!(apply_spreadsheet_import_group(&whole_group, &state2).is_err());
        let after2 = state2.project.lock().unwrap();
        let after2 = after2.as_ref().unwrap();
        assert_eq!(after2.elements.len(), before2.elements.len());
        assert_eq!(after2.relationships.len(), before2.relationships.len());
    }
}

#[cfg(test)]
mod pr41_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use systems_modeler_core::RelationshipId;

    const NS: &str = "catia:pr41-fixture";

    fn workspace(name: &str) -> (WorkspaceState, ElementId, ElementId) {
        let state = WorkspaceState::default();
        let mut project = Project::new(name);
        let root = project.root_id;
        let program = project
            .create_element(ElementKind::Package, "VehicleProgram", root)
            .unwrap();
        project
            .set_external_id(program, external_key(NS, "PROGRAM"))
            .unwrap();
        *state.project.lock().unwrap() = Some(project);
        (state, root, program)
    }

    fn fixture_path() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pr41_requirements_traceability.xlsx")
            .to_string_lossy()
            .into_owned()
    }

    fn temp_csv(contents: &str) -> String {
        let path = std::env::temp_dir().join(format!("pr41-{}.csv", uuid::Uuid::new_v4()));
        fs::write(&path, contents).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn map(
        name: &str,
        source: String,
        worksheet: Option<&str>,
        kind: ElementKind,
        target: ElementId,
        columns: &[(&str, SpreadsheetSemanticProperty)],
    ) -> SpreadsheetImportMap {
        SpreadsheetImportMap {
            name: name.into(),
            source,
            worksheet: worksheet.map(ToOwned::to_owned),
            header_row: 1,
            element_kind: kind,
            relationship_kind: None,
            relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,
            target_scope: target,
            identification_property: SpreadsheetIdentificationProperty::ExternalId,
            search_scope: SpreadsheetSearchScope::TargetRecursive,
            source_namespace: NS.into(),
            mapping_version: "1".into(),
            column_mappings: columns
                .iter()
                .map(|(source_column, property)| SpreadsheetColumnMapping {
                    source_column: (*source_column).into(),
                    property: *property,
                })
                .collect(),
        }
    }

    fn relationship_map(
        name: &str,
        source: String,
        worksheet: Option<&str>,
        target: ElementId,
        configured_kind: Option<RelationshipKind>,
        identity: SpreadsheetRelationshipIdentityPolicy,
        columns: &[(&str, SpreadsheetSemanticProperty)],
    ) -> SpreadsheetImportMap {
        let mut result = map(name, source, worksheet, ElementKind::Block, target, columns);
        result.relationship_kind = configured_kind;
        result.relationship_identity = identity;
        result
    }

    fn seed_element(
        project: &mut Project,
        owner: ElementId,
        kind: ElementKind,
        name: &str,
        external_id: &str,
    ) -> ElementId {
        let id = project.create_element(kind, name, owner).unwrap();
        project
            .set_external_id(id, external_key(NS, external_id))
            .unwrap();
        id
    }

    fn seed_requirement(
        project: &mut Project,
        owner: ElementId,
        name: &str,
        requirement_id: &str,
        external_id: &str,
    ) -> ElementId {
        let id = project
            .create_requirement(name, requirement_id, format!("{name} text"), owner)
            .unwrap();
        project
            .set_external_id(id, external_key(NS, external_id))
            .unwrap();
        id
    }

    fn basic_link_columns(with_owner: bool) -> Vec<(&'static str, SpreadsheetSemanticProperty)> {
        let mut columns = vec![
            ("Link Identifier", SpreadsheetSemanticProperty::ExternalId),
            ("Link Type", SpreadsheetSemanticProperty::RelationshipKind),
            ("Design Object", SpreadsheetSemanticProperty::Source),
            ("Requirement Number", SpreadsheetSemanticProperty::Target),
            ("Description", SpreadsheetSemanticProperty::Documentation),
        ];
        if with_owner {
            columns.push(("Relationship Package", SpreadsheetSemanticProperty::Owner));
        }
        columns
    }

    fn fixture_group(program: ElementId) -> SpreadsheetImportMapGroup {
        let fixture = fixture_path();
        let packages = map(
            "Packages",
            fixture.clone(),
            Some("Package Catalog"),
            ElementKind::Package,
            program,
            &[
                ("Package Key", SpreadsheetSemanticProperty::ExternalId),
                ("Package Label", SpreadsheetSemanticProperty::Name),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let requirements = map(
            "Requirements",
            fixture.clone(),
            Some("Requirement Catalog"),
            ElementKind::Requirement,
            program,
            &[
                (
                    "Requirement External Key",
                    SpreadsheetSemanticProperty::ExternalId,
                ),
                (
                    "Requirement Number",
                    SpreadsheetSemanticProperty::RequirementId,
                ),
                ("Requirement Title", SpreadsheetSemanticProperty::Name),
                (
                    "Requirement Statement",
                    SpreadsheetSemanticProperty::RequirementText,
                ),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let design = map(
            "Design",
            fixture.clone(),
            Some("Design Catalog"),
            ElementKind::Block,
            program,
            &[
                ("Design Key", SpreadsheetSemanticProperty::ExternalId),
                ("Design Name", SpreadsheetSemanticProperty::Name),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let verification = map(
            "Verification",
            fixture.clone(),
            Some("Verification Catalog"),
            ElementKind::TestCase,
            program,
            &[
                ("Test Key", SpreadsheetSemanticProperty::ExternalId),
                ("Test Name", SpreadsheetSemanticProperty::Name),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let links = relationship_map(
            "Requirement Links",
            fixture,
            Some("Requirement Links"),
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(true),
        );
        SpreadsheetImportMapGroup {
            mappings: vec![packages, requirements, design, verification, links],
        }
    }

    fn find_req(project: &Project, requirement_id: &str) -> ElementId {
        project
            .elements
            .values()
            .find(|element| {
                element.kind == ElementKind::Requirement
                    && element.requirement_id.as_deref() == Some(requirement_id)
            })
            .unwrap()
            .id
    }

    fn find_ext(project: &Project, external_id: &str) -> ElementId {
        project
            .elements
            .values()
            .find(|element| element.external_id == external_key(NS, external_id))
            .unwrap()
            .id
    }

    #[test]
    fn pr41_xlsx_ordered_group_creates_all_six_kinds_with_correct_direction_and_plan_local_resolution()
     {
        let (state, _root, program) = workspace("PR41 XLSX");
        let group = fixture_group(program);
        let before_elements = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len();
        let before_relationships = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .len();

        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 18);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            before_elements
        );
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            before_relationships
        );

        let project_snapshot = state.project.lock().unwrap().as_ref().unwrap().clone();
        let prepared = prepare_spreadsheet_import(&group, &project_snapshot);
        let relationship_kinds = prepared
            .plan
            .operations
            .iter()
            .filter_map(|operation| match operation {
                ModelBuildOperation::CreateRelationship { kind, .. } => Some(kind.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for kind in [
            RelationshipKind::DeriveRequirement,
            RelationshipKind::Satisfy,
            RelationshipKind::Verify,
            RelationshipKind::Refine,
            RelationshipKind::Trace,
            RelationshipKind::Copy,
        ] {
            assert!(
                relationship_kinds.contains(&kind),
                "missing {kind:?} in ModelBuildPlan"
            );
        }
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            0
        );

        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.relationships.len(), 6);
        assert_eq!(project.elements.len(), 14);

        let req_sys_1 = find_req(project, "REQ-SYS-001");
        let req_sys_2 = find_req(project, "REQ-SYS-002");
        let req_sub_1 = find_req(project, "REQ-SUB-001");
        let req_copy_1 = find_req(project, "REQ-COPY-001");
        let brake = find_ext(project, "BLK-BRAKE");
        let vehicle = find_ext(project, "BLK-VEH");
        let behavior = find_ext(project, "BLK-BEHAV");
        let test = find_ext(project, "TC-BRAKE");

        let relationship = |external_id: &str| {
            project
                .relationships
                .values()
                .find(|relationship| relationship.external_id == external_key(NS, external_id))
                .unwrap()
        };
        let derive = relationship("DER-1");
        assert_eq!(derive.kind, RelationshipKind::DeriveRequirement);
        assert_eq!((derive.source_id, derive.target_id), (req_sub_1, req_sys_1));
        let satisfy = relationship("SAT-1");
        assert_eq!(satisfy.kind, RelationshipKind::Satisfy);
        assert_eq!((satisfy.source_id, satisfy.target_id), (brake, req_sub_1));
        let verify = relationship("VER-1");
        assert_eq!(verify.kind, RelationshipKind::Verify);
        assert_eq!((verify.source_id, verify.target_id), (test, req_sub_1));
        let refine = relationship("REF-1");
        assert_eq!(refine.kind, RelationshipKind::Refine);
        assert_eq!((refine.source_id, refine.target_id), (behavior, req_sys_2));
        let copy = relationship("COPY-1");
        assert_eq!(copy.kind, RelationshipKind::Copy);
        assert_eq!((copy.source_id, copy.target_id), (req_copy_1, req_sys_1));
        let trace = relationship("TRACE-1");
        assert_eq!(trace.kind, RelationshipKind::Trace);
        assert_eq!((trace.source_id, trace.target_id), (vehicle, req_sys_2));

        let traceability = find_ext(project, "PKG-TRC");
        assert!(
            project
                .relationships
                .values()
                .all(|relationship| relationship.owner_id == Some(traceability))
        );
        let verified_requirement = project.element(req_sub_1).unwrap();
        assert_eq!(
            verified_requirement.requirement_id.as_deref(),
            Some("REQ-SUB-001")
        );
        assert_eq!(
            verified_requirement.requirement_text.as_deref(),
            Some("Brake text")
        );
        drop(guard);

        let second = preview_spreadsheet_import_group(&group, &state);
        assert!(second.is_valid(), "{:?}", second.diagnostics);
        assert_eq!(second.totals.no_change, 18);
        assert_eq!(second.totals.create, 0);
        assert_eq!(second.totals.update, 0);
    }

    #[test]
    fn pr41_ownerless_business_columns_use_explicit_package_target_not_loose_root() {
        let (state, root, program) = workspace("PR41 Owner");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed_element(project, program, ElementKind::Block, "BrakeSystem", "BLK-1");
            seed_requirement(project, program, "Brake Requirement", "REQ-1", "REQEXT-1");
        }
        let source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-OWNER,Satisfy,BLK-1,REQ-1,ownerless business mapping\n",
        );
        let mapping = relationship_map(
            "Requirement Links",
            source.clone(),
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![mapping],
        };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.relationships.len(), 1);
        assert_eq!(
            project.relationships.values().next().unwrap().owner_id,
            Some(program)
        );
        drop(guard);

        let root_mapping = relationship_map(
            "Loose Root",
            source,
            None,
            root,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let blocked = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![root_mapping],
            },
            &state,
        );
        assert!(!blocked.is_valid());
        assert!(
            blocked
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "RELATIONSHIP_OWNER_REQUIRED")
        );
    }

    #[test]
    fn pr41_requirement_id_ambiguity_unresolved_and_unknown_kind_are_blocked() {
        let (state, _root, program) = workspace("PR41 Requirement IDs");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed_element(project, program, ElementKind::Block, "Design", "BLK-1");
            let first = seed_element(
                project,
                program,
                ElementKind::Requirement,
                "First",
                "REQEXT-A",
            );
            let second = seed_element(
                project,
                program,
                ElementKind::Requirement,
                "Second",
                "REQEXT-B",
            );
            project.element_mut(first).unwrap().requirement_id = Some("REQ-DUP".into());
            project.element_mut(second).unwrap().requirement_id = Some("REQ-DUP".into());
        }

        let ambiguous_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-A,Satisfy,BLK-1,REQ-DUP,ambiguous\n",
        );
        let ambiguous_map = relationship_map(
            "Ambiguous Requirement",
            ambiguous_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let ambiguous = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![ambiguous_map],
            },
            &state,
        );
        assert!(!ambiguous.is_valid());
        assert!(
            ambiguous
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "AMBIGUOUS_REQUIREMENT_ID")
        );

        let missing_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-M,Satisfy,BLK-1,REQ-MISSING,missing\n",
        );
        let missing_map = relationship_map(
            "Missing Requirement",
            missing_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let missing = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![missing_map],
            },
            &state,
        );
        assert!(!missing.is_valid());
        assert!(missing.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "TARGET_UNRESOLVED" && diagnostic.reason.contains("Requirement ID")
        }));

        let unknown_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nUNSUP-1,NotARelationship,BLK-1,REQ-DUP,unsupported\n",
        );
        let unknown_map = relationship_map(
            "Unknown Kind",
            unknown_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let unknown = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![unknown_map],
            },
            &state,
        );
        assert!(!unknown.is_valid());
        assert!(
            unknown
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "RELATIONSHIP_KIND_UNSUPPORTED")
        );
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            0
        );
    }

    #[test]
    fn pr41_invalid_satisfy_verify_derive_and_copy_endpoints_are_blocked_without_mutation() {
        let (state, _root, program) = workspace("PR41 Invalid Endpoints");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed_element(project, program, ElementKind::Block, "BlockOne", "BLK-1");
            seed_element(project, program, ElementKind::Block, "BlockTwo", "BLK-2");
            seed_element(project, program, ElementKind::TestCase, "TestOne", "TC-1");
            seed_requirement(project, program, "Req One", "REQ-1", "REQEXT-1");
            seed_requirement(project, program, "Req Two", "REQ-2", "REQEXT-2");
        }
        let cases = [
            ("BAD-SAT-SRC", "Satisfy", "REQEXT-1", "REQ-2"),
            ("BAD-SAT-TGT", "Satisfy", "BLK-1", "BLK-2"),
            ("BAD-VER-SRC", "Verify", "BLK-1", "REQ-1"),
            ("BAD-VER-TGT", "Verify", "TC-1", "BLK-2"),
            ("BAD-DER-SRC", "deriveReqt", "BLK-1", "REQ-1"),
            ("BAD-DER-TGT", "deriveReqt", "REQEXT-1", "BLK-2"),
            ("BAD-COPY-SRC", "Copy", "BLK-1", "REQ-1"),
            ("BAD-COPY-TGT", "Copy", "REQEXT-1", "BLK-2"),
        ];
        for (id, kind, source_value, target_value) in cases {
            let source = temp_csv(&format!(
                "Link Identifier,Link Type,Design Object,Requirement Number,Description\n{id},{kind},{source_value},{target_value},invalid\n"
            ));
            let mapping = relationship_map(
                id,
                source,
                None,
                program,
                None,
                SpreadsheetRelationshipIdentityPolicy::ExternalId,
                &basic_link_columns(false),
            );
            let preview = preview_spreadsheet_import_group(
                &SpreadsheetImportMapGroup {
                    mappings: vec![mapping],
                },
                &state,
            );
            assert!(
                !preview.is_valid(),
                "{id} unexpectedly valid: {:?}",
                preview.diagnostics
            );
            assert_eq!(
                state
                    .project
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .relationships
                    .len(),
                0
            );
        }
    }

    #[test]
    fn pr41_reimport_no_change_valid_update_invalid_update_duplicate_and_kind_mismatch() {
        let (state, _root, program) = workspace("PR41 Reimport");
        let (block_one, block_two, requirement, testcase) = {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let block_one = seed_element(project, program, ElementKind::Block, "BlockOne", "BLK-1");
            let block_two = seed_element(project, program, ElementKind::Block, "BlockTwo", "BLK-2");
            let requirement = seed_requirement(project, program, "Req", "REQ-1", "REQEXT-1");
            let testcase = seed_element(project, program, ElementKind::TestCase, "Test", "TC-1");
            (block_one, block_two, requirement, testcase)
        };

        let first_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-1,Satisfy,BLK-1,REQ-1,first\n",
        );
        let first_map = relationship_map(
            "Satisfy",
            first_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let first_group = SpreadsheetImportMapGroup {
            mappings: vec![first_map],
        };
        apply_spreadsheet_import_group(&first_group, &state).unwrap();
        let relationship_id = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .values()
            .next()
            .unwrap()
            .id;
        let same = preview_spreadsheet_import_group(&first_group, &state);
        assert_eq!(same.totals.no_change, 1);

        let update_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-1,Satisfy,BLK-2,REQ-1,updated\n",
        );
        let update_map = relationship_map(
            "Satisfy Update",
            update_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let update_group = SpreadsheetImportMapGroup {
            mappings: vec![update_map],
        };
        let update_preview = preview_spreadsheet_import_group(&update_group, &state);
        assert!(
            update_preview.is_valid(),
            "{:?}",
            update_preview.diagnostics
        );
        assert_eq!(update_preview.totals.update, 1);
        apply_spreadsheet_import_group(&update_group, &state).unwrap();
        {
            let guard = state.project.lock().unwrap();
            let relationship = guard
                .as_ref()
                .unwrap()
                .relationship(relationship_id)
                .unwrap();
            assert_eq!(relationship.id, relationship_id);
            assert_eq!(relationship.source_id, block_two);
            assert_eq!(relationship.target_id, requirement);
            assert_ne!(relationship.source_id, block_one);
        }

        let invalid_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-1,Satisfy,REQEXT-1,REQ-1,invalid update\n",
        );
        let invalid_map = relationship_map(
            "Invalid Update",
            invalid_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let invalid = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![invalid_map],
            },
            &state,
        );
        assert!(!invalid.is_valid());
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationship(relationship_id)
                .unwrap()
                .source_id,
            block_two
        );

        let mismatch_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-1,Verify,TC-1,REQ-1,wrong kind\n",
        );
        let mismatch_map = relationship_map(
            "Kind Mismatch",
            mismatch_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let mismatch = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![mismatch_map],
            },
            &state,
        );
        assert!(!mismatch.is_valid());
        assert!(
            mismatch
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "RELATIONSHIP_IDENTITY_KIND_MISMATCH")
        );

        let duplicate_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nDUP-1,Satisfy,BLK-1,REQ-1,a\nDUP-1,Satisfy,BLK-2,REQ-1,b\n",
        );
        let duplicate_map = relationship_map(
            "Duplicate",
            duplicate_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let duplicate = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![duplicate_map],
            },
            &state,
        );
        assert!(!duplicate.is_valid());
        assert!(
            duplicate
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "DUPLICATE_SOURCE_EXTERNAL_ID")
        );
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            1
        );
        assert_eq!(
            testcase,
            find_ext(state.project.lock().unwrap().as_ref().unwrap(), "TC-1")
        );
    }

    #[test]
    fn pr41_fallback_identity_requires_configuration_and_ambiguity_blocks() {
        let (state, _root, program) = workspace("PR41 Fallback");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed_element(project, program, ElementKind::Block, "Block", "BLK-1");
            seed_requirement(project, program, "Req", "REQ-1", "REQEXT-1");
        }
        let source = temp_csv("Link Type,Design Object,Requirement Number\nSatisfy,BLK-1,REQ-1\n");
        let columns = [
            ("Link Type", SpreadsheetSemanticProperty::RelationshipKind),
            ("Design Object", SpreadsheetSemanticProperty::Source),
            ("Requirement Number", SpreadsheetSemanticProperty::Target),
        ];
        let unconfigured = relationship_map(
            "No Fallback",
            source.clone(),
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        let blocked = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![unconfigured],
            },
            &state,
        );
        assert!(!blocked.is_valid());
        assert!(
            blocked
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "RELATIONSHIP_EXTERNAL_ID_REQUIRED")
        );

        let configured = relationship_map(
            "Configured Fallback",
            source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::KindSourceTarget,
            &columns,
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![configured],
        };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 1);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        assert_eq!(
            preview_spreadsheet_import_group(&group, &state)
                .totals
                .no_change,
            1
        );

        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let first_id = project.relationships.values().next().unwrap().id;
            let mut duplicate = project.relationship(first_id).unwrap().clone();
            project
                .relationships
                .get_mut(&first_id)
                .unwrap()
                .external_id = "manual::one".into();
            duplicate.id = RelationshipId::new();
            duplicate.external_id = "manual::two".into();
            project.relationships.insert(duplicate.id, duplicate);
        }
        let ambiguous = preview_spreadsheet_import_group(&group, &state);
        assert!(!ambiguous.is_valid());
        assert!(
            ambiguous
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "AMBIGUOUS_RELATIONSHIP")
        );
    }

    #[test]
    fn pr41_late_invalid_verify_rolls_back_whole_mapgroup() {
        let (state, _root, program) = workspace("PR41 Atomic");
        let before = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len();
        let package_source = temp_csv(
            "Package Key,Package Label\nPKG-REQ,Requirements\nPKG-DES,Design\nPKG-VER,Verification\n",
        );
        let block_source = temp_csv(
            "Design Key,Design Name,Parent Package\nBLK-BRAKE,BrakeSystem,Design\nBLK-CONTROLLER,BrakeController,Design\n",
        );
        let requirement_source = temp_csv(
            "Requirement External Key,Requirement Number,Requirement Title,Requirement Statement,Parent Package\nREQEXT-1,REQ-1,Brake Requirement,Brake text,Requirements\n",
        );
        let test_source =
            temp_csv("Test Key,Test Name,Parent Package\nTC-1,BrakeTest,Verification\n");
        let links_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-1,Satisfy,BLK-BRAKE,REQ-1,valid\nVER-1,Verify,TC-1,REQ-1,valid\nVER-BAD,Verify,BLK-CONTROLLER,REQ-1,invalid last row\n",
        );
        let packages = map(
            "Packages",
            package_source,
            None,
            ElementKind::Package,
            program,
            &[
                ("Package Key", SpreadsheetSemanticProperty::ExternalId),
                ("Package Label", SpreadsheetSemanticProperty::Name),
            ],
        );
        let blocks = map(
            "Blocks",
            block_source,
            None,
            ElementKind::Block,
            program,
            &[
                ("Design Key", SpreadsheetSemanticProperty::ExternalId),
                ("Design Name", SpreadsheetSemanticProperty::Name),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let requirements = map(
            "Requirements",
            requirement_source,
            None,
            ElementKind::Requirement,
            program,
            &[
                (
                    "Requirement External Key",
                    SpreadsheetSemanticProperty::ExternalId,
                ),
                (
                    "Requirement Number",
                    SpreadsheetSemanticProperty::RequirementId,
                ),
                ("Requirement Title", SpreadsheetSemanticProperty::Name),
                (
                    "Requirement Statement",
                    SpreadsheetSemanticProperty::RequirementText,
                ),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let tests = map(
            "Tests",
            test_source,
            None,
            ElementKind::TestCase,
            program,
            &[
                ("Test Key", SpreadsheetSemanticProperty::ExternalId),
                ("Test Name", SpreadsheetSemanticProperty::Name),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let links = relationship_map(
            "Links",
            links_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![packages, blocks, requirements, tests, links],
        };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(!preview.is_valid());
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            before
        );
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            0
        );
        assert!(apply_spreadsheet_import_group(&group, &state).is_err());
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            before
        );
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            0
        );
    }
}

#[cfg(test)]
mod pr42_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use systems_modeler_core::RelationshipId;

    const NS: &str = "catia:pr42-fixture";

    fn workspace(name: &str) -> (WorkspaceState, ElementId, ElementId) {
        let state = WorkspaceState::default();
        let mut project = Project::new(name);
        let root = project.root_id;
        let package = project
            .create_element(ElementKind::Package, "Allocation", root)
            .unwrap();
        *state.project.lock().unwrap() = Some(project);
        (state, root, package)
    }

    fn temp_csv(contents: &str) -> String {
        let path = std::env::temp_dir().join(format!("pr42-{}.csv", uuid::Uuid::new_v4()));
        fs::write(&path, contents).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn fixture_path() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pr42_allocations.xlsx")
            .to_string_lossy()
            .into_owned()
    }

    fn seed(
        project: &mut Project,
        owner: ElementId,
        kind: ElementKind,
        name: &str,
        external_id: &str,
    ) -> ElementId {
        let id = project.create_element(kind, name, owner).unwrap();
        project
            .set_external_id(id, external_key(NS, external_id))
            .unwrap();
        id
    }

    fn relationship_map(
        name: &str,
        source: String,
        worksheet: Option<&str>,
        target: ElementId,
        identity: SpreadsheetRelationshipIdentityPolicy,
        columns: &[(&str, SpreadsheetSemanticProperty)],
    ) -> SpreadsheetImportMap {
        SpreadsheetImportMap {
            name: name.into(),
            source,
            worksheet: worksheet.map(ToOwned::to_owned),
            header_row: 1,
            element_kind: ElementKind::Block,
            relationship_kind: Some(RelationshipKind::Allocate),
            relationship_identity: identity,
            target_scope: target,
            identification_property: SpreadsheetIdentificationProperty::ExternalId,
            search_scope: SpreadsheetSearchScope::TargetRecursive,
            source_namespace: NS.into(),
            mapping_version: "1".into(),
            column_mappings: columns
                .iter()
                .map(|(source_column, property)| SpreadsheetColumnMapping {
                    source_column: (*source_column).into(),
                    property: *property,
                })
                .collect(),
        }
    }

    fn element_map(name: &str, source: String, target: ElementId) -> SpreadsheetImportMap {
        SpreadsheetImportMap {
            name: name.into(),
            source,
            worksheet: None,
            header_row: 1,
            element_kind: ElementKind::Block,
            relationship_kind: None,
            relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,
            target_scope: target,
            identification_property: SpreadsheetIdentificationProperty::ExternalId,
            search_scope: SpreadsheetSearchScope::TargetRecursive,
            source_namespace: NS.into(),
            mapping_version: "1".into(),
            column_mappings: vec![
                SpreadsheetColumnMapping {
                    source_column: "Element Key".into(),
                    property: SpreadsheetSemanticProperty::ExternalId,
                },
                SpreadsheetColumnMapping {
                    source_column: "Element Label".into(),
                    property: SpreadsheetSemanticProperty::Name,
                },
            ],
        }
    }

    fn business_columns() -> Vec<(&'static str, SpreadsheetSemanticProperty)> {
        vec![
            ("Allocation ID", SpreadsheetSemanticProperty::ExternalId),
            ("Function", SpreadsheetSemanticProperty::Source),
            ("Allocated Component", SpreadsheetSemanticProperty::Target),
            ("Description", SpreadsheetSemanticProperty::Documentation),
        ]
    }

    #[test]
    fn pr42_xlsx_maps_to_native_allocate_plan_and_preview_is_non_mutating() {
        let (state, _root, package) = workspace("PR42 XLSX");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            for (name, external) in [
                ("LogicalController", "LOGICAL"),
                ("PhysicalController", "PHYSICAL"),
                ("BrakeFunction", "BRAKE-FN"),
                ("BrakeController", "BRAKE-CTRL"),
                ("PowerFunction", "POWER-FN"),
                ("PowerUnit", "POWER-UNIT"),
            ] {
                seed(project, package, ElementKind::Block, name, external);
            }
        }
        let mapping = relationship_map(
            "Functional Allocation",
            fixture_path(),
            Some("Functional Allocation"),
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &business_columns(),
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![mapping],
        };
        let before = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .len();
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 3);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            before
        );

        let snapshot = state.project.lock().unwrap().as_ref().unwrap().clone();
        let prepared = prepare_spreadsheet_import(&group, &snapshot);
        assert!(prepared.plan.operations.iter().any(|operation| matches!(
            operation,
            ModelBuildOperation::CreateRelationship {
                kind: RelationshipKind::Allocate,
                ..
            }
        )));
        assert!(!prepared.plan.operations.iter().any(|operation| matches!(
            operation,
            ModelBuildOperation::CreateDiagram { .. }
                | ModelBuildOperation::PresentElement { .. }
                | ModelBuildOperation::PresentRelationship { .. }
        )));

        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(
            project
                .relationships
                .values()
                .filter(|relationship| relationship.kind == RelationshipKind::Allocate)
                .count(),
            3
        );
        let logical = project
            .relationships
            .values()
            .find(|relationship| relationship.external_id == external_key(NS, "ALLOC-001"))
            .unwrap();
        assert_eq!(logical.owner_id, Some(package));
        assert_eq!(logical.documentation, "Logical to physical allocation");
    }

    #[test]
    fn pr42_csv_external_ids_qnames_and_reimport_update_without_duplication() {
        let (state, _root, package) = workspace("PR42 CSV");
        let (a, b, c) = {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            (
                seed(project, package, ElementKind::Block, "A", "A"),
                seed(project, package, ElementKind::Block, "B", "B"),
                seed(project, package, ElementKind::Block, "C", "C"),
            )
        };
        let columns = [
            ("Allocation Key", SpreadsheetSemanticProperty::ExternalId),
            ("From Element", SpreadsheetSemanticProperty::Source),
            ("To Element", SpreadsheetSemanticProperty::Target),
            ("Notes", SpreadsheetSemanticProperty::Documentation),
        ];
        let first = relationship_map(
            "CSV Allocation",
            temp_csv("Allocation Key,From Element,To Element,Notes\nALLOC-CSV,A,B,first\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        let first_group = SpreadsheetImportMapGroup {
            mappings: vec![first],
        };
        apply_spreadsheet_import_group(&first_group, &state).unwrap();
        let relationship_id = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .values()
            .next()
            .unwrap()
            .id;
        assert_eq!(
            preview_spreadsheet_import_group(&first_group, &state)
                .totals
                .no_change,
            1
        );

        let metadata_update = relationship_map(
            "CSV Allocation",
            temp_csv("Allocation Key,From Element,To Element,Notes\nALLOC-CSV,A,B,updated\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        let metadata_group = SpreadsheetImportMapGroup {
            mappings: vec![metadata_update],
        };
        assert_eq!(
            preview_spreadsheet_import_group(&metadata_group, &state)
                .totals
                .update,
            1
        );
        apply_spreadsheet_import_group(&metadata_group, &state).unwrap();

        let endpoint_update = relationship_map(
            "CSV Allocation",
            temp_csv("Allocation Key,From Element,To Element,Notes\nALLOC-CSV,A,C,updated\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        let endpoint_group = SpreadsheetImportMapGroup {
            mappings: vec![endpoint_update],
        };
        assert_eq!(
            preview_spreadsheet_import_group(&endpoint_group, &state)
                .totals
                .update,
            1
        );
        apply_spreadsheet_import_group(&endpoint_group, &state).unwrap();
        {
            let guard = state.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            let relationship = project.relationship(relationship_id).unwrap();
            assert_eq!(relationship.id, relationship_id);
            assert_eq!(relationship.source_id, a);
            assert_eq!(relationship.target_id, c);
            assert_eq!(relationship.documentation, "updated");
            assert_eq!(project.relationships.len(), 1);
        }

        let (source_qname, target_qname) = {
            let guard = state.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            (
                project.qualified_name(b).unwrap(),
                project.qualified_name(c).unwrap(),
            )
        };
        let qname_map = relationship_map(
            "Qualified Allocation",
            temp_csv(&format!(
                "Allocation Key,From Element,To Element,Notes\nALLOC-Q,{source_qname},{target_qname},qualified\n"
            )),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        apply_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![qname_map],
            },
            &state,
        )
        .unwrap();
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            2
        );
    }

    #[test]
    fn pr42_plan_local_endpoints_and_late_invalid_row_are_atomic() {
        let (blocked_state, _root, package) = workspace("PR42 Atomic");
        let blocks = element_map(
            "Blocks",
            temp_csv("Element Key,Element Label\nA,A\nB,B\n"),
            package,
        );
        let links = relationship_map(
            "Allocations",
            temp_csv(
                "Allocation ID,Function,Allocated Component,Description\nALLOC-1,A,B,valid\nALLOC-BAD,A,Missing,invalid\n",
            ),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &business_columns(),
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![blocks, links],
        };
        let before = blocked_state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len();
        let preview = preview_spreadsheet_import_group(&group, &blocked_state);
        assert!(!preview.is_valid());
        assert!(
            preview
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "TARGET_UNRESOLVED")
        );
        assert_eq!(
            blocked_state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            before
        );
        assert!(
            blocked_state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .is_empty()
        );
        assert!(apply_spreadsheet_import_group(&group, &blocked_state).is_err());

        let (state, _root, package) = workspace("PR42 Plan Local");
        let blocks = element_map(
            "Blocks",
            temp_csv("Element Key,Element Label\nA,A\nB,B\n"),
            package,
        );
        let links = relationship_map(
            "Allocations",
            temp_csv("Allocation ID,Function,Allocated Component,Description\nALLOC-1,A,B,valid\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &business_columns(),
        );
        let valid_group = SpreadsheetImportMapGroup {
            mappings: vec![blocks, links],
        };
        let project = state.project.lock().unwrap().as_ref().unwrap().clone();
        let prepared = prepare_spreadsheet_import(&valid_group, &project);
        assert!(
            prepared.preview.is_valid(),
            "{:?}",
            prepared.preview.diagnostics
        );
        let create_element_pos = prepared
            .plan
            .operations
            .iter()
            .position(|operation| matches!(operation, ModelBuildOperation::CreateElement { .. }))
            .unwrap();
        let allocate_pos = prepared
            .plan
            .operations
            .iter()
            .position(|operation| {
                matches!(
                    operation,
                    ModelBuildOperation::CreateRelationship {
                        kind: RelationshipKind::Allocate,
                        ..
                    }
                )
            })
            .unwrap();
        assert!(create_element_pos < allocate_pos);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            2
        );
        apply_spreadsheet_import_group(&valid_group, &state).unwrap();
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            1
        );
    }

    #[test]
    fn pr42_self_invalid_endpoint_owner_and_reference_failures_are_blocked() {
        let (state, _root, package) = workspace("PR42 Diagnostics");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed(project, package, ElementKind::Block, "A", "A");
            seed(project, package, ElementKind::Block, "B", "B");
            seed(project, package, ElementKind::Comment, "Note", "NOTE");
            seed(project, package, ElementKind::Block, "Dup", "DUP-1");
            seed(project, package, ElementKind::Block, "Dup", "DUP-2");
            seed(project, package, ElementKind::Block, "TargetDup", "TGT-1");
            seed(project, package, ElementKind::Block, "TargetDup", "TGT-2");
        }
        let columns = business_columns();
        let preview_for = |csv: &str| {
            let map = relationship_map(
                "Diagnostics",
                temp_csv(csv),
                None,
                package,
                SpreadsheetRelationshipIdentityPolicy::ExternalId,
                &columns,
            );
            preview_spreadsheet_import_group(
                &SpreadsheetImportMapGroup {
                    mappings: vec![map],
                },
                &state,
            )
        };

        let self_ref =
            preview_for("Allocation ID,Function,Allocated Component,Description\nSELF,A,A,self\n");
        assert!(!self_ref.is_valid());
        assert!(
            self_ref
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ALLOCATION_SELF_REFERENCE")
        );

        let invalid_endpoint = preview_for(
            "Allocation ID,Function,Allocated Component,Description\nBAD-END,NOTE,B,invalid\n",
        );
        assert!(!invalid_endpoint.is_valid());
        assert!(invalid_endpoint.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "SEMANTIC_VALIDATION"
                && diagnostic.reason.contains("invalid Allocation endpoints")
        }));

        let unresolved_source = preview_for(
            "Allocation ID,Function,Allocated Component,Description\nMISS-S,Missing,B,missing\n",
        );
        assert!(
            unresolved_source
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "SOURCE_UNRESOLVED")
        );
        let unresolved_target = preview_for(
            "Allocation ID,Function,Allocated Component,Description\nMISS-T,A,Missing,missing\n",
        );
        assert!(
            unresolved_target
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "TARGET_UNRESOLVED")
        );
        let ambiguous_source = preview_for(
            "Allocation ID,Function,Allocated Component,Description\nAMB-S,Dup,B,ambiguous\n",
        );
        assert!(
            ambiguous_source
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "SOURCE_AMBIGUOUS")
        );
        let ambiguous_target = preview_for(
            "Allocation ID,Function,Allocated Component,Description\nAMB-T,A,TargetDup,ambiguous\n",
        );
        assert!(
            ambiguous_target
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "TARGET_AMBIGUOUS")
        );

        let owner_columns = [
            ("Allocation ID", SpreadsheetSemanticProperty::ExternalId),
            ("Function", SpreadsheetSemanticProperty::Source),
            ("Allocated Component", SpreadsheetSemanticProperty::Target),
            ("Owner", SpreadsheetSemanticProperty::Owner),
        ];
        let unresolved_owner = relationship_map(
            "Owner unresolved",
            temp_csv("Allocation ID,Function,Allocated Component,Owner\nOWN-1,A,B,MissingOwner\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &owner_columns,
        );
        let unresolved_owner = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![unresolved_owner],
            },
            &state,
        );
        assert!(
            unresolved_owner
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "OWNER_UNRESOLVED")
        );
        let illegal_owner = relationship_map(
            "Owner illegal",
            temp_csv("Allocation ID,Function,Allocated Component,Owner\nOWN-2,A,B,A\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &owner_columns,
        );
        let illegal_owner = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![illegal_owner],
            },
            &state,
        );
        assert!(!illegal_owner.is_valid());
        assert!(illegal_owner.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .reason
                .contains("Allocation relationships must be owned by a Model or Package")
        }));
        assert!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .is_empty()
        );
    }

    #[test]
    fn pr42_duplicate_source_id_fallback_ambiguity_and_invalid_update_are_blocked() {
        let (state, _root, package) = workspace("PR42 Identity");
        let (a, b, c) = {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            (
                seed(project, package, ElementKind::Block, "A", "A"),
                seed(project, package, ElementKind::Block, "B", "B"),
                seed(project, package, ElementKind::Block, "C", "C"),
            )
        };
        let duplicate = relationship_map(
            "Duplicate IDs",
            temp_csv(
                "Allocation ID,Function,Allocated Component,Description\nDUP,A,B,one\nDUP,A,C,two\n",
            ),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &business_columns(),
        );
        let duplicate = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![duplicate],
            },
            &state,
        );
        assert!(
            duplicate
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "DUPLICATE_SOURCE_EXTERNAL_ID")
        );

        let initial = relationship_map(
            "Initial",
            temp_csv(
                "Allocation ID,Function,Allocated Component,Description\nALLOC-1,A,B,initial\n",
            ),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &business_columns(),
        );
        apply_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![initial],
            },
            &state,
        )
        .unwrap();
        let relationship_id = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .values()
            .next()
            .unwrap()
            .id;

        let invalid_update = relationship_map(
            "Invalid update",
            temp_csv(
                "Allocation ID,Function,Allocated Component,Description\nALLOC-1,NOTE,B,invalid\n",
            ),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &business_columns(),
        );
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed(project, package, ElementKind::Comment, "NOTE", "NOTE");
        }
        let invalid_preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![invalid_update],
            },
            &state,
        );
        assert!(!invalid_preview.is_valid());
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationship(relationship_id)
                .unwrap()
                .source_id,
            a
        );

        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let first = project
                .create_relationship(RelationshipKind::Allocate, b, c, Some(package))
                .unwrap();
            project.relationships.get_mut(&first).unwrap().external_id = "manual::one".into();
            let mut second = project.relationship(first).unwrap().clone();
            second.id = RelationshipId::new();
            second.external_id = "manual::two".into();
            project.relationships.insert(second.id, second);
        }
        let fallback_columns = [
            ("Function", SpreadsheetSemanticProperty::Source),
            ("Allocated Component", SpreadsheetSemanticProperty::Target),
        ];
        let fallback = relationship_map(
            "Fallback ambiguity",
            temp_csv("Function,Allocated Component\nB,C\n"),
            None,
            package,
            SpreadsheetRelationshipIdentityPolicy::KindSourceTarget,
            &fallback_columns,
        );
        let fallback = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![fallback],
            },
            &state,
        );
        assert!(
            fallback
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "AMBIGUOUS_RELATIONSHIP")
        );
    }
}
