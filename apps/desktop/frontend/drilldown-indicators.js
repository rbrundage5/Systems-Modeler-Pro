(() => {
  'use strict';

  const SVG_NS = 'http://www.w3.org/2000/svg';
  const MARKER_CLASS = 'smp-child-diagram-indicator';
  const ACTIVITY_RAKE_CLASS = 'smp-sysml-call-behavior-rake';
  let scheduled = false;

  const style = document.createElement('style');
  style.textContent = `
    .${MARKER_CLASS}{
      position:absolute;right:6px;bottom:5px;width:18px;height:12px;z-index:12;
      pointer-events:none;color:#343a40;
      background:
        linear-gradient(currentColor,currentColor) 2px 2px/14px 1.5px no-repeat,
        linear-gradient(currentColor,currentColor) 3px 2px/1.5px 8px no-repeat,
        linear-gradient(currentColor,currentColor) 8.25px 2px/1.5px 8px no-repeat,
        linear-gradient(currentColor,currentColor) 13.5px 2px/1.5px 8px no-repeat;
    }
    .smp-has-child-diagram{cursor:pointer}
    .${ACTIVITY_RAKE_CLASS}{pointer-events:none;color:#343a40}
  `;
  document.head.appendChild(style);

  function app() {
    return window.smpState || null;
  }

  function projectElement(id) {
    return app()?.snapshot?.project?.elements?.find(
      (element) => String(element.id) === String(id),
    ) || null;
  }

  function hasPackageDrillDownTarget(element, currentDiagramId) {
    const application = app();
    if (!application || !element) return false;
    const elementId = String(element.id);
    if ((application.snapshot?.diagrams || []).some((diagram) =>
      String(diagram.id) !== String(currentDiagramId)
      && (String(diagram.owner_id || '') === elementId
        || String(diagram.semantic_context_id || '') === elementId))) return true;
    if ((application.snapshot?.ibd_diagrams || []).some(
      (diagram) => String(diagram.context_block_id || '') === elementId,
    )) return true;
    if ((application.behaviorSnapshot?.diagrams || []).some(
      (diagram) => String(diagram.context_id || '') === elementId,
    )) return true;
    return (application.activitySnapshot?.diagrams || []).some((diagram) =>
      String(diagram.context_id || diagram.owner_id || '') === elementId);
  }

  function ensureHtmlRake(presentation, title) {
    presentation.classList.add('smp-has-child-diagram');
    let marker = presentation.querySelector(`:scope > .${MARKER_CLASS}`);
    if (!marker) {
      marker = document.createElement('span');
      marker.className = MARKER_CLASS;
      marker.setAttribute('aria-hidden', 'true');
      presentation.appendChild(marker);
    }
    marker.title = title;
  }

  function clearHtmlRake(presentation) {
    presentation.classList.remove('smp-has-child-diagram');
    presentation.querySelector(`:scope > .${MARKER_CLASS}`)?.remove();
  }

  function decorateBddAndPackage() {
    const application = app();
    const diagram = application?.snapshot?.diagrams?.find(
      (candidate) => String(candidate.id) === String(application.selectedDiagramId),
    );
    if (!diagram || !['bdd', 'package'].includes(String(diagram.family))) return;
    const nodes = new Map((diagram.nodes || []).map((node) => [String(node.id), node]));
    document.querySelectorAll('#canvas .bdd-block[data-presentation-id]').forEach((presentation) => {
      const node = nodes.get(String(presentation.dataset.presentationId || ''));
      const element = projectElement(node?.element_id);
      let hasChild = false;
      if (diagram.family === 'bdd') {
        hasChild = Boolean(element && ['Block', 'AssociationBlock'].includes(element.kind)
          && (application.snapshot?.ibd_diagrams || []).some(
            (child) => String(child.context_block_id || '') === String(element.id),
          ));
      } else if (diagram.family === 'package') {
        hasChild = hasPackageDrillDownTarget(element, diagram.id);
      }
      if (hasChild) {
        ensureHtmlRake(
          presentation,
          'Child diagram available — double-click to drill down',
        );
      } else {
        clearHtmlRake(presentation);
      }
    });
  }

  function calledActivityId(node) {
    const kind = node?.kind?.Action?.kind;
    if (!kind || typeof kind === 'string' || !kind.CallBehavior) return null;
    return typeof kind.CallBehavior === 'string'
      ? kind.CallBehavior
      : kind.CallBehavior.activity_id || kind.CallBehavior.activityId || null;
  }

  function addActivityRake(group, presentation) {
    if (group.querySelector(`.${ACTIVITY_RAKE_CLASS}`)) return;
    const x = Number(presentation.x) + Number(presentation.width) - 25;
    const y = Number(presentation.y) + Number(presentation.height) - 15;
    if (![x, y].every(Number.isFinite)) return;
    const rake = document.createElementNS(SVG_NS, 'g');
    rake.setAttribute('class', ACTIVITY_RAKE_CLASS);
    const title = document.createElementNS(SVG_NS, 'title');
    title.textContent = 'Referenced Activity is described on another Activity Diagram; double-click to open it.';
    rake.appendChild(title);
    const line = (x1, y1, x2, y2) => {
      const node = document.createElementNS(SVG_NS, 'line');
      node.setAttribute('x1', String(x1));
      node.setAttribute('y1', String(y1));
      node.setAttribute('x2', String(x2));
      node.setAttribute('y2', String(y2));
      node.setAttribute('stroke', 'currentColor');
      node.setAttribute('stroke-width', '1.4');
      rake.appendChild(node);
    };
    line(x, y, x + 16, y);
    line(x + 2, y, x + 2, y + 9);
    line(x + 8, y, x + 8, y + 9);
    line(x + 14, y, x + 14, y + 9);
    group.appendChild(rake);
  }

  function decorateActivity() {
    const application = app();
    const diagram = application?.activitySnapshot?.diagrams?.find(
      (candidate) => String(candidate.id) === String(application.selectedActivityDiagramId),
    );
    if (!diagram) return;
    const activity = application.activitySnapshot?.repository?.activities?.[String(diagram.activity_id)];
    if (!activity) return;
    const presentations = new Map((diagram.nodes || []).map(
      (node) => [String(node.activity_node_id), node],
    ));
    document.querySelectorAll('#canvas .activity-node[data-activity-node-id]').forEach((group) => {
      group.querySelector(`.${ACTIVITY_RAKE_CLASS}`)?.remove();
      const nodeId = String(group.dataset.activityNodeId || '');
      const node = activity.nodes?.find((candidate) => String(candidate.id) === nodeId);
      const called = calledActivityId(node);
      const child = called && (application.activitySnapshot?.diagrams || []).some(
        (candidate) => String(candidate.activity_id) === String(called),
      );
      if (!child) return;
      const presentation = presentations.get(nodeId);
      if (presentation) addActivityRake(group, presentation);
    });
  }

  function decorate() {
    scheduled = false;
    decorateBddAndPackage();
    decorateActivity();
  }

  function schedule() {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(decorate);
  }

  const observer = new MutationObserver(schedule);
  observer.observe(document.getElementById('canvas') || document.body, {
    childList: true,
    subtree: true,
  });
  window.addEventListener('DOMContentLoaded', schedule);
  schedule();
})();
