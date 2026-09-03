# Professional EV SysML Demo

Use `examples/model-script/professional-ev-demo.groovy` as the primary Systems Modeler Pro sales/lunch-and-learn demonstration model.

## Import

1. Create a new project.
2. Open **File**.
3. Choose **Import Model Script**.
4. Select `professional-ev-demo.groovy`.
5. Review the dry-run preview and choose **Import**.

The file is intentionally a single directly importable model script. It uses the product's normal file-import path, native Rust `ModelBuildPlan` construction, semantic validation, automatic diagram population, Clean Layout, and routing. It is not intended to be pasted into a backend or developer console.

## Demo design

The model is deliberately compact: **12 diagrams** cover all nine supported SysML diagram families while keeping one connected engineering story. Requirements and structural decomposition are the center of the demonstration; behavior, parametrics, simulation, sequence execution, interfaces, and verification are shown as consequences of that same model rather than as disconnected feature samples.

### Recommended presentation order

1. **Requirements Architecture** — start with stakeholder needs and show derivation into system/interface/performance requirements. Explain that requirements are semantic model elements, not text boxes.
2. **Electric Vehicle System Definition** — show the system and six primary subsystem types. Expand the `ElectricVehicle` compartments to show the typed PartProperties that realize the parts breakdown.
3. **Electric Vehicle Internal Structure** — show the same architecture as occurrences with ProxyPorts/FullPorts, Assembly/Delegation connectors, and ItemFlows.
4. **Powertrain Internal Structure** — drill down from `Powertrain` to inverter and motor. This is the main structural drill-down example.
5. **Operational Use Cases** — connect driver/service intent to the engineering model with Association, Include, Extend, Generalization, Trace, and Allocate semantics.
6. **Operate Electric Vehicle** — show executable behavior and the call to `StartVehicle`.
7. **Start Vehicle** — drill down from the CallBehavior action. Show CallOperation, SendSignal, AcceptEvent, partitions, and executable control flow.
8. **Vehicle Operating Modes** — demonstrate state-machine execution with Start, Torque, Stop, and Fault signals.
9. **Vehicle Startup Sequence** — show actual structural lifeline paths, including the nested `powertrain.inverter` occurrence, with Operation/Signal signatures.
10. **Vehicle Performance Analysis** — run the parametric evaluation. Expected demonstration results are **4,500 N tractive force**, **135,000 W / 135 kW electrical power**, and **400 km range**.
11. **Verification and Test Traceability** — close the loop with verification-facing requirement copies and TestCases.
12. **EV Digital Engineering Model Organization** — finish by showing that the repository is a structured MBSE project rather than a collection of unrelated drawings.

## Sales story

The demonstration should answer a small set of engineering questions rather than enumerate toolbar features:

- What requirement drives this design decision?
- Which subsystem satisfies it?
- What parts and interfaces implement the subsystem?
- What behavior executes on that architecture?
- What analysis demonstrates the required performance?
- What TestCase verifies the requirement?
- If a subsystem or requirement changes, what connected model content becomes relevant for review or retest?

This sequence illustrates the core MBSE value proposition: a change to one authoritative model element can be traced through requirements, architecture, behavior, analysis, and verification without manually reconciling separate document sets.

## Presentation quality contract

Every shipped diagram in this demo is populated on import and requests native **Clean Layout** and **Route** processing. The demo contract intentionally limits the model to 12 diagrams so the repository stays understandable during a live presentation. Do not expand it into a feature dump unless a new diagram adds a distinct engineering story that cannot be shown by drill-down or an existing view.
