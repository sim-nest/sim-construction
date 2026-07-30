# Modeled reference project control

This is the one primary construction-control recipe. It composes the standard
runtime loader, `sim/construction-project`, the construction office bridge, an
injected `table/hash` backend, the existing Gantt model, MSPDI exchange, and
modeled Powerproject and Dalux sites. Stable project, fact, evidence, schedule
task, vendor, office, effect-ledger, and accounting-reference ids connect the
components without copying an external payload or calling an implementation
helper.

`main.siml` is an inspectable Lisp composition plan over the public operations.
The checked harness executes the corresponding public Rust library surfaces,
round-trips one Gantt plan through MSPDI and a modeled Powerproject receipt,
reads a modeled Dalux item without a network capability, persists facts through
the injected Table, rebuilds historical and changed-since views, and projects a
weekly `Doc`/`Sheet`/`Deck` office pack. Its semantic summary is byte-stable.

Modeled placement is the only default. The recipe grants no `net-connect`,
`process-spawn`, or `credentials` capability and carries no endpoint, bridge
path, credential, or live response. A host may configure a live placement only
outside this recipe, with the existing site capabilities and host-owned
credentials. Powerproject desktop additionally requires its installed bridge;
Dalux additionally requires `SIM_CONSTRUCTION_LIVE_DALUX=1`. The construction
libraries own neither transport nor secret handling.
