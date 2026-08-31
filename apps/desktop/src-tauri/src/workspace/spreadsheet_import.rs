use super::{
    WorkspaceState,
    bulk_model::{
        BuildDiagnostic, BuildDiagnosticSeverity, BuildReference, ElementReference, ModelBuildOperation,
        ModelBuildPlan, ModelBuildResult, apply_model_build, external_key, preview_model_build,
    },
};
use calamine::{Reader, open_workbook_auto};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use systems_modeler_core::{Element, ElementId, ElementKind, Project, VisibilityKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SpreadsheetSemanticProperty {
    Name,
    Documentation,
    Owner,
    ExternalId,
    Visibility,
    RequirementId,
    RequirementText,
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
    source_namespace: String,
    identification_value: Option<String>,
}

#[derive(Debug, Clone)]
struct PlannedElement {
    external_id: String,
    kind: ElementKind,
    name: String,
    qualified_name: String,
    owner: ElementReference,
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
    )
}

fn is_namespace_kind(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Model | ElementKind::Package | ElementKind::ModelLibrary
    )
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
        source_namespace: map.map(|mapping| mapping.source_namespace.clone()),
        identification_value,
        semantic_property: property,
        reason: reason.into(),
    }
}

fn read_spreadsheet_table(map: &SpreadsheetImportMap) -> Result<SpreadsheetTable, SpreadsheetImportDiagnostic> {
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

fn read_csv_table(map: &SpreadsheetImportMap) -> Result<SpreadsheetTable, SpreadsheetImportDiagnostic> {
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

fn read_xlsx_table(map: &SpreadsheetImportMap) -> Result<SpreadsheetTable, SpreadsheetImportDiagnostic> {
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
    let first_physical_row = range
        .start()
        .map(|(row, _)| row as usize + 1)
        .unwrap_or(1);
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
        header_indexes.entry(header.as_str()).or_default().push(index);
    }
    let mut result = BTreeMap::new();
    for column_mapping in &map.column_mappings {
        let source_column = column_mapping.source_column.trim();
        let matches = header_indexes.get(source_column).cloned().unwrap_or_default();
        match matches.as_slice() {
            [] => {
                return Err(diagnostic(
                    Some(map),
                    Some(map.header_row),
                    Some(source_column.into()),
                    Some(column_mapping.property),
                    None,
                    "MAPPED_COLUMN_MISSING",
                    format!("mapped column '{source_column}' is not present in the configured header row"),
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
                format!("semantic property {:?} is mapped more than once", column_mapping.property),
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
                row.values.get(*index).cloned().unwrap_or_default().trim().to_string(),
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
    if !supported_kind(&map.element_kind) {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "ELEMENT_KIND_UNSUPPORTED",
            format!("{:?} is outside the PR38 basic-element scope", map.element_kind),
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
            format!("target '{}' ({:?}) is not a semantic namespace", target.name, target.kind),
        ));
    }
    let required_identification_property = match map.identification_property {
        SpreadsheetIdentificationProperty::ExternalId => SpreadsheetSemanticProperty::ExternalId,
        SpreadsheetIdentificationProperty::Name => SpreadsheetSemanticProperty::Name,
        SpreadsheetIdentificationProperty::RequirementId => SpreadsheetSemanticProperty::RequirementId,
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
            format!("{:?} identification requires a mapped {:?} column", map.identification_property, required_identification_property),
        ));
    }
    Ok(())
}

fn distance_from_target(project: &Project, element_id: ElementId, target: ElementId) -> Option<usize> {
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

fn existing_owner_in_scope(
    project: &Project,
    owner_id: ElementId,
    target: ElementId,
    search_scope: SpreadsheetSearchScope,
) -> bool {
    match search_scope {
        SpreadsheetSearchScope::TargetOnly => owner_id == target,
        SpreadsheetSearchScope::TargetRecursive => distance_from_target(project, owner_id, target).is_some(),
    }
}

fn qname_aliases(canonical: &str, root_name: &str, target_qname: &str) -> Vec<String> {
    let mut aliases = vec![canonical.to_string()];
    if let Some(relative) = canonical.strip_prefix(&format!("{root_name}::")) {
        aliases.push(relative.to_string());
    }
    if canonical == target_qname {
        aliases.push(target_qname.rsplit("::").next().unwrap_or(target_qname).to_string());
    } else if let Some(relative) = canonical.strip_prefix(&format!("{target_qname}::")) {
        aliases.push(relative.to_string());
    }
    aliases.sort();
    aliases.dedup();
    aliases
}

fn resolve_owner(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned: &[PlannedElement],
    value: Option<&str>,
) -> Result<ResolvedOwner, SpreadsheetImportDiagnostic> {
    let target = project.element(map.target_scope).expect("validated target");
    let target_qname = project
        .qualified_name(map.target_scope)
        .map_err(|error| diagnostic(Some(map), None, None, Some(SpreadsheetSemanticProperty::Owner), None, "TARGET_SCOPE_UNRESOLVED", error.to_string()))?;
    let root_name = project.element(project.root_id).map(|root| root.name.as_str()).unwrap_or_default();
    let Some(requested) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(ResolvedOwner {
            reference: BuildReference::Existing(map.target_scope),
            qualified_name: target_qname,
            kind: target.kind.clone(),
            depth_from_target: 0,
        });
    };

    let mut existing = project
        .elements
        .values()
        .filter(|element| element.is_namespace())
        .filter(|element| existing_owner_in_scope(project, element.id, map.target_scope, map.search_scope))
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
        .filter(|element| is_namespace_kind(&element.kind))
        .filter(|element| match map.search_scope {
            SpreadsheetSearchScope::TargetOnly => false,
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
            depth_from_target: distance_from_target(project, element.id, map.target_scope).unwrap_or(0),
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
            mapped_column_name(map, SpreadsheetSemanticProperty::Owner),
            Some(SpreadsheetSemanticProperty::Owner),
            None,
            "OWNER_UNRESOLVED",
            format!("could not resolve Owner '{requested}' within {:?} search scope", map.search_scope),
        )),
        _ => Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, SpreadsheetSemanticProperty::Owner),
            Some(SpreadsheetSemanticProperty::Owner),
            None,
            "OWNER_AMBIGUOUS",
            format!("Owner '{requested}' resolves to more than one semantic namespace"),
        )),
    }
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
        .filter(|element| existing_match_in_scope(project, element, map.target_scope, map.search_scope))
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
            format!("external ID identifies {:?}, not {:?}", element.kind, map.element_kind),
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
            format!("{:?} identification value is blank", map.identification_property),
        )
    })?;
    let matches = project
        .elements
        .values()
        .filter(|element| element.kind == map.element_kind)
        .filter(|element| existing_match_in_scope(project, element, map.target_scope, map.search_scope))
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
    let property = match map.identification_property {
        SpreadsheetIdentificationProperty::ExternalId => SpreadsheetSemanticProperty::ExternalId,
        SpreadsheetIdentificationProperty::Name => SpreadsheetSemanticProperty::Name,
        SpreadsheetIdentificationProperty::RequirementId => SpreadsheetSemanticProperty::RequirementId,
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
        source_namespace: map.source_namespace.clone(),
        identification_value: identification_value(map, values),
    }
}

