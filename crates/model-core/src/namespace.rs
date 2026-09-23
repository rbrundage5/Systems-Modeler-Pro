use crate::{Element, ElementId, ElementKind, Project, RelationshipKind, VisibilityKind};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NamespaceResolutionError {
    #[error("model element does not exist")]
    ElementNotFound,
    #[error("'{name}' ({kind:?}) is not a namespace")]
    NotNamespace { name: String, kind: ElementKind },
    #[error("'{name}' has no enclosing namespace")]
    NoEnclosingNamespace { name: String },
    #[error("name to resolve cannot be empty")]
    EmptyName,
    #[error("'{name}' is not visible from '{context}'")]
    NotFound { context: String, name: String },
    #[error("'{name}' is ambiguous from '{context}'; candidates: {candidates:?}")]
    Ambiguous {
        context: String,
        name: String,
        candidates: Vec<String>,
    },
    #[error("qualified name cannot be empty")]
    EmptyQualifiedName,
    #[error("qualified name '{name}' does not identify a model element")]
    QualifiedNameNotFound { name: String },
    #[error("qualified name '{name}' is ambiguous; candidates: {candidates:?}")]
    QualifiedNameAmbiguous {
        name: String,
        candidates: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NameBinding {
    local_name: String,
    element_id: ElementId,
}

impl Project {
    /// Returns the canonical containment-qualified name for a semantic element.
    /// Imports and aliases do not change the element's canonical qualified name.
    pub fn qualified_name(
        &self,
        element_id: ElementId,
    ) -> Result<String, NamespaceResolutionError> {
        let mut names = Vec::new();
        let mut current = Some(element_id);
        let mut visited = HashSet::new();
        while let Some(id) = current {
            if !visited.insert(id) {
                break;
            }
            let element = self
                .elements
                .get(&id)
                .ok_or(NamespaceResolutionError::ElementNotFound)?;
            names.push(element.name.clone());
            current = element.owner_id;
        }
        names.reverse();
        Ok(names.join("::"))
    }

    /// Resolves a canonical containment-qualified name. The model-root prefix is
    /// optional so `Library::Type` and `Model::Library::Type` can identify the
    /// same element when unambiguous.
    pub fn resolve_qualified_name(
        &self,
        qualified_name: &str,
    ) -> Result<ElementId, NamespaceResolutionError> {
        let requested = qualified_name.trim();
        if requested.is_empty() {
            return Err(NamespaceResolutionError::EmptyQualifiedName);
        }

        let mut matches = Vec::new();
        for element in self.elements.values() {
            let canonical = self.qualified_name(element.id)?;
            let relative = canonical
                .split_once("::")
                .map(|(_, remainder)| remainder)
                .unwrap_or(canonical.as_str());
            if requested == canonical || requested == relative {
                matches.push(element.id);
            }
        }
        self.resolve_unique_qualified(requested, matches)
    }

    /// Resolves a name exactly as it is visible from a namespace. Precedence is
    /// owned members, explicit ElementImports, then PackageImports. PackageImport
    /// contributes only the imported package's publicly exported members.
    pub fn resolve_name(
        &self,
        namespace_id: ElementId,
        name: &str,
    ) -> Result<ElementId, NamespaceResolutionError> {
        let requested = name.trim();
        if requested.is_empty() {
            return Err(NamespaceResolutionError::EmptyName);
        }
        if requested.contains("::") {
            return self.resolve_qualified_name(requested);
        }
        let namespace = self.require_namespace(namespace_id)?;
        let context = self
            .qualified_name(namespace_id)
            .unwrap_or_else(|_| namespace.name.clone());

        let owned = self.owned_bindings(namespace_id, false);
        if let Some(result) = self.resolve_binding_layer(&context, requested, owned)? {
            return Ok(result);
        }

        let element_imports = self.element_import_bindings(namespace_id, false);
        if let Some(result) = self.resolve_binding_layer(&context, requested, element_imports)? {
            return Ok(result);
        }

        let package_imports = self.package_import_bindings(namespace_id, false);
        if let Some(result) = self.resolve_binding_layer(&context, requested, package_imports)? {
            return Ok(result);
        }

        Err(NamespaceResolutionError::NotFound {
            context,
            name: requested.to_string(),
        })
    }

    /// Returns the nearest semantic namespace at or above an element. This keeps
    /// callers such as importers and execution engines from reimplementing owner
    /// walking when they need to resolve a type or member from an element context.
    pub fn enclosing_namespace(
        &self,
        context_element_id: ElementId,
    ) -> Result<ElementId, NamespaceResolutionError> {
        let context = self
            .elements
            .get(&context_element_id)
            .ok_or(NamespaceResolutionError::ElementNotFound)?;
        let context_name = context.name.clone();
        let mut current = Some(context_element_id);
        let mut visited = HashSet::new();
        while let Some(id) = current {
            if !visited.insert(id) {
                break;
            }
            let element = self
                .elements
                .get(&id)
                .ok_or(NamespaceResolutionError::ElementNotFound)?;
            if element.is_namespace() {
                return Ok(id);
            }
            current = element.owner_id;
        }
        Err(NamespaceResolutionError::NoEnclosingNamespace { name: context_name })
    }

    /// Resolves a visible name from any semantic element by first locating the
    /// element's nearest enclosing namespace. Qualified names are also accepted.
    pub fn resolve_from_context(
        &self,
        context_element_id: ElementId,
        name: &str,
    ) -> Result<ElementId, NamespaceResolutionError> {
        let requested = name.trim();
        if requested.is_empty() {
            return Err(NamespaceResolutionError::EmptyName);
        }
        if requested.contains("::") {
            return self.resolve_qualified_name(requested);
        }
        let namespace_id = self.enclosing_namespace(context_element_id)?;
        self.resolve_name(namespace_id, requested)
    }

    /// Returns all distinct semantic elements locally visible from a namespace.
    /// Ambiguous names remain represented by their distinct element identities.
    pub fn visible_members(
        &self,
        namespace_id: ElementId,
    ) -> Result<Vec<ElementId>, NamespaceResolutionError> {
        self.require_namespace(namespace_id)?;
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        for binding in self
            .owned_bindings(namespace_id, false)
            .into_iter()
            .chain(self.element_import_bindings(namespace_id, false))
            .chain(self.package_import_bindings(namespace_id, false))
        {
            if seen.insert(binding.element_id) {
                result.push(binding.element_id);
            }
        }
        result.sort_by(|left, right| {
            let left_name = self.qualified_name(*left).unwrap_or_default();
            let right_name = self.qualified_name(*right).unwrap_or_default();
            left_name
                .cmp(&right_name)
                .then_with(|| left.to_string().cmp(&right.to_string()))
        });
        Ok(result)
    }

    pub fn visible_members_from_context(
        &self,
        context_element_id: ElementId,
    ) -> Result<Vec<ElementId>, NamespaceResolutionError> {
        let namespace_id = self.enclosing_namespace(context_element_id)?;
        self.visible_members(namespace_id)
    }

    fn require_namespace(
        &self,
        namespace_id: ElementId,
    ) -> Result<&Element, NamespaceResolutionError> {
        let namespace = self
            .elements
            .get(&namespace_id)
            .ok_or(NamespaceResolutionError::ElementNotFound)?;
        if !namespace.is_namespace() {
            return Err(NamespaceResolutionError::NotNamespace {
                name: namespace.name.clone(),
                kind: namespace.kind.clone(),
            });
        }
        Ok(namespace)
    }

    fn resolve_unique_qualified(
        &self,
        requested: &str,
        mut matches: Vec<ElementId>,
    ) -> Result<ElementId, NamespaceResolutionError> {
        matches.sort_by_key(ToString::to_string);
        matches.dedup();
        match matches.as_slice() {
            [only] => Ok(*only),
            [] => Err(NamespaceResolutionError::QualifiedNameNotFound {
                name: requested.to_string(),
            }),
            _ => Err(NamespaceResolutionError::QualifiedNameAmbiguous {
                name: requested.to_string(),
                candidates: matches
                    .into_iter()
                    .filter_map(|id| self.qualified_name(id).ok())
                    .collect(),
            }),
        }
    }

    fn resolve_binding_layer(
        &self,
        context: &str,
        requested: &str,
        bindings: Vec<NameBinding>,
    ) -> Result<Option<ElementId>, NamespaceResolutionError> {
        let mut matches: Vec<_> = bindings
            .into_iter()
            .filter(|binding| binding.local_name == requested)
            .map(|binding| binding.element_id)
            .collect();
        matches.sort_by_key(ToString::to_string);
        matches.dedup();
        match matches.as_slice() {
            [] => Ok(None),
            [only] => Ok(Some(*only)),
            _ => {
                let mut candidates: Vec<_> = matches
                    .into_iter()
                    .filter_map(|id| self.qualified_name(id).ok())
                    .collect();
                candidates.sort();
                Err(NamespaceResolutionError::Ambiguous {
                    context: context.to_string(),
                    name: requested.to_string(),
                    candidates,
                })
            }
        }
    }

    fn owned_bindings(&self, namespace_id: ElementId, exported_only: bool) -> Vec<NameBinding> {
        let mut bindings: Vec<_> = self
            .elements
            .values()
            .filter(|element| {
                element.owner_id == Some(namespace_id)
                    && element.is_packageable()
                    && (!exported_only || element.visibility == VisibilityKind::Public)
            })
            .map(|element| NameBinding {
                local_name: element.name.clone(),
                element_id: element.id,
            })
            .collect();
        bindings.sort_by(|left, right| {
            left.local_name.cmp(&right.local_name).then_with(|| {
                left.element_id
                    .to_string()
                    .cmp(&right.element_id.to_string())
            })
        });
        bindings
    }

    fn element_import_bindings(
        &self,
        namespace_id: ElementId,
        exported_only: bool,
    ) -> Vec<NameBinding> {
        let mut bindings = Vec::new();
        for relationship in self.relationships.values().filter(|relationship| {
            relationship.kind == RelationshipKind::ElementImport
                && relationship.source_id == namespace_id
                && (!exported_only || relationship.visibility == VisibilityKind::Public)
        }) {
            let Some(target) = self.elements.get(&relationship.target_id) else {
                continue;
            };
            let local_name = relationship
                .alias
                .as_deref()
                .map(str::trim)
                .filter(|alias| !alias.is_empty())
                .unwrap_or(target.name.as_str())
                .to_string();
            bindings.push(NameBinding {
                local_name,
                element_id: target.id,
            });
        }
        bindings.sort_by(|left, right| {
            left.local_name.cmp(&right.local_name).then_with(|| {
                left.element_id
                    .to_string()
                    .cmp(&right.element_id.to_string())
            })
        });
        bindings
    }

    fn package_import_bindings(
        &self,
        namespace_id: ElementId,
        exported_only: bool,
    ) -> Vec<NameBinding> {
        // Index this query's snapshot once. Walking every import path repeated
        // whole-project scans, revisited diamonds exponentially and used the
        // native call stack for deeply chained libraries.
        let mut exports: HashMap<ElementId, Vec<NameBinding>> = HashMap::new();
        for element in self.elements.values() {
            if element.is_packageable()
                && element.visibility == VisibilityKind::Public
                && let Some(owner) = element.owner_id
            {
                exports.entry(owner).or_default().push(NameBinding {
                    local_name: element.name.clone(),
                    element_id: element.id,
                });
            }
        }
        let mut imports: HashMap<ElementId, Vec<ElementId>> = HashMap::new();
        for relationship in self.relationships.values() {
            if relationship.kind == RelationshipKind::PackageImport
                && (relationship.visibility == VisibilityKind::Public
                    || (!exported_only && relationship.source_id == namespace_id))
            {
                imports
                    .entry(relationship.source_id)
                    .or_default()
                    .push(relationship.target_id);
            } else if relationship.kind == RelationshipKind::ElementImport
                && relationship.visibility == VisibilityKind::Public
                && let Some(target) = self.elements.get(&relationship.target_id)
            {
                let local_name = relationship
                    .alias
                    .as_deref()
                    .map(str::trim)
                    .filter(|alias| !alias.is_empty())
                    .unwrap_or(target.name.as_str())
                    .to_string();
                exports
                    .entry(relationship.source_id)
                    .or_default()
                    .push(NameBinding {
                        local_name,
                        element_id: target.id,
                    });
            }
        }
        let mut bindings = Vec::new();
        let mut visited = HashSet::from([namespace_id]);
        let mut pending = imports.remove(&namespace_id).unwrap_or_default();
        while let Some(imported_id) = pending.pop() {
            if !visited.insert(imported_id)
                || !self
                    .elements
                    .get(&imported_id)
                    .is_some_and(Element::is_namespace)
            {
                continue;
            }
            // Every reached package exports publicly, regardless of how many
            // paths lead to it. Aliases and distinct semantic IDs stay separate.
            bindings.extend(exports.remove(&imported_id).unwrap_or_default());
            pending.extend(imports.remove(&imported_id).unwrap_or_default());
        }
        bindings.sort_by(|left, right| {
            left.local_name.cmp(&right.local_name).then_with(|| {
                left.element_id
                    .to_string()
                    .cmp(&right.element_id.to_string())
            })
        });
        bindings.dedup();
        bindings
    }
}
