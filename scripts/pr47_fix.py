from pathlib import Path

p = Path('apps/desktop/src-tauri/src/workspace/bulk_model.rs')
text = p.read_text()

# The first implementation pass deliberately used a generic external-id anchor.
# Remove that block from Connector update; PR47 metadata belongs only to the
# generic relationship update arm below.
wrong = '''                    if let Some(value) = alias {
                        validation_project.relationships.get_mut(&id).map(|_| ());
                        let candidate = validation_project
                            .relationships
                            .values_mut()
                            .find(|relationship| {
                                relationship.kind == kind
                                    && relationship.source_id == next_source
                                    && relationship.target_id == next_target
                                    && relationship.owner_id == next_owner
                            })
                            .expect("validated replacement relationship");
                        candidate.alias = value.clone();
                    }
                    if let Some(value) = extension_condition {
                        let candidate = validation_project
                            .relationships
                            .values_mut()
                            .find(|relationship| {
                                relationship.kind == kind
                                    && relationship.source_id == next_source
                                    && relationship.target_id == next_target
                                    && relationship.owner_id == next_owner
                            })
                            .expect("validated replacement relationship");
                        candidate.extension_condition = value.clone();
                    }
                    if let Some(value) = extension_location {
                        let candidate = validation_project
                            .relationships
                            .values_mut()
                            .find(|relationship| {
                                relationship.kind == kind
                                    && relationship.source_id == next_source
                                    && relationship.target_id == next_target
                                    && relationship.owner_id == next_owner
                            })
                            .expect("validated replacement relationship");
                        candidate.extension_location = value.clone();
                    }
                    validation_project.validate().map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;

'''
if text.count(wrong) != 1:
    raise SystemExit(f'expected exactly one misplaced PR47 validation block, found {text.count(wrong)}')
text = text.replace(wrong, '', 1)

old = '''                    let mut validation_project = project.clone();
                    validation_project.relationships.remove(&id);
                    if kind == RelationshipKind::Association {
                        validation_project
                            .create_association(
                                next_owner,
                                next_association_ends.clone().unwrap_or_default(),
                            )
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?;
                    } else {
                        validation_project
                            .create_relationship(kind.clone(), next_source, next_target, next_owner)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?;
                    }

                    let next_external_id = external_id
'''
new = '''                    let mut validation_project = project.clone();
                    validation_project.relationships.remove(&id);
                    let replacement_id = if kind == RelationshipKind::Association {
                        validation_project.create_association(
                            next_owner,
                            next_association_ends.clone().unwrap_or_default(),
                        )
                    } else {
                        validation_project.create_relationship(
                            kind.clone(),
                            next_source,
                            next_target,
                            next_owner,
                        )
                    }
                    .map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                    {
                        let candidate = validation_project
                            .relationships
                            .get_mut(&replacement_id)
                            .expect("replacement relationship exists");
                        if let Some(value) = alias {
                            candidate.alias = value.clone();
                        }
                        if let Some(value) = extension_condition {
                            candidate.extension_condition = value.clone();
                        }
                        if let Some(value) = extension_location {
                            candidate.extension_location = value.clone();
                        }
                    }
                    validation_project.validate().map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;

                    let next_external_id = external_id
'''
if old not in text:
    raise SystemExit('missing generic relationship candidate-validation anchor')
text = text.replace(old, new, 1)
p.write_text(text)
