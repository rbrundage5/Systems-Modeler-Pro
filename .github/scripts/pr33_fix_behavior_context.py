from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def replace(path: str, old: str, new: str) -> None:
    file = ROOT / path
    text = file.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"expected patch anchor not found in {path}: {old[:80]!r}")
    file.write_text(text.replace(old, new, 1), encoding="utf-8")


activity = "apps/desktop/frontend/activity-ui.js"
replace(
    activity,
    """  function semanticNode(id) {\n    return activeActivity()?.nodes?.find((node) => String(node.id) === String(id)) || null;\n  }\n""",
    """  function projectElement(id) {\n    return state.snapshot?.project?.elements?.find(\n      (element) => String(element.id) === String(id),\n    ) || null;\n  }\n\n  function semanticNode(id) {\n    return activeActivity()?.nodes?.find((node) => String(node.id) === String(id)) || null;\n  }\n""",
)
replace(
    activity,
    """    const diagram = state.activitySnapshot?.diagrams?.find((candidate) => String(candidate.id) === String(id));\n    await window.smpRendererHost?.activate({ diagramId:id, familyId:'activity', name:diagram?.name || 'Activity Diagram', semanticContextId:diagram?.activity_id || '' });\n""",
    """    const diagram = state.activitySnapshot?.diagrams?.find((candidate) => String(candidate.id) === String(id));\n    const activity = diagram\n      ? state.activitySnapshot?.repository?.activities?.[String(diagram.activity_id)]\n      : null;\n    const context = projectElement(activity?.context_id);\n    await window.smpRendererHost?.activate({\n      diagramId: id,\n      familyId: 'activity',\n      name: diagram?.name || 'Activity Diagram',\n      modelElementName: context?.name || activity?.name || diagram?.name || 'Activity',\n      semanticContextId: activity?.context_id || '',\n    });\n""",
)
replace(
    activity,
    """    const selected = state.snapshot.project.elements?.find((element) => element.id === state.selectedElementId);\n    const contextId = selected?.kind === 'Block' ? selected.id : null;\n""",
    """    const selected = state.snapshot.project.elements?.find((element) => element.id === state.selectedElementId);\n    const hasBehaviorContext = ['Block', 'AssociationBlock', 'InterfaceBlock'].includes(selected?.kind);\n    const contextId = hasBehaviorContext ? selected.id : null;\n""",
)
replace(
    activity,
    """    const name = prompt('Activity name', selected?.kind === 'Block' ? `${selected.name} Activity` : 'System Activity');\n    if (!name) return;\n    state.selectedActivityDiagramId = await runCommand('Creating Activity…', () => requireInvoke()('create_activity_diagram', {\n      ownerId,\n      contextId,\n      name,\n    }));\n    state.selectedDiagramId = null;\n    state.selectedBehaviorDiagramId = null;\n    clearActivityInteraction();\n    await loadActivitySnapshot();\n    await loadActivityPalette();\n    render();\n""",
    """    const name = prompt('Activity name', hasBehaviorContext ? `${selected.name} Activity` : 'System Activity');\n    if (!name) return;\n    const diagramId = await runCommand('Creating Activity…', () => requireInvoke()('create_activity_diagram', {\n      ownerId,\n      contextId,\n      name,\n    }));\n    state.selectedDiagramId = null;\n    state.selectedBehaviorDiagramId = null;\n    clearActivityInteraction();\n    await loadActivitySnapshot();\n    await selectActivityDiagram(diagramId);\n""",
)
replace(
    activity,
    """    if (!node) {\n      panel.innerHTML = `<div class=\"property-heading\">Activity</div><label>Name<input value=\"${escapeAttr(activity?.name || diagram.name)}\" disabled></label><label>Stable ID<input value=\"${escapeAttr(diagram.activity_id)}\" disabled></label><div class=\"muted\">Select an Activity node to inspect its Rust-owned semantics.</div>`;\n      return;\n    }\n""",
    """    if (!node) {\n      const context = projectElement(activity?.context_id);\n      panel.innerHTML = `<div class=\"property-heading\">Activity</div><label>Name<input value=\"${escapeAttr(activity?.name || diagram.name)}\" disabled></label><label>Context<input value=\"${escapeAttr(context?.name || 'Uncontextualized')}\" disabled></label><label>Stable ID<input value=\"${escapeAttr(diagram.activity_id)}\" disabled></label><div class=\"muted\">Select an Activity node to inspect its Rust-owned semantics.</div>`;\n      return;\n    }\n""",
)

