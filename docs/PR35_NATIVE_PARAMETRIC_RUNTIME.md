# PR35 Native Parametric Runtime

> **Documentation status:** MILESTONE CONTRACT — retained reference; current completeness must be revalidated.
> Removed stale in-progress branch instruction; added links to existing runtime implementation and tests.
> See [documentation index](README.md) and [review register](DOCUMENTATION_REVIEW.md).


PR35 integrates the existing Rust-owned SysML Parametric evaluator with the shared PR31–PR34 execution session and PR33 structural runtime. Runtime values remain transient and occurrence-scoped; authored ValueProperty defaults and diagram geometry are not modified by execution.

Implementation and regression fixtures are present in the reviewed repository baseline:

- [Core runtime](../crates/model-core/src/parametric_execution.rs)
- [Desktop integration](../apps/desktop/src-tauri/src/workspace/parametric_execution.rs)
- [Runtime tests](../crates/model-core/tests/pr35_parametric_runtime.rs)
- [Integration contract](../scripts/validate_parametric_runtime_integration.py)

The core adapter uses a scratch project with occurrence-scoped runtime values and
writes results to the shared transient execution session. This documentation
review did not execute the runtime or establish complete solver/UI qualification.
