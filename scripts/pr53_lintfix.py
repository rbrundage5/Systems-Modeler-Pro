from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"{label}: source fragment not found")
    return text.replace(old, new, 1)


path = Path("apps/desktop/src-tauri/src/workspace/reqif_interchange.rs")
text = path.read_text(encoding="utf-8")
text = replace_once(
    text,
    '''#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReqifSynchronizationPolicy {
    Additive,
    AuthoritativeReqifScope,
}

impl Default for ReqifSynchronizationPolicy {
    fn default() -> Self {
        Self::Additive
    }
}
''',
    '''#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReqifSynchronizationPolicy {
    #[default]
    Additive,
    AuthoritativeReqifScope,
}
''',
    "derive ReqIF synchronization default",
)
text = replace_once(
    text,
    '''
    pub fn spec_types_by_id(&self) -> BTreeMap<String, &ReqifSpecType> {
        self.spec_types
            .iter()
            .map(|spec_type| (spec_type.identifier.clone(), spec_type))
            .collect()
    }
''',
    "\n",
    "remove unused ReqIF spec type index helper",
)
path.write_text(text, encoding="utf-8")

path = Path("apps/desktop/src-tauri/src/workspace/reqif_runtime.rs")
text = path.read_text(encoding="utf-8")
text = replace_once(
    text,
    "    drop(project);\n    candidate\n",
    "    candidate\n",
    "remove ineffective reference drop",
)
text = replace_once(
    text,
    '''    if let Some(history_state) = history_state {
        if let Err(reason) = history::checkpoint_states(workspace, activity, history_state) {
            prepared.preview.diagnostics.push(diagnostic(
                ReqifDiagnosticSeverity::Error,
                "SEMANTIC_VALIDATION",
                reason,
                None,
                None,
            ));
            prepared.preview.recount();
            return prepared.preview;
        }
    }
''',
    '''    if let Some(history_state) = history_state
        && let Err(reason) = history::checkpoint_states(workspace, activity, history_state)
    {
        prepared.preview.diagnostics.push(diagnostic(
            ReqifDiagnosticSeverity::Error,
            "SEMANTIC_VALIDATION",
            reason,
            None,
            None,
        ));
        prepared.preview.recount();
        return prepared.preview;
    }
''',
    "collapse ReqIF history checkpoint conditional",
)
text = replace_once(
    text,
    '''        if let Some((identifier, source)) = binding_lookup.get(&element.id.to_string()) {
            if !used_identifiers.contains(*identifier)
                && let Some(original) = source
                    .document
                    .spec_objects
                    .iter()
                    .find(|object| object.identifier == **identifier)
            {
                for datatype in &source.document.datatypes {
                    datatypes
                        .entry(datatype.identifier.clone())
                        .or_insert_with(|| datatype.clone());
                }
                for spec_type in &source.document.spec_types {
                    spec_types
                        .entry(spec_type.identifier.clone())
                        .or_insert_with(|| spec_type.clone());
                }
                let object = update_imported_object(original.clone(), element, source);
                used_identifiers.insert(object.identifier.clone());
                object_ids_by_native.insert(element.id.to_string(), object.identifier.clone());
                spec_objects.push(object);
                continue;
            }
        }
''',
    '''        if let Some((identifier, source)) = binding_lookup.get(&element.id.to_string())
            && !used_identifiers.contains(*identifier)
            && let Some(original) = source
                .document
                .spec_objects
                .iter()
                .find(|object| object.identifier == **identifier)
        {
            for datatype in &source.document.datatypes {
                datatypes
                    .entry(datatype.identifier.clone())
                    .or_insert_with(|| datatype.clone());
            }
            for spec_type in &source.document.spec_types {
                spec_types
                    .entry(spec_type.identifier.clone())
                    .or_insert_with(|| spec_type.clone());
            }
            let object = update_imported_object(original.clone(), element, source);
            used_identifiers.insert(object.identifier.clone());
            object_ids_by_native.insert(element.id.to_string(), object.identifier.clone());
            spec_objects.push(object);
            continue;
        }
''',
    "collapse ReqIF imported object reuse conditional",
)
path.write_text(text, encoding="utf-8")
