# Simulator Policy

The world simulator is a development tool. It does not need to perfectly
recreate every private Stationeers implementation detail. It does need to make
unsafe assumptions visible.

## Baseline Schedule

The default simulator schedule should be deterministic and easy to debug:

```text
run active IC10 housings in stable world order
```

That is useful for tests and traces, but compiler correctness must not depend on
that order.

## Schedule Variation

The simulator should also support deterministic alternate schedules:

| Mode | Purpose |
| --- | --- |
| Stable | Reproducible baseline. |
| Rotating | Exposes assumptions about the first housing always running first. |
| Seeded shuffle | Exposes order-sensitive bugs while remaining reproducible. |

These modes are not claims about the game's exact implementation. They are
adversarial testing tools.

## Access Trace

The simulator should record world-facing accesses:

```text
tick
actor IC ReferenceId
operation: read or write
target: device logic field or device stack address
```

The trace should be available to tests and later to StationOS compiler
diagnostics.

## Diagnostics

Debug diagnostics should flag:

- multiple writes to the same target in one tick,
- read/write overlap on the same target in one tick,
- repeated volatile reads of the same logic field by the same IC in one tick.

Diagnostics are not the same as hard runtime errors. Repeated volatile reads can
be intentional. The diagnostic means the program or compiler should make its
sampling decision explicit.
