# PR14 workspace convergence

PR14 starts at merged PR13 commit `fb80053a24477db27f4436ecae291b6030dabfd7`.

## Ownership boundary

The Rust presentation manifest is the single mapping from semantic kind to restrained light-theme category and color. Renderers may select notation geometry, but must not invent semantic colors. The Rust command manifest similarly describes labels, shortcuts, supported diagram families, adapters, and unavailable explanations.

The shared frontend workspace captures transient pointer and panel input only. Rust calculates pointer-centered zoom and Fit Diagram results, validates viewport and panel preferences, and persists them under the application configuration directory. Viewports are keyed by diagram identity and never rewrite semantic or presentation coordinates.

BDD, IBD, State Machine, Sequence, and Activity mount the same canvas, workspace header, engineering grid, scroll container, selection vocabulary, resize handle treatment, and empty-canvas selection behavior. Diagram renderers remain responsible only for their notation.

## Current limitations

Routing adapters currently exist for BDD, IBD, and Activity. Route and Clean Layout metadata explicitly disables unsupported behavior diagrams rather than presenting a silent command. Existing family-specific editing dialogs will be migrated incrementally onto the shared non-blocking dialog host before desktop acceptance is complete.

Linux and Windows CI and visual desktop acceptance remain required before the pull request is marked ready.
