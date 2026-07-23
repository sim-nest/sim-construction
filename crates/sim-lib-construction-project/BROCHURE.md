# sim-lib-construction-project

In one line: the construction charter spine that keeps project intent tied to evidence.

## What it gives you

`sim-lib-construction-project` gives a construction team a narrow starting
record: project identity, customer intent, delivery model, currency, acceptance,
and source references. The readiness result says exactly which charter facts are
present and which are still missing.

## Why you will be glad

- A charter becomes something software can check instead of a loose note.
- Evidence stays as references to outside systems, so private documents stay
  outside the codebase.
- The same record can feed schedule, office, ledger, and handover layers without
  changing their ownership.

## Where it fits

This crate is the first construction-control layer. It owns construction project
charter vocabulary and leaves documents, Gantt schedules, ledgers, tables, and
runtime loading with their existing SIM owners.
