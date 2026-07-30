# sim-construction

`sim-construction` holds SIM's construction project-control domain libraries.
The crates contribute construction project identities, charter records,
append-only project fact books, deterministic as-of snapshots, schedule exchange,
reference-only evidence links, and readiness summaries without adding
construction policy to the kernel.

The repository keeps public examples synthetic by default. Committed fixtures
name modeled projects, documents, roles, and evidence references only.

## Crates

- `sim-codec-mspdi`: Microsoft Project XML schedule exchange for portable
  construction Gantt documents.
- `sim-lib-construction-office`: reference-only evidence composition between
  construction facts and existing office documents.
- `sim-lib-construction-project`: construction project identities, charter
  records, append-only fact books, deterministic snapshots, change reports,
  evidence states, commissioning/handover hierarchy, accountable acceptance
  gates, readiness summaries, and a loadable Citizen/Shape operation surface.
- `sim-site-dalux`: Dalux project-item placement for construction evidence,
  modeled by default and gated for live API-identity use.
- `sim-site-powerproject`: Powerproject desktop and Project for the web
  placements for construction Gantt schedules.

## Validation

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
cargo run -p xtask -- simdoc --check
cargo run -p xtask -- check-file-sizes
```
