# Execution and Scheduling

IC10 execution is tick-based. A chip runs a bounded number of instructions in a
world tick, or stops earlier when it executes `yield`, turns off, halts, sleeps,
or faults.

## Per-Chip Budget

The practical IC10 budget is about 128 instructions per chip per world tick.
This budget is not global. If eight active IC10 housings each burn a full slice,
the world has run roughly eight times one chip's budget during that tick.

StationC exploits that fact by spreading work across workers. The compiler and
runtime should still assume that each worker is a slow cooperative actor, not a
preemptive CPU thread.

## Inter-Chip Order

Public behavior is best treated as sequential IC10 slices in an unspecified
order:

```text
tick N:
    IC A runs up to its stopping point
    IC B runs up to its stopping point
    IC C runs up to its stopping point
```

The exact order is not a StationC contract. It may be stable in one save, change
after reload, change after network edits, or change after a game update.

StationC therefore treats any dependency on inter-chip order as a bug.

## What Is Defined

Inside one IC10 program, instruction order is defined:

```ic10
move r0 1
add r0 r0 1
yield
```

The `add` sees the value written by `move`.

Across IC10 programs, order is not defined:

```text
IC A writes stack[0]
IC B reads stack[0]
```

If both operations can happen in the same world tick without a protocol, the
result depends on scheduler order and is outside the defined StationC model.

## Tick Boundaries

Use `yield` or StationC `sleep_ticks` to make a world boundary explicit. This is
especially important around real devices, because many device consequences are
only visible after the world has advanced.

The safe control shape is:

```text
tick N:
    sense world state
    compute desired actions
tick N+1:
    apply actions
tick N+2:
    observe consequences
```
