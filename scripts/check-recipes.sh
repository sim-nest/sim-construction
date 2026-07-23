#!/usr/bin/env sh
set -eu

cargo test -p sim-lib-construction-project recipes_export_project_charter --quiet
printf 'check-recipes: OK (1 construction recipe)\n'
