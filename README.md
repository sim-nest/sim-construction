# sim-construction

`sim-construction` holds SIM's construction project-control domain libraries.
The crates contribute construction project identities, charter records, schedule
exchange, reference-only evidence links, and deterministic readiness summaries
without adding construction policy to the kernel.

The repository keeps public examples synthetic by default. Committed fixtures
name modeled projects, documents, roles, and evidence references only.

## Crates

- `sim-codec-mspdi`: Microsoft Project XML schedule exchange for portable
  construction Gantt documents.
- `sim-lib-construction-project`: construction project identities, charter
  records, evidence states, and charter readiness summaries.

## Validation

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
cargo run -p xtask -- simdoc --check
cargo run -p xtask -- check-file-sizes
```
