# World Model Overview

This layer documents behavior above raw IC10 instructions. IC10 describes what
one chip can execute. The world model describes how many chips, device state,
world ticks, and StationC safety rules fit together.

The goal is not to claim that every internal Stationeers scheduler or device
detail is known. The goal is to define the assumptions StationC is allowed to
make and the assumptions it must avoid.

Device behavior in the world simulator is scenario-defined. A simulated device
exposes the logic fields, slots, stack contents, and tick behavior declared by a
test scenario. The world simulator is not a reimplementation of Stationeers'
atmospherics, power, manufacturing, agriculture, or other device simulations.

## Pages

| Page | Topic |
| --- | --- |
| [`1-execution-and-scheduling.md`](1-execution-and-scheduling.md) | IC10 tick slices, per-chip budgets, and unspecified inter-chip order. |
| [`2-world-state-and-volatility.md`](2-world-state-and-volatility.md) | Device reads, volatile observations, and coherent samples. |
| [`3-shared-state-and-synchronization.md`](3-shared-state-and-synchronization.md) | Stack memory, mailboxes, command buffers, and unsafe shared access. |
| [`4-simulator-policy.md`](4-simulator-policy.md) | Deterministic simulation, schedule variation, access traces, and diagnostics. |

## Working Model

Each active IC10 housing gets its own execution budget each world tick. A useful
mental model is:

```text
for each world tick:
    for each active IC10 housing in some internal order:
        run until yield, sleep, halt, error, disabled, or budget exhaustion
    advance scenario-defined device state
```

The per-chip budget is the source of useful parallel throughput. Multiple IC10
chips can do more total work per tick than one IC10 chip, because the budget is
per chip rather than global.

That does not imply a normal shared-memory thread model. StationC must treat the
global order between IC10 housings as unspecified.

## StationC Rule

StationC code may rely on instruction order inside one IC10 program. StationC
code may not rely on one IC10 housing running before another unless the runtime
creates an explicit protocol that makes the order irrelevant.

Good designs use:

- exclusive result slots,
- command buffers,
- host-owned scheduler state,
- sequence-numbered mailboxes,
- tick-phased read/compute/apply loops.

Bad designs rely on placement order, network order, or two IC10 chips racing to
the same memory cell.
