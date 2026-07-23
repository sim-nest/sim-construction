# sim-site-dalux

In one line: Dalux project items become construction evidence behind API identity gates.

## What it gives you

This crate gives construction project data a Dalux boundary that reads items
into local office documents and keeps live service access behind a bearer token
from an API identity. It gives hosts a small, named place to connect Dalux
without changing the construction or office document models.

## Why you will be glad

- Project item lists can be reviewed without a live Dalux account in tests.
- Note updates carry only the note text, not a broad edit payload.
- Service errors hide access tokens and long project names before they leave the adapter.

## Where it fits

It sits with the construction-owned vendor placements and uses the shared office
site boundary. Use it when a construction workflow needs Dalux project item
evidence or a narrow item note update.
