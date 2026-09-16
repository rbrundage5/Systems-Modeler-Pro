# C20.04 — Authenticated project presence

Baseline: `a18a140da2ffdb0e0f31ffa665f292dbed4bba2e` (C20.03 candidate).
One workflow: users viewing/editing the same shared project see the active sessions,
their display labels, authenticated actor identities and editor/viewer roles.

Presence uses a separate authenticated endpoint and never advances the model
revision. The server derives actor and role from credentials, limits stored sessions
and expires silent sessions after 45 seconds. A client cannot delete another actor's
session. Display labels are self-selected labels, not verified account names; actor
identity remains visible. Presence does not indicate a lock or permission to edit.

The open Shared Projects window sends a heartbeat every ten seconds even while a
draft is being composed. Presence responses update only the participant list.
Disconnect/project switch attempts an explicit departure; lost connections and
closed windows expire. Server restart clears ephemeral presence, while the next
heartbeat restores it. Credentials and presence are never written to model files.

Allowed production paths: shared collaboration protocol types/capability; server
presence module and authenticated adapter; desktop collaboration session/command
registration; shared-project controller. Supporting paths: focused server/client/UI
tests and collaboration docs. No model semantics, history, routing, native project
storage, dependencies or CI/agent policy changes.

Acceptance: editor and viewer clients see each other in one project; another project
or unauthorized caller cannot inspect/modify those sessions; departure/expiry clears
entries, revision remains unchanged, and draft text survives heartbeat responses.
Use a controlled monotonic clock for expiry tests; do not wait 45 seconds in tests.
Native two-device rendering/deployment acceptance and independent review remain open.
