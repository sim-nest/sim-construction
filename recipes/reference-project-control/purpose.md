# Modeled reference project control

This is the one primary construction-control recipe. Its entirely invented
Stockholm-region scenario follows the compact Nordhamn Market commercial
renovation from opportunity to an admitted reference. Invented organizations,
roles, address, project and document ids, SEK values, measurements, schedules,
and supplier responses make the scenario useful without carrying personal or
live service data.

The recipe composes the standard runtime loader,
`sim/construction-project`, the construction office bridge, an injected
`table/hash` backend, the existing Gantt model, MSPDI exchange, and modeled
Powerproject and Dalux sites. Stable project, fact, evidence, schedule task,
vendor, office, effect-ledger, and accounting-reference ids connect the
components without copying an external payload or calling an implementation
helper.

`main.siml` is an inspectable Lisp composition plan over the public operations.
The checked harness executes the corresponding public Rust library surfaces,
round-trips one Gantt plan through MSPDI and a modeled Powerproject receipt,
reads a modeled field item without a network capability, persists the complete
fact sequence through the injected Table, rebuilds every decision boundary and
changed-since view, and projects a weekly `Doc`/`Sheet`/`Deck` office pack. It
proves conflict, supersession, missing and expired evidence, late authority,
non-waivable safety, a time-bounded exception, critical schedule consequence,
partial approval without doubled exposure, defect correction, visibility
non-interference, accountable closeout, and final reference admission.

The harness runs the scenario twice. Canonical snapshots, explanations,
commercial exposure, office projections, and reference manifests must be
byte-identical, and the resulting semantic summary is checked against the
committed golden.

Modeled placement is the only default. The recipe grants no `net-connect`,
`process-spawn`, or `credentials` capability and carries no endpoint, bridge
path, credential, or live response. A host may configure a live placement only
outside this recipe, with the existing site capabilities and host-owned
credentials. Powerproject desktop additionally requires its installed bridge;
Dalux additionally requires `SIM_CONSTRUCTION_LIVE_DALUX=1`. The construction
libraries own neither transport nor authentication material.
