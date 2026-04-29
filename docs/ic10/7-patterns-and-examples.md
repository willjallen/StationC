# Patterns and Examples

This page collects IC10 examples, beginner guidance, debugging techniques, and
higher-level patterns.

## Beginner Instruction Set

Learn this small subset first:

| Category | Instructions and values |
| --- | --- |
| General | `alias`, labels, `yield`. |
| Jumps | `j label`, `jal label`, `j ra`. |
| Branching | `beq`, `bne`, `bgt`, `blt`, plus `-al` variants. |
| Device I/O | `l`, `lb`, `ls`, `s`, `sb`. |
| Logic/math | `seqz`, `move`, `add`, `sub`, `mul`, `div`. |
| Device variables | `On`, `Open`, `Setting`, `Activate`, `Temperature`, `Pressure`. |

In the in-game IC editor, the `f`, `x`, and `s(x)` buttons show available
instructions and variables.

`seqz` is the beginner-friendly logical NOT:

```ic10
seqz r0 r1 # r0 = 1 if r1 is 0, otherwise 0
```

## Basic Operations

Store a constant:

```ic10
move r0 10
```

Copy a register:

```ic10
move r0 r1
```

Read device temperature:

```ic10
l r0 d0 Temperature
```

Write a device state:

```ic10
s d0 On r0
```

Use aliases for readability:

```ic10
alias sensor d0
alias light d1
alias active r0

l active sensor Activate
seqz active active
s light On active
```

That script is the automatic night light pattern: read daylight sensor
activation, invert it, and write the result to one or more lights.

## Practice Scripts

Two beginner exercises:

Automatic night light:

1. Load `Activate` from a daylight sensor.
2. Flip the value with `seqz`.
3. Store the result to `On` for one or more lights.

Automatic wall cooler:

1. Read `Temperature` from a gas sensor.
2. If the value is greater than a high threshold, turn on the cooler.
3. If the value is less than a low threshold, turn off the cooler.
4. Leave the cooler unchanged between thresholds.

Wall coolers need at least `12.5 kPa` pressure in the connected pipe.

## Schmitt Trigger Pattern

A Schmitt trigger avoids rapid toggling by using two thresholds. The device turns
on below one threshold and turns off above another.

```ic10
alias sensor d0
alias device d1

define mintemp 293.15
define maxtemp 298.15

start:
yield
l r0 sensor Temperature
blt r0 mintemp turnOn
bgt r0 maxtemp turnOff
j start

turnOn:
s device On 1
j start

turnOff:
s device On 0
j start
```

For a cooler, the on/off meaning may be inverted depending on whether the device
should run above the high threshold or below the low threshold.

A `select`-based Schmitt trigger style can also work, where `select` acts like a
ternary conditional for range-based toggles.

## Debugging With IC Housing Setting

You can display a register by writing it to `Setting` on the IC housing:

```ic10
s db Setting r0
```

This has no side effects on a normal IC housing. An Air Conditioner is an
exception.

To check if a block of code executes, write a recognizable number:

```ic10
s db Setting 137
```

## Breakpoint-Like Debug Function

A simple debug subroutine:

```ic10
# ... some code ...
jal debug
# ... more code ...
jal debug
# ... rest of code ...

debug:
s db Setting ra # show stored line number
s db On 0       # stop execution; turn IC housing back on manually to proceed
j ra            # return
```

Because `jal` stores the next line number in `ra`, displaying `ra` shows where
the call came from.

## Batch and ReferenceId Discovery

Use batch reads when more than six devices need to be addressed or when devices
are identified by name rather than IC housing pins.

```ic10
# Average charge ratio across station batteries.
lb r0 HASH("StructureBattery") Ratio Average
```

Find one named device's `ReferenceId`, then use direct reference access:

```ic10
lbn r1 HASH("StructureLogicSorter") HASH("Sorter Corn") ReferenceId Maximum
ble r1 ninf ra
sd r1 Mode 1
```

Using `Maximum` pairs well with the no-device result of `ninf`.

## Harvie Automation Caveat

A Harvie automation design can batch-store to all Harvie devices on a network
while reading only one master Harvie and one master tray. Every Harvie repeats
the master's action.

The caveat is timing: crops mature at different speeds. If seeds are planted
manually and the master receives the first seed, the batch harvest can trigger
too early for slower plants.

## Solar Panel Tracking

For solar panel two-axis tracking, the relevant IC10 pieces are:

- Read `SolarAngle` or sensor values.
- Compute horizontal and vertical settings.
- Store `Horizontal`, `Vertical`, `HorizontalRatio`, or `VerticalRatio`,
  depending on the panel/device support.

Check the Stationpedia for the exact logic values exposed by the solar hardware
in the current game version.

## Lines Per Tick Experiment

This experiment infers how many lines execute per tick when a script does not
call `yield`:

```ic10
move r0 1
add r0 r0 3
s db Setting r0
j 1
```

Observed values:

```text
127
256 (+129)
385 (+129)
511 (+126)
640 (+129)
769 (+129)
895 (+126)
1024 (+129)
1153 (+129)
```

The repeating `+129`, `+129`, `+126` sequence suggests a practical execution
budget of about `128` lines per tick. Empty rows also count toward this number.

## Nested Function Calls

For one-level calls, `jal` and `j ra` are enough:

```ic10
jal function
j ra
```

For nested calls, save `ra` on the stack:

```ic10
orientPanelsToStar:
push ra

# calculate panel orientation into r0 and r1

jal orientPanelsTo

# more work could happen here

pop ra
j ra

orientPanelsTo:
# set panel orientation
j ra
```

Saving `ra` in a spare register only handles shallow cases. The stack scales to
the IC stack limit, which is 512 values.

## Common Failure Modes

| Symptom | Likely cause |
| --- | --- |
| Logic type error | Device pin points to the wrong device, is unset, or the device does not support that logic value. |
| Jump goes to the wrong place | Raw line numbers changed; use labels. |
| Return goes to the wrong place | Nested call overwrote `ra`; push/pop `ra`. |
| Batch read returns `nan` or `ninf` | No matching devices for that batch mode. |
| Bitwise NOT gives negative values | `not` is bitwise; use `seqz` for logical NOT. |
| Device screw labels look right but device is wrong | `alias` changed labels only; configure the IC housing screws. |

## Tooling Mentioned in the Dump

Useful IC10 emulators and syntax-highlighting resources:

| Reference | Description |
| --- | --- |
| `[1]` | Stationeers Code Simulator. |
| `[2]` | Stationeers IC10 Editor and Emulator with editor, debugger, emulator, and sharing. |
| `[3]` | Stationeering, an IC10 chip simulation with IDE, error checking, stack visibility, and register visibility. |
| `[4]` | EASy68K, a 68000 structured assembly language IDE. |
| `[5]` | IC10 MIPS syntax highlighting for Visual Studio Code, updated Feb 10, 2022. |
| `[6]` | IC10 MIPS syntax highlighting for KDE kwrite/kate. |
| `[7]` | IC10 MIPS syntax highlighting for Notepad++. |
| `[8]` | IC10 MIPS syntax highlighting for Notepad++, updated Nov 8, 2022. |
| `[9]` | IC10 MIPS syntax highlighting for Notepad++, updated Mar 23, 2024. |
| `[10]` | Repository with many code examples. |
