from pathlib import Path

runtime = Path('crates/model-core/src/structural_runtime.rs')
text = runtime.read_text()
old = 'fn classifier_conforms(project: &Project, actual: ElementId, expected: ElementId) -> bool {'
if text.count(old) != 1:
    raise SystemExit(f'classifier_conforms pattern count {text.count(old)}')
text = text.replace(
    old,
    'pub(crate) fn classifier_conforms(project: &Project, actual: ElementId, expected: ElementId) -> bool {',
    1,
)
runtime.write_text(text)

execution = Path('crates/model-core/src/execution.rs')
text = execution.read_text()
old = '''        if let Some(instance_id) = instance_id {
            self.require_instance_semantic(instance_id, None)?;
        }
        validate_runtime_assignment(project, element, &value)?;
        self.values.insert(
            RuntimeValueKey {
                instance_id,
                semantic_element_id,
            },
            value,
        );
        self.push_trace(
            TraceKind::ValueSet,
            TraceContext {
                semantic_element_id: Some(semantic_element_id),
                runtime_instance_id: instance_id,
                ..TraceContext::default()
            },
'''
new = '''        // Preserve the pre-PR33 model-scoped API for a single unambiguous
        // structural occurrence. This lets existing PR31/PR32 callers that write
        // a ValueProperty with `None` continue to update the owning occurrence,
        // while repeated classifier occurrences remain isolated and require an
        // explicit RuntimeInstanceId.
        let instance_id = if instance_id.is_none() && element.kind == ElementKind::ValueProperty {
            element.owner_id.and_then(|owner_id| {
                self.structural_runtime.as_ref().and_then(|runtime| {
                    let mut candidates: Vec<_> = runtime
                        .instances
                        .values()
                        .filter(|instance| {
                            instance.classifier_id.is_some_and(|classifier_id| {
                                crate::structural_runtime::classifier_conforms(
                                    project,
                                    classifier_id,
                                    owner_id,
                                )
                            })
                        })
                        .map(|instance| instance.id)
                        .collect();
                    candidates.sort_by_key(ToString::to_string);
                    candidates.dedup();
                    (candidates.len() == 1).then_some(candidates[0])
                })
            })
        } else {
            instance_id
        };
        if let Some(instance_id) = instance_id {
            self.require_instance_semantic(instance_id, None)?;
        }
        validate_runtime_assignment(project, element, &value)?;
        self.values.insert(
            RuntimeValueKey {
                instance_id,
                semantic_element_id,
            },
            value,
        );
        self.push_trace(
            TraceKind::ValueSet,
            TraceContext {
                semantic_element_id: Some(semantic_element_id),
                runtime_instance_id: instance_id,
                ..TraceContext::default()
            },
'''
if text.count(old) != 1:
    raise SystemExit(f'set_value compatibility pattern count {text.count(old)}')
text = text.replace(old, new, 1)
execution.write_text(text)
