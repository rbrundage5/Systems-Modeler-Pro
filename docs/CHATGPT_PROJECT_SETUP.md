# What to put in ChatGPT Project sources

## Minimal setup

1. Add `CHATGPT_PROJECT_BRIEF.md` as the primary Project source. It contains the
   stable goals, constraints, and agreed working method.
2. Add the repository URL to the Project context:
   https://github.com/rbrundage5/Systems-Modeler-Pro
3. Add `docs/README.md` and `docs/DOCUMENTATION_REVIEW.md` as reference sources
   if useful. Prefer fetching current GitHub versions during work; uploaded files
   are dated snapshots and must be refreshed after relevant merges.
4. Add legally available SysML/UML references and selected reference-tool examples
   only when needed for standards/visual audits. Label their edition and purpose.
5. Add `IMPORTER_INPUT_SPECIFICATION.md` only for model-generation/import work;
   preserve its baseline and refresh it after importer changes.

## Project instructions — paste this paragraph

Use Systems-Modeler-Pro's current GitHub repository as the authoritative project
reference. Start with its documentation index and applicable repository instructions.
Treat uploaded documents as snapshots and historical PR files as historical evidence.
Follow the active work order, preserve existing functionality and Rust authority,
and record the baseline commit. Within authorized work, let the lead coordinate
bounded audit, implementation, and independent review agents and select available
models according to difficulty and risk. Report evidence, unresolved gaps, and the
next step clearly. Do not infer feature completeness from documentation alone.
Do not merge automatically.

## Remove from active Project sources

Remove superseded copies of the same document, old branch-specific work prompts,
old PR status/checklists, and outdated handoff summaries from the active source set.
Retain originals outside the active source set if historical traceability is needed.
Do not upload the entire repository or every PR document. Keep unique user
requirements, approved designs, and useful reference examples.

The repository review cannot identify which files are currently uploaded to your
ChatGPT Project. Apply these categories to those sources; this change does not
modify the Project settings or remove uploaded sources automatically.

## After this documentation PR

Review/merge the documentation cleanup, refresh the source files above, then create
the separate agent-setup work package. Agent definitions and automatic dispatch
have not been installed or started by this documentation change.
