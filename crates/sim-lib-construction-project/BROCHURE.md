# sim-lib-construction-project

In one line: the construction control spine that keeps project facts tied to evidence.

## What it gives you

`sim-lib-construction-project` gives a construction team a narrow control record:
project identity, customer intent, delivery model, currency, acceptance, changes,
and source references. It can replay the project as it looked at a chosen point
and say exactly what changed between two points.

## Why you will be glad

- A charter becomes something software can check instead of a loose note.
- Evidence stays as references to outside systems, so private documents stay
  outside the codebase.
- Corrections keep the earlier record visible, so reports do not silently rewrite
  what the team knew.
- The same record can feed schedule, office, ledger, and handover layers without
  changing their ownership.

## Where it fits

This crate is the construction-control layer for project facts. It owns project
charter, fact, snapshot, and change vocabulary while documents, Gantt schedules,
ledgers, tables, and runtime loading stay with their existing SIM owners.
