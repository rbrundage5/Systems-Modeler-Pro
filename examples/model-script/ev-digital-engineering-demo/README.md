# Electric Vehicle Digital Engineering Demo

This directory contains the presentation-grade Systems Modeler Pro SysML demonstration used for PR57. It is intentionally **not** a feature dump. The semantic model is broad enough to exercise engineering traceability, interfaces, behavior, runtime execution, and parametric analysis, while the visible presentation is limited to a curated set of **14 diagrams**.

## Demo objective

The demo should tell one connected engineering story:

**requirements → architecture/parts breakdown → internal interfaces → behavior → executable analysis/simulation → verification**

It must demonstrate that Systems Modeler Pro is a semantic MBSE environment rather than a collection of unrelated drawings. The audience should be able to follow the model without needing to understand the repository beforehand.

## Import order

Import the modules in this exact order into a new project:

1. `00-foundation.groovy`
2. `10-architecture-structure.groovy`
3. `11-architecture-execution-features.groovy`
4. `20-requirements-verification.groovy`
5. `21-use-cases-traceability.groovy`
6. `30-vehicle-connectors.groovy`
7. `31-subsystem-connectors.groovy`
8. `40-core-activities.groovy`
9. `41-extended-activities.groovy`
10. `50-state-machines.groovy`
11. `51-sequences.groovy`
12. `52-parametrics-runtime-diagrams.groovy`

Every module uses the same source namespace, `demo:ev-digital-engineering:v3`, and stable External IDs. Reapplying the modules is intended to update/reuse the same semantics rather than duplicate the model.

## Final presentation set — 14 diagrams

| Order | Diagram | Family | Purpose |
| ---: | --- | --- | --- |
| 1 | **EV Digital Engineering Project Overview** | Package | Orient the audience to the engineering repository. |
| 2 | **Core EV System Requirements** | Requirement | Establish the system-level engineering obligations. |
| 3 | **Verification Requirements and Test Cases** | Requirement | Emphasize verification planning and requirement coverage. |
| 4 | **Electric Vehicle System Breakdown** | BDD | Show the vehicle definition and major owned subsystem usages/parts. |
| 5 | **Powertrain Component Breakdown** | BDD | Show decomposition depth and the drill-down architecture story. |
| 6 | **Electric Vehicle Internal Interfaces** | IBD | Show real parts, typed ports, connectors, and ItemFlows. |
| 7 | **Energy Storage and Charging Internal View** | IBD | Show a focused subsystem internal architecture and charging boundary. |
| 8 | **EV Operational Use Cases** | Use Case | Connect actors and operational intent to the modeled system. |
| 9 | **Operate Electric Vehicle** | Activity | Show end-to-end behavior using CallBehavior actions. |
| 10 | **Start Vehicle — Drill-Down** | Activity | Demonstrate semantic drill-down into a child behavior. |
| 11 | **Vehicle Operational Modes** | State Machine | Show lifecycle/mode behavior and supported event triggers. |
| 12 | **Vehicle Startup Sequence** | Sequence | Show interactions across real structural PartProperty paths. |
| 13 | **Tractive Force Analysis — Expected 4.5 kN** | Parametric | Execute `F = m * a` against real ValueProperties. |
| 14 | **Driving Range Analysis — Expected 400 km** | Parametric | Execute `R = E / C` against real ValueProperties. |

The underlying model also contains additional Activities, a battery State Machine, a driving Sequence, electrical-power analysis, and thermal-margin analysis. Those semantics remain available for deeper engineering discussion and regression coverage, but they are deliberately **not** presented as additional diagrams in the standard sales walkthrough.

## Model scope

### Requirements and verification

The model contains stakeholder, system, interface, performance, safety/reliability, and verification requirements plus TestCases. The semantic traceability set must include:

- `DeriveRequirement`
- `Satisfy`
- `Verify`
- `Refine`
- `Trace`
- `Copy`

The presentation should emphasize that architecture, behavior, analysis, and TestCases connect back to requirements. The requirement views must remain readable; do not turn either diagram into a wall of boxes.

### Architecture and parts breakdown

The vehicle architecture uses real `PartProperty` composition. The major vehicle usages are:

- Powertrain
- Energy Storage
- Controls
- HMI
- Thermal Management
- Brakes
- Sensors

The child architecture includes Motor, Inverter, Battery Pack, BMS, Charger, VCU, thermal components, brake components, and sensors. Do not replace this with legacy `Composition` relationships.

### Interfaces

Interfaces are typed through reusable `InterfaceBlock` definitions. The model demonstrates:

- `ProxyPort`
- `FullPort`
- Assembly connectors
- Delegation connectors
- nested connector paths
- `ItemFlow`
- conveyed modeled items

The two IBDs are the primary visual proof that the architecture is connected rather than merely decomposed.

### Behavior and drill-down

`OperateElectricVehicle` is the high-level Activity. Its CallBehavior actions invoke child Activities, including `StartVehicleBehavior`. The demo should explicitly open **Start Vehicle — Drill-Down** from the parent behavior to demonstrate model navigation.

The behavior repository also contains real Operations, Parameters, Signals, Receptions, pins, ObjectFlows, ControlFlows, and partitions tied to model elements.

### State and sequence behavior

