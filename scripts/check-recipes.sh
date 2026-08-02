#!/usr/bin/env sh
set -eu

cargo test -p sim-lib-construction-project recipes_export_project_charter --quiet
cargo test -p sim-site-dalux --test reference_project_control --quiet
printf 'check-recipes: OK (embedded construction recipes + 1 primary recipe)\n'
