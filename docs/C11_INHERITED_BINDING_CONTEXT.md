# REL-BIND-01A: inherited roles in Parametric contexts

Baseline: `bfdd50ea63b1bc1eec7445c2e52c94c087d538ee`. Independent PR targeting main.
Direct primary-session implementation. Independent review remains outstanding.
Allowed production paths: core `parametrics.rs`; desktop workspace `parametrics.rs`
and `bdd_elements.rs`. Tests: existing core `pr35_parametric_runtime.rs` and
colocated desktop command tests. This file records the bounded work order.

Binding authoring, validation, evaluation and diagram placement currently require
direct ownership. Reuse native classifier-feature membership so inherited visible
ValueProperties and ConstraintProperties retain their original identity and owner.
Validate each selected binding in its evaluation/diagram context: local and ancestor
bindings may apply, but sibling/unrelated context wiring may not leak into a scope.
Duplicate bindings are rejected within one owning context, not globally across
specializations that reuse the same inherited role IDs.

Sources inspected: supplied SysML 1.6 section 8.3.2.3 (binding equality and accessible
property ends), UML 2.5.1 section 9.2.3.2 (inherited non-private members). Preserve the
existing numeric type/unit rules. Reuse the existing ParametricExecutionEngine and
preview-only desktop evaluation; authored defaults must remain unchanged at runtime.

Acceptance: place inherited properties through the native desktop command; validate
and reopen their presentations; evaluate a derived Block through native runtime;
retain original IDs, owners and authored defaults. Reject private-ancestor roles,
foreign context bindings and same-context duplicates without partial mutation;
failed placement preserves presentations and history. Existing Parametric tests pass.

Excluded: inherited ConstraintBlock parameter/expression definitions, arbitrary
nested binding paths, block-instance equality execution, redefinition/subsetting,
new persistence fields, UI redesign and full scale qualification. These remain
explicit separate capabilities. CI and rendered/manual evidence are reported on
the actual candidate; source inspection is not rendered acceptance.
