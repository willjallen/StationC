# Shared State and Synchronization

IC10 devices and housings expose stack memory. StationC uses those stacks to
build ROM banks, RAM banks, mailboxes, worker result buffers, and scheduler
state.

That does not make the world a normal shared-memory machine.

## Defined Shared Access

Defined StationC shared access requires one of these patterns:

| Pattern | Rule |
| --- | --- |
| Exclusive result slot | One worker owns one output range for one job. |
| Host-owned state | Only the host writes scheduler fields. |
| Command buffer | Workers append or fill assigned command slots; a later apply phase writes devices. |
| Mailbox sequence | Producer and consumer use explicit state and sequence fields. |

The MVP should prefer exclusive result slots and host-owned state.

## Undefined Shared Access

These are outside the defined StationC model:

```text
two workers write the same RAM cell in one tick
worker A reads a cell while worker B writes it in the same tick
two workers implement a lock with read-then-write and no atomic primitive
two workers write the same device field without a command-buffer protocol
```

The problem is not just that the result is hard to predict. The problem is that
StationC has no contract for which IC10 slice runs first.

## Mailbox Shape

A safe mailbox should have enough metadata to detect stale or partial messages:

```text
state
sequence
sender
payload[0]
payload[1]
...
```

A writer fills payload first, then publishes state and sequence. A reader checks
state and sequence before consuming. The exact layout belongs to the StationOS
ABI, but the rule is stable: payload and ownership transitions must be explicit.

## Device Writes

Device writes should usually be command-buffered:

```text
sense phase:
    workers read devices
compute phase:
    workers write desired actions into assigned command slots
apply phase:
    one owner applies device writes deterministically
```

This avoids hidden order dependence between workers.