behavior = "apps/desktop/frontend/behavior-ui.js"
replace(
    behavior,
    """    const diagram = state.behaviorSnapshot?.diagrams?.find((candidate) => String(candidate.id) === String(id));\n    await window.smpRendererHost?.activate({\n      diagramId: id,\n      familyId: diagram?.kind === 'Sequence' ? 'sequence' : 'state-machine',\n      name: diagram?.name || 'Behavior Diagram',\n      semanticContextId: diagram?.context_id || '',\n    });\n""",
    """    const diagram = state.behaviorSnapshot?.diagrams?.find((candidate) => String(candidate.id) === String(id));\n    const context = projectElement(diagram?.context_id);\n    await window.smpRendererHost?.activate({\n      diagramId: id,\n      familyId: diagram?.kind === 'Sequence' ? 'sequence' : 'state-machine',\n      name: diagram?.name || 'Behavior Diagram',\n      modelElementName: context?.name || diagram?.name || 'Behavior',\n      semanticContextId: diagram?.context_id || '',\n    });\n""",
)

ribbon = "apps/desktop/frontend/behavior-ribbon.js"
replace(
    ribbon,
    """      state.selectedBehaviorDiagramId = diagramId;\n      state.selectedDiagramId = null;\n      state.selectedElementId = null;\n      state.selectedRelationshipId = null;\n      state.selectedBehaviorItem = null;\n      state.behaviorTool = null;\n      state.behaviorPending = null;\n      render();\n      $('status').textContent = `${label} created for ${context.name}`;\n""",
    """      if (typeof window.smpSelectBehaviorDiagram === 'function') {\n        await window.smpSelectBehaviorDiagram(diagramId);\n      } else {\n        state.selectedBehaviorDiagramId = diagramId;\n        state.selectedDiagramId = null;\n        state.selectedElementId = null;\n        state.selectedRelationshipId = null;\n        state.selectedBehaviorItem = null;\n        state.behaviorTool = null;\n        state.behaviorPending = null;\n        render();\n      }\n      $('status').textContent = `${label} created for ${context.name}`;\n""",
)

shared = "apps/desktop/frontend/shared-workspace.js"
replace(
    shared,
    """    if (activityId) {\n      const diagram = application.activitySnapshot?.diagrams?.find((item) => String(item.id) === String(activityId));\n      if (diagram) await activate({ diagramId:activityId, familyId:'activity', name:diagram.name, semanticContextId:diagram.activity_id || '' });\n      return;\n    }\n    if (behaviorId) {\n      const diagram = application.behaviorSnapshot?.diagrams?.find((item) => String(item.id) === String(behaviorId));\n      if (diagram) await activate({ diagramId:behaviorId, familyId:diagram.kind === 'Sequence' ? 'sequence' : 'state-machine', name:diagram.name, semanticContextId:diagram.context_id || '' });\n      return;\n    }\n""",
    """    if (activityId) {\n      const diagram = application.activitySnapshot?.diagrams?.find((item) => String(item.id) === String(activityId));\n      const activity = diagram\n        ? application.activitySnapshot?.repository?.activities?.[String(diagram.activity_id)]\n        : null;\n      const context = application.snapshot?.project?.elements?.find(\n        (element) => String(element.id) === String(activity?.context_id),\n      );\n      if (diagram) await activate({\n        diagramId: activityId,\n        familyId: 'activity',\n        name: diagram.name,\n        modelElementName: context?.name || activity?.name || diagram.name,\n        semanticContextId: activity?.context_id || '',\n      });\n      return;\n    }\n    if (behaviorId) {\n      const diagram = application.behaviorSnapshot?.diagrams?.find((item) => String(item.id) === String(behaviorId));\n      const context = application.snapshot?.project?.elements?.find(\n        (element) => String(element.id) === String(diagram?.context_id),\n      );\n      if (diagram) await activate({\n        diagramId: behaviorId,\n        familyId: diagram.kind === 'Sequence' ? 'sequence' : 'state-machine',\n        name: diagram.name,\n        modelElementName: context?.name || diagram.name,\n        semanticContextId: diagram.context_id || '',\n      });\n      return;\n    }\n""",
)

# Frontend contract: the State Machine palette must remain Rust-defined with an Initial
# pseudostate, while Signal is selected as a transition trigger rather than modeled as a
# State Machine vertex. Fail this repair if either contract disappeared.
main = (ROOT / "apps/desktop/src-tauri/src/main.rs").read_text(encoding="utf-8")
completion = (ROOT / "apps/desktop/frontend/behavior-completion-ui.js").read_text(encoding="utf-8")
if 'element_item("Initial", "Initial", "InitialPseudostate")' not in main:
    raise SystemExit("State Machine Rust palette no longer exposes Initial")
if "['None', 'Signal', 'Call', 'Time', 'Change', 'AnyReceive']" not in completion:
    raise SystemExit("State Machine transition editor no longer exposes Signal trigger selection")

print("PR33 behavior context/frame repair applied")
