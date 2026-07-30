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
`sim-ledger`.
