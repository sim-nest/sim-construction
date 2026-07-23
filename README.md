# sim-construction

`sim-construction` holds SIM's construction project-control domain library. The
project crate contributes construction project identities, charter records,
reference-only evidence links, and deterministic readiness summaries without
adding construction policy to the kernel.

The repository keeps public examples synthetic by default. Committed fixtures
name modeled projects, documents, roles, and evidence references only.

## Crates

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
