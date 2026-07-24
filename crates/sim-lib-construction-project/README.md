# sim-lib-construction-project

Construction project-control records for SIM.

The crate defines stable project and role identifiers, charter records,
append-only fact books, deterministic as-of snapshots, change reports, evidence
states, and readiness summaries. It stores references to source systems through
`sim-lib-doc-core` rather than copying external project content. The embedded
recipes use modeled project data and describe an accepted charter plus a stable
what-changed report.
