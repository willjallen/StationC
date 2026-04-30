# IC10 Specification Overview

This page set is an ordered IC10 reference, starting with the execution model and
ending with instruction tables, device variables, and common patterns.

IC10 is the in-game scripting language used by Stationeers IC10 chips. It is
inspired by MIPS assembly, but it is not a real MIPS CPU. IC10 scripts run on
chips made at the Electronics Printer and installed in compatible devices or IC
housings.

## Reading Order

Read the pages in numeric order:

| Page | Topic |
| --- | --- |
| [`0-overview.md`](0-overview.md) | What IC10 is, script shape, and notation used in the docs. |
| [`1-program-state.md`](1-program-state.md) | Registers, aliases, constants, labels, numeric values, and indirect references. |
| [`2-device-io.md`](2-device-io.md) | Device pins, load/store, batch I/O, direct references, and cable network channels. |
| [`3-control-flow-and-stack.md`](3-control-flow-and-stack.md) | Stack memory, jumps, branches, calls, returns, and execution pacing. |
| [`4-instruction-reference-core.md`](4-instruction-reference-core.md) | Utility, math, trigonometry, stack, device, slot, reagent, and batch instructions. |
| [`5-instruction-reference-conditions.md`](5-instruction-reference-conditions.md) | Bitwise, comparison, selection, and branching instructions. |
| [`6-device-and-slot-variables.md`](6-device-and-slot-variables.md) | Device logic variables, data network colors, slot variables, and filter values. |
| [`7-patterns-and-examples.md`](7-patterns-and-examples.md) | Beginner patterns, debugging tricks, Schmitt triggers, call nesting, and examples. |
| [`8-simulator-support.md`](8-simulator-support.md) | StationC simulator coverage for documented IC10 features. |

## Script Model

IC10 is line-oriented. A script is a list of instructions, labels, and comments.
Most instruction operands are numbers, registers, device references, labels, or
logic variable names.

```ic10
alias sensor d0
alias temperature r0

start:
yield
l temperature sensor Temperature
s db Setting temperature
j start
```

The example:

- Gives readable names to device pin `d0` and register `r0`.
- Defines a label named `start`.
- Pauses for one game tick with `yield`.
- Loads the `Temperature` logic value from the device on `d0`.
- Stores that value to the mounted device's `Setting` value through `db`.
- Jumps back to `start`.

## Core Pieces

IC10 programs are built around a few small concepts:

| Concept | Meaning |
| --- | --- |
| Internal registers | `r0` through `r15`; calculations happen here. |
| Device registers | `d0` through `d5`; configured by IC housing screws. |
| Mounted device | `db`; the device the IC chip is installed in. |
| Return address | `ra`; used by call-like jumps and branches. |
| Stack pointer | `sp`; points at the next stack index for push/pop operations. |
| Stack | 512 numeric values stored per IC chip, and on some devices. |
| Labels | Named line positions for jumps and branches. |
| Logic types | Device variables such as `On`, `Open`, `Temperature`, and `Pressure`. |
| Slot logic types | Slot variables such as `Occupied`, `Quantity`, `Charge`, and `Mature`. |

## Instruction Shape

IC10 usually puts the destination first.

```ic10
move r0 10        # r0 = 10
add r1 r0 3      # r1 = r0 + 3
l r2 d0 Pressure # r2 = d0.Pressure
s d1 On r1       # d1.On = r1
```

These pages use the following operand conventions:

| Notation | Meaning |
| --- | --- |
| `r?` | Any internal register such as `r0`, `r1`, or `r15`. |
| `d?` | Any device register such as `d0`, `d1`, `d5`, or `db`. |
| `a(r?\|num)` | Operand `a` may be a register or a numeric value. |
| `int` | An integer line number, label, or relative line count depending on instruction. |
| `logicType` | A device logic variable such as `Temperature`, `On`, or `ReferenceId`. |
| `logicSlotType` | A slot logic variable such as `Occupied`, `Charge`, or `Mature`. |
| `deviceHash` | A prefab hash, often produced by `HASH("PrefabName")`. |
| `nameHash` | A device name hash, often produced by `HASH("Device Name")`. |
| `id` | A device `ReferenceId`. |

## Comments

`#` starts a comment. The game ignores everything from `#` to the end of that
line.

```ic10
alias MyAlias r0 # This comment is ignored.
# A whole-line comment is valid too.
```

## Execution Pacing

`yield` pauses execution for one tick. If a script does not yield, the IC still
does not run forever in a single tick. A practical execution budget is about 128
script lines before the IC pauses for a tick.

That budget is per IC housing, not a global budget shared by every IC in the
world. Multiple ICs can therefore execute more total IC10 instructions per tick
than one IC. Do not rely on the order in which different IC housings run. Treat
inter-IC order as unspecified, even if it appears stable in one save.

Most useful IC10 scripts are loops:

```ic10
start:
yield
# read inputs
# compute outputs
# write outputs
j start
```

Without a loop, a script runs through once and stops.

## Stationpedia Dependency

IC10 does not know what you meant a device pin to be. If `d0` is configured to
the wrong thing, or if a selected device does not support a requested logic
type, the script will fail at runtime. Use the in-game Stationpedia and the
configuration cartridge to inspect device logic values, slot values, prefab
hashes, and connection numbers.

## Minimal Mental Model

1. Put external values into registers with `l`, `ls`, `lb`, or related load
   instructions.
2. Compute only through internal registers.
3. Write results back to devices with `s`, `ss`, `sb`, or related store
   instructions.
4. Use labels and branches for loops and decisions.
5. Use aliases to keep scripts readable, but remember aliases do not configure
   IC housing screws.
