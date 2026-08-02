# sim-lib-construction-office

Reference-only office evidence composition for construction project control.

This crate joins a construction project, control, and fact sequence to an
existing office document and external reference. It reuses the office evidence
store and its broad link roles while the construction fact keeps the precise
semantic relation and acceptance state.

The bridge applies project visibility and capability checks before resolving a
link. It stores no document bodies, credentials, service responses, or vendor
tokens.

`project_office_pack` projects an explicit role-cadence control selection from
one historical or current `ProjectBook` snapshot into the existing office
`Doc`, `Sheet`, and `Deck` values. Facts are capability-filtered before snapshot
derivation, every pack carries reproducibility and provenance metadata, and
`OfficePack::suite_scene` uses the installed suite surface. Daily, weekly,
monthly/gate, handover, closeout, and reference-review horizons share one
safety-first priority order without adding a reporting store or document model.

`construction-office-evidence` is the shortest checked recipe. It shows the
project-scoped pointer written to the office evidence store and the precise
relation, original external reference, and acceptance state resolved from the
construction fact.

`project-chief-weekly-pack` shows a deterministic weekly pack with an
unaccepted mandatory item, changed-since-meeting controls, and the complete
visibility/provenance header.
