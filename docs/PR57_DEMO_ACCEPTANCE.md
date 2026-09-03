# PR57 Professional EV Demo Acceptance

PR57 is accepted only when the demo remains a single directly importable model-script file and the live presentation stays compact.

Acceptance criteria:

- `examples/model-script/professional-ev-demo.groovy` imports through **File -> Import Model Script**; no backend console or developer-only entry point is required.
- Exactly 12 curated diagrams cover all nine supported diagram families: Package, BDD, IBD, Requirement, Use Case, Activity, State Machine, Sequence, and Parametric.
- Requirements and verification are first-class: stakeholder/system/interface/performance requirements are connected by derive/satisfy/verify/refine/trace/copy semantics and TestCases.
- Structural decomposition is multi-level: ElectricVehicle -> subsystem PartProperties -> Powertrain -> inverter/motor, with typed ProxyPorts/FullPorts, Assembly/Delegation connectors, and ItemFlows.
- The structural story supports drill-down from the vehicle IBD into the Powertrain IBD rather than duplicating many subsystem diagrams.
- Activity behavior includes an end-to-end operation view and a drill-down startup behavior with CallBehavior, CallOperation, SendSignal, AcceptEvent, partitions, and control flow.
- State-machine and sequence views are executable-model views, not disconnected artwork.
- Parametric analysis evaluates tractive force, electrical power, and range from bound ValueProperties, with expected nominal results of 4,500 N, 135 kW, and 400 km.
- Every shipped diagram requests native population, Clean Layout, and Route processing so first-open presentation is suitable for a professional demonstration.
- The demo remains one connected engineering story that supports requirements-to-design-to-behavior-to-analysis-to-verification navigation and change-impact discussion.

This is a demonstration model, not a production vehicle safety case.