fn mapped_field_changes(
    map: &SpreadsheetImportMap,
    element: &Element,
    owner: &ResolvedOwner,
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
        .map(|value| parse_visibility(map, 0, value))
        .transpose()?;
    let owner_mapped = values.contains_key(&SpreadsheetSemanticProperty::Owner);

    let owner_changed = owner_mapped
        && match owner.reference {
            BuildReference::Existing(id) => element.owner_id != Some(id),
            BuildReference::External(_) => true,
        };
    let changed = name.as_ref().is_some_and(|value| element.name != *value)
        || documentation
            .as_ref()
            .is_some_and(|value| element.documentation != *value)
        || external_id.as_ref().is_some_and(|value| {
            element.external_id != external_key(&map.source_namespace, value)
        })
        || visibility.is_some_and(|value| element.visibility != value)
        || requirement_id
            .as_ref()
            .is_some_and(|value| element.requirement_id.as_deref() != Some(value.as_str()))
        || requirement_text
            .as_ref()
            .is_some_and(|value| element.requirement_text.as_deref() != Some(value.as_str()))
        || owner_changed;

    Ok((
        changed,
        ModelBuildOperation::UpdateElementFields {
            element: BuildReference::Existing(element.id),
            name,
            owner: owner_mapped.then(|| owner.reference.clone()),
            external_id,
            documentation,
            visibility,
            requirement_id,
            requirement_text,
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

            if let Some(external_id) = non_empty_value(&values, SpreadsheetSemanticProperty::ExternalId) {
                let key = external_key(&map.source_namespace, external_id);
                if !seen_source_external_ids.insert(key.clone()) {
                    block_row(diagnostic(
                        Some(map),
                        Some(row.row_number),
                        mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
                        Some(SpreadsheetSemanticProperty::ExternalId),
                        Some(external_id.into()),
                        "DUPLICATE_SOURCE_EXTERNAL_ID",
                        format!("source external ID '{key}' appears more than once in this import group"),
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
            if !is_namespace_kind(&owner.kind) {
                block_row(diagnostic(
                    Some(map),
                    Some(row.row_number),
                    mapped_column_name(map, SpreadsheetSemanticProperty::Owner),
                    Some(SpreadsheetSemanticProperty::Owner),
                    id_value.clone(),
                    "INVALID_OWNERSHIP",
                    format!("{:?} cannot be owned by {:?} in the PR38 packageable-element scope", map.element_kind, owner.kind),
                ));
                continue;
            }

            let existing = match find_existing(map, project, &values) {
                Ok(existing) => existing,
                Err(error) => {
                    block_row(error);
                    continue;
                }
            };

            if let Some(existing) = existing {
                match mapped_field_changes(map, existing, &owner, &values) {
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

            let Some(external_id) = non_empty_value(&values, SpreadsheetSemanticProperty::ExternalId) else {
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
            if map.element_kind == ElementKind::Requirement {
                if non_empty_value(&values, SpreadsheetSemanticProperty::RequirementId).is_none()
                    || non_empty_value(&values, SpreadsheetSemanticProperty::RequirementText).is_none()
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
            }

            let context = row_context(map, row.row_number, &values);
            operations.push(ModelBuildOperation::CreateElement {
                external_id: external_id.to_string(),
                kind: map.element_kind.clone(),
                name: name.to_string(),
                owner: owner.reference.clone(),
                type_ref: None,
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
            if documentation.is_some()
                || visibility.is_some()
                || requirement_id.is_some()
                || requirement_text.is_some()
            {
                operations.push(ModelBuildOperation::UpdateElementFields {
                    element: BuildReference::External(external_id.to_string()),
                    name: None,
                    owner: None,
                    external_id: None,
                    documentation,
                    visibility,
                    requirement_id,
                    requirement_text,
                });
                operation_contexts.push(context);
            }

            let qualified_name = format!("{}::{name}", owner.qualified_name);
            planned.push(PlannedElement {
                external_id: external_id.to_string(),
                kind: map.element_kind.clone(),
                name: name.to_string(),
                qualified_name,
                owner: owner.reference,
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
    let context = diagnostic.operation.and_then(|operation| contexts.get(operation));
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
        if let Some(row) = spreadsheet_diagnostic.row {
            if let Some(row_preview) = preview.rows.iter_mut().find(|candidate| {
                candidate.row == row
                    && spreadsheet_diagnostic
                        .import_map
                        .as_deref()
                        .is_some_and(|map| candidate.import_map == map)
            }) {
                row_preview.action = match spreadsheet_diagnostic.severity {
                    SpreadsheetDiagnosticSeverity::Error => SpreadsheetRowAction::Blocked,
                    SpreadsheetDiagnosticSeverity::Warning => SpreadsheetRowAction::Warning,
                };
            }
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
                ("Package Identifier", SpreadsheetSemanticProperty::ExternalId),
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
        assert!(state.project.lock().unwrap().as_ref().unwrap().elements.len() == 1);
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
                ("Package Identifier", SpreadsheetSemanticProperty::ExternalId),
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
        assert_eq!(project.qualified_name(engine.id).unwrap(), "PR38 Ordered::Vehicle::Structure::Engine");
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
        assert_eq!(preview_spreadsheet_import_group(&group, &state).totals.create, 1);
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
            let package = project.create_element(ElementKind::Package, "Nested", root).unwrap();
            nested = project.create_element(ElementKind::Block, "Controller", package).unwrap();
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
                &SpreadsheetImportMapGroup { mappings: vec![target_only] },
                &state,
            )
            .totals
            .create,
            1
        );
        assert_eq!(
            preview_spreadsheet_import_group(
                &SpreadsheetImportMapGroup { mappings: vec![recursive] },
                &state,
            )
            .totals
            .update,
            1
        );
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().element(nested).unwrap().documentation, "");
    }

    #[test]
    fn external_name_and_requirement_id_identification_classify_updates_and_no_change() {
        let (state, root) = workspace("PR38 Identity");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let block = project.create_element(ElementKind::Block, "Engine", root).unwrap();
            project.set_external_id(block, "catia:pr38-fixture::BLK-ENGINE").unwrap();
            project.element_mut(block).unwrap().documentation = "Old".into();
            let requirement = project.create_requirement("Safety", "REQ-7", "Old text", root).unwrap();
            project.set_external_id(requirement, "legacy::REQ-7").unwrap();
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
        let req_source = temp_csv("New External,Requirement Key,Requirement Text,Name\nREQ-NEW,REQ-7,Updated text,Safety\n");
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
                ("Requirement Key", SpreadsheetSemanticProperty::RequirementId),
                ("Requirement Text", SpreadsheetSemanticProperty::RequirementText),
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
                &SpreadsheetImportMapGroup { mappings: vec![no_change] },
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
            project.create_element(ElementKind::Block, "Controller", root).unwrap();
            project.create_element(ElementKind::Block, "Controller", root).unwrap();
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
            &SpreadsheetImportMapGroup { mappings: vec![mapping] },
            &state,
        );
        assert_eq!(preview.totals.blocked, 1);
        assert!(preview.diagnostics.iter().any(|diagnostic| diagnostic.code == "AMBIGUOUS_IDENTIFICATION"));
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
            assert!(preview.diagnostics.iter().any(|diagnostic| diagnostic.code == code), "missing {code}: {:?}", preview.diagnostics);
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
            &SpreadsheetImportMapGroup { mappings: vec![mapping] },
            &state,
        );
        assert!(!preview.is_valid());
        assert!(preview.diagnostics.iter().any(|diagnostic| diagnostic.code == "TARGET_SCOPE_UNRESOLVED"));
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(), 1);
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
        let before = state.project.lock().unwrap().as_ref().unwrap().elements.len();
        let preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![valid] },
            &state,
        );
        assert!(preview.is_valid());
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(), before);

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
        assert!(apply_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![blocked] },
            &state,
        )
        .is_err());
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(), before);
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
                ("Requirement Text", SpreadsheetSemanticProperty::RequirementText),
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
            ModelBuildOperation::CreateElement { .. } | ModelBuildOperation::UpdateElementFields { .. }
        )));
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(), 1);
        assert!(apply_model_build(&prepared.plan, &state).is_err());
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(), 1);
    }
}
