# sim-lib-construction-project

Construction project-control records for SIM.

The crate defines stable project and role identifiers, charter records,
append-only fact books, deterministic as-of snapshots, change reports, evidence
states, readiness summaries, and baseline-aware risk/opportunity exposure. It
stores references to source systems through `sim-lib-doc-core` rather than
copying external project content. Forecast consequences retain typed values,
named methods, and as-of bases; escalation outputs accountable attention
recommendations rather than decisions. Stable construction change chains keep
scope, time, exact supplier/customer amounts, authority, execution, settlement,
and closure together while leaving journals and payment truth with
`sim-ledger`. Graph-backed system, area, work-package, asset-group, and milestone
handover derives distinct technical, evidence, authority, contractual,
occupancy/use, and final completion gates from leaf commissioning evidence.

`ConstructionProjectLib` exposes the domain through normal SIM contracts. It
registers semantic Citizen read-constructs for durable records and reports,
Shapes for every public constructor and operation, and a thin
`sim/construction-project` host library. The operations cover append,
snapshot/as-of, validate, status, explain, diff-since, gate-report,
schedule-impact, readiness, exposure, handover burn-down, and reference
admission. Pure validation needs no capability; book reads and writes require
only their construction capabilities, and reference publication remains
separate.

The `loadable-project-control` Lisp recipe demonstrates standard bootloader
loading, fact construction and append, a historical snapshot, and blocker
explanation. Vendor integrations remain independently placed through
`EvalFabric`; this crate adds no parser, matcher, loader, or vendor effect path.
