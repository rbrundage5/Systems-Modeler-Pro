# REL-EDIT-02 / C01.C16: BDD relationship deletion lock order

Baseline: 596e24f. Direct primary-session implementation.
Only production path: desktop `workspace/relationship_editing.rs`.

Delete acquired diagrams then project, while reconnect, creation and structural
Properties acquire project then diagrams. Concurrent operations could deadlock.
Delete now uses the same project-before-diagrams order and acquires both guards
before removing semantic or presentation data. No cascade policy is changed.

Native regressions hold the diagrams lock while a poisoned project lock must
produce an immediate error, detecting the former inversion with a bounded
channel timeout. A poisoned diagrams lock must preserve the serialized project.
This tests native lock behavior, not a source-string assertion.

Hosted CI required. Independent review remains outstanding.