The vehicle State Machine demonstrates real trigger semantics supported by the application. The startup Sequence uses lifelines that resolve to actual structural PartProperty paths and messages that resolve to actual Operations or Signals.

### Parametric analysis / simulation

Four executable engineering analyses remain in the semantic model:

| Analysis | Equation | Controlled inputs | Expected result |
| --- | --- | --- | ---: |
| Tractive force | `F = m * a` | `m=1800`, `a=2.5` | `4500 N` |
| Electrical power | `P = V * I` | `V=400`, `I=375` | `150000 W` |
| Driving range | `R = E / C` | `E=84`, `C=0.21` | `400 km` |
| Thermal margin | `M = L - Q` | `L=18`, `Q=12` | `6 kW` |

Only tractive force and driving range are part of the standard 14-diagram presentation. Electrical power and thermal margin remain available as additional executable capability without adding visual clutter.

### Runtime / occurrence demonstration

`DemoFleet` contains two independent `ElectricVehicle` PartProperty occurrences (`vehicleA` and `vehicleB`). Use this structure when demonstrating occurrence-specific runtime values or execution isolation. The unrelated `FacilitySupportSystem` branch exists intentionally to support an impact-analysis discussion about what is *not* affected by an EV change.

## Presentation quality contract

Every final diagram must satisfy all of the following before PR57 is considered presentation-ready:

- native SysML frame/header is visible and correct;
- diagram name and semantic context are correct;
- no blank or token-only diagrams;
- clean layout completes successfully;
- routing completes successfully;
- connectors/relationships do not pass through nodes, compartments, ports, labels, or the frame;
- no detached relationship labels;
- no hidden connector endpoints;
- no overlapping primary nodes;
- hierarchy is visually obvious;
- labels are readable at normal fit-to-view zoom;
- unnecessary elements are omitted from the presentation rather than squeezed into the canvas;
- consistent left-to-right or top-to-bottom engineering flow is used within a diagram;
- drill-down indicators are visible where a meaningful child diagram exists;
- all presentations refer to existing semantic elements; diagrams are never the semantic source of truth.

## Density limits

The goal is clarity, not maximum element count.

- Package overview: major top-level packages only.
- Requirement views: target roughly 6–12 primary requirement/TestCase nodes per view.
- BDD views: target roughly 4–10 primary blocks per view.
- IBD views: target roughly 4–8 primary parts plus only the ports/connectors necessary to tell the interface story.
- Use Case: target roughly 3 actors and 5–7 use cases.
- Activity: target roughly 5–9 visible primary nodes.
- State Machine: target roughly 5–8 principal states/pseudostates.
- Sequence: target roughly 4–5 lifelines and a concise startup exchange.
- Parametric: show one engineering question per diagram.

If a view exceeds these limits, prefer drill-down or semantic depth in the repository instead of adding more boxes to the presentation.

## Recommended live walkthrough

1. Open **EV Digital Engineering Project Overview**.
2. Open **Core EV System Requirements** and establish what the system must achieve.
3. Open **Verification Requirements and Test Cases** and explain requirement-to-evidence planning.
4. Open **Electric Vehicle System Breakdown** and show the major parts/subsystems.
5. Drill into **Powertrain Component Breakdown**.
6. Open **Electric Vehicle Internal Interfaces** and show typed ports, connectors and ItemFlows.
7. Open **Energy Storage and Charging Internal View** for one focused subsystem example.
8. Open **EV Operational Use Cases**.
9. Open **Operate Electric Vehicle**, then drill into **Start Vehicle — Drill-Down**.
10. Open **Vehicle Operational Modes**.
11. Open **Vehicle Startup Sequence** and point out that the lifelines/messages resolve to the same architecture and executable definitions.
12. Open **Tractive Force Analysis — Expected 4.5 kN**, run/evaluate it, and show the expected output.
13. Open **Driving Range Analysis — Expected 400 km**, change a controlled input if useful, evaluate again, then reset/re-run to demonstrate deterministic analysis.
14. If runtime isolation is relevant to the audience, select the `DemoFleet.vehicleA` / `vehicleB` occurrences from the repository rather than adding another standard presentation diagram.

## PR57 completion checklist

PR57 is complete only when:

- [ ] all 12 model-script modules dry-run without diagnostics;
- [ ] all modules apply atomically in numeric order;
- [ ] reapplying the modules does not duplicate semantic content or diagrams;
- [ ] the project validates after import;
- [ ] exactly 14 standard demo diagrams exist;
- [ ] all nine supported diagram families are represented;
- [ ] every final diagram is populated, clean-laid-out and routed;
- [ ] both requirement diagrams are readable and emphasize requirement/verification intent;
- [ ] the BDD → IBD / Activity → child Activity drill-down paths work visibly;
- [ ] connector and ItemFlow semantics validate across the IBDs;
- [ ] startup Activity/State Machine/Sequence execution remains functional;
- [ ] tractive-force evaluation produces `4500` from controlled defaults;
- [ ] driving-range evaluation produces `400` from controlled defaults;
- [ ] repeated vehicle occurrences retain runtime isolation;
- [ ] portable project/export/re-import does not lose semantic identity;
- [ ] existing core and desktop regression suites remain green;
- [ ] no previously qualified modeling/editing behavior is removed or bypassed.
