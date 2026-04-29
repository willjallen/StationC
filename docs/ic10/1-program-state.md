# Program State

IC10 state is small: internal registers, device references, special registers,
labels/constants, numeric values, and optional stack memory. This page covers the
state that every other instruction works against.

## Internal Registers

The IC has sixteen general internal registers:

```text
r0 r1 r2 r3 r4 r5 r6 r7 r8 r9 r10 r11 r12 r13 r14 r15
```

These docs write any internal register as `r?`. Arithmetic and comparisons
operate on internal registers or literal numbers. Device values must first be
loaded into an internal register before normal calculations can use them.

```ic10
move r0 2      # Store 2 in r0.
add r1 r0 3   # Store r0 + 3 in r1.
```

## Device Registers

Device registers refer to external devices:

| Register | Meaning |
| --- | --- |
| `d0` through `d5` | The six IC housing device pins selected with the screwdriver. |
| `db` | The device the IC chip is mounted in. |

`db` is useful when a chip is inserted directly into a device such as an
atmospheric device and no separate IC housing is needed.

Device registers are only references. The IC is not aware of your intended
wiring. If a script reads `d0 Temperature`, but `d0` is not assigned or the
assigned device has no `Temperature` logic value, the script will error.

## Special Registers

IC10 has two special registers:

| Register | Name | Purpose |
| --- | --- | --- |
| `ra` | Return address | Stores the next line number after `jal` and branch instructions ending in `al`. |
| `sp` | Stack pointer | Tracks the next stack index used by `push`, `pop`, and `peek`. |

Both are writable. IC10 does not protect them from ordinary instructions.

```ic10
jal subroutine # stores the next line number in ra, then jumps
j ra           # returns to the stored line
```

`sp` is also just a register. You can read or write it directly, which is useful
for stack traversal but dangerous if done accidentally.

## Aliases

`alias` gives a readable name to a register or device reference.

```ic10
alias sensor d0
alias temperature r0

l temperature sensor Temperature
```

For registers, aliases are readability only. For device references, aliases also
affect the names shown on the IC base screws. An alias still does not configure
the screw; the player must still select the actual device.

Prefer aliases once a script has more than a few lines. Compare:

```ic10
l r0 d0 Temperature
s d1 On r0
```

with:

```ic10
alias sensor d0
alias cooler d1
alias temperature r0

l temperature sensor Temperature
s cooler On temperature
```

## Constants

`define` creates a name that is replaced with a numeric value throughout the
program.

```ic10
define pi 3.14159
move r0 pi
```

Constants are useful for values such as temperature thresholds, prefab hashes,
and mode numbers.

```ic10
define minimumTemperature 293.15
define maximumTemperature 298.15
```

`HASH("Name")` is commonly used where a prefab hash or name hash is required.
Hashes are CRC-32 checksums computed from the represented strings.

```ic10
lb r0 HASH("StructureBattery") Ratio Average
```

## Labels

A label names a script line. Branch and jump instructions can target labels
instead of raw line numbers.

```ic10
main:
yield
j main
```

Although a label has the numeric value of its line number, do not use label
values for calculations. Inserting or deleting lines changes those values.

Use unique label names. Do not name labels after IC10 keywords or logic values
such as `Temperature:` or `Setting:` because that overwrites the keyword's
normal meaning in the script.

## Numeric Values

Registers and constants are usually decimal values backed by double-precision
floating point. IC10 does not expose a separate integer type. Integer-looking
values are still numeric values in the same register model.

Practical consequences:

- Decimal literals are written normally: `12345`, `123.456`, `-10`.
- Hexadecimal literals are prefixed with `$`, for example `$E1B2`.
- Hex values are often used for `ReferenceId` values.
- Very large integer values can lose precision once they exceed the safe range
  of double-precision representation.

```ic10
move r0 12345
move r1 123.456
move r2 $E1B2
```

IC10 also uses these special numeric constants:

| Constant | Meaning |
| --- | --- |
| `nan` | Not-a-number. Cable network channels default to this before being written. |
| `pinf` | Positive infinity. |
| `ninf` | Negative infinity. |

## Binary and Bitwise Values

Binary notation is prefixed with `%`. Underscores are ignored and can be used as
visual separators.

```ic10
move r0 %0000_1111
```

Bitwise instructions operate over 64-bit integer representations, including the
sign bit. The integer range is `2^63 - 1` through `-2^63`, and negative values
follow two's-complement behavior.

```ic10
not r0 0 # r0 becomes -1, not logical true
```

For boolean logic, `seqz`, `min`, `max`, and `sne` are often better choices
than bitwise instructions. The details are in
[`5-instruction-reference-conditions.md`](5-instruction-reference-conditions.md).

## Indirect Register References

Adding another `r` in front of a register name makes IC10 treat the register's
value as a register index.

```ic10
move r0 5
move rr0 10
```

Because `r0` contains `5`, `rr0` points at `r5`, so the second instruction is
equivalent to:

```ic10
move r5 10
```

The pointer value must be between `0` and `15`, because there are only sixteen
internal registers.

Indirect references can be chained:

```ic10
move r1 2
move r2 3
move rrr1 4 # r1 -> r2 -> r3, so this stores 4 in r3
```

## Indirect Device References

The same idea works for device references.

```ic10
move r0 2
s dr0 On 1
```

Since `r0` contains `2`, `dr0` points at `d2`, making the store equivalent to:

```ic10
s d2 On 1
```

This is useful for loops that scan device pins, but it makes scripts harder to
debug. Use it when a loop genuinely needs it.

## Dynamic Logic Types

Logic types such as `Temperature` or `RatioOxygen` are enum-like numeric values.
Most scripts hardcode them, but their numeric values can also be used to select
logic types dynamically.

One pattern is to push desired logic types on the stack and pop them in a loop:

```ic10
push LogicType.RatioOxygen
push LogicType.RatioVolatiles
push LogicType.Temperature

loop:
pop r1
l r0 myDevice r1
# use r0 here
bgtz sp loop
```

Another pattern is to start with `LogicType.Channel0`, increment the value, and
read through `LogicType.Channel7`.

## Boolean Conventions

Many device values use `1` for true/on/open and `0` for false/off/closed. When a
device expects a binary value, values `>= 1` are treated as `1`, and values
`< 1` are treated as `0`.

Common boolean-like logic values include:

| Value | Common meaning |
| --- | --- |
| `On` | `1` is on, `0` is off. |
| `Open` | `1` is open, `0` is closed. |
| `Activate` | Often `1` when running or active. |
| `Error` | `1` when the device is in an error state. |
