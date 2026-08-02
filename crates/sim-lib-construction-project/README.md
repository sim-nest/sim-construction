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

For durable books, the host constructs any existing runtime `Table` or `Dir`
and injects it with `ConstructionProjectLib::with_project_book` or
`ProjectBookRepository::new`. The library stores ordinary semantic expressions
under one version-neutral layout:

```text
projects/<ProjectId>/facts/<seq>
projects/<ProjectId>/baselines/<id>
projects/<ProjectId>/policies/<id>
projects/<ProjectId>/projections/<name>/<as-of>
```

Every component is validated by `TablePath`. Authoritative facts are contiguous
and have one externally serialized writer because the portable Table contract
does not promise compare-and-swap. Reads address only the repository's bound
project, rebuild from facts, and fail closed on an absent or corrupt fact.
Projections carry the fact stream's sequence and canonical content identity;
missing, partial, or stale projections are disposable and are regenerated.
Backend and construction capability errors remain unchanged.

The `loadable-project-control` Lisp recipe demonstrates standard bootloader
loading, fact construction and append, a historical snapshot, and blocker
explanation. `table-backed-project-book` demonstrates the injected persistence
boundary and exact layout. Vendor integrations remain independently placed through
`EvalFabric`; this crate adds no parser, matcher, loader, or vendor effect path.
