# Table-backed construction project book

Shows the durable boundary for one synthetic project book. A host selects and
constructs an existing `Table` or `Dir`, then injects it into
`ConstructionProjectLib::with_project_book`; construction does not name a
filesystem, database, memory, cloud, or transport backend.

Facts use canonical `TablePath` components and one serialized authoritative
writer. Reading sequence 1 rebuilds from the fact expression, not from the
projection. A partial, missing, stale, or content-mismatched projection is
discarded and regenerated, while the same damage to an authoritative fact
fails closed. The crate's conformance suite runs this contract unchanged
against `sim-table-hash` and persistent `sim-table-fs`.
