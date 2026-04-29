# Control Flow and Stack

IC10 control flow is built from labels, jumps, branches, the return address
register `ra`, and the stack pointer `sp`. The stack also provides persistent
per-chip storage.

## Labels and Jumps

Labels name script lines:

```ic10
main:
j main
```

`j` jumps to an absolute line number or label.

```ic10
j 0
j main
```

`jr` performs a relative jump by a line count.

```ic10
jr -2
```

`jal` jumps and stores the next line number in `ra`.

```ic10
jal average
j ra
```

That gives IC10 a simple call/return pattern:

```ic10
start:
jal average
s db Setting r0
yield
j start

average:
add r0 r0 r1
div r0 r0 2
j ra
```

## Branches

Branch instructions are conditional jumps. The common shape is:

```text
b<condition> left right target
```

For example:

```ic10
beq r0 0 isZero
bgt r0 100 tooHigh
blt r0 10 tooLow
```

Zero-test branches omit one comparison operand:

```ic10
beqz r0 isZero
bnez r0 isNonZero
```

IC10 has these branch families:

| Family | Meaning |
| --- | --- |
| `b-` | Absolute branch to a line number or label. |
| `b-al` | Absolute branch and store the next line number in `ra`. |
| `br-` | Relative branch by a line count. |

Comparison instructions with the `s-` prefix do not branch. They store `1` or
`0` in a destination register.

```ic10
sgt r0 r1 100 # r0 = 1 if r1 > 100, otherwise 0
```

The full condition table is in
[`5-instruction-reference-conditions.md`](5-instruction-reference-conditions.md).

## Return Address Register

`ra` is overwritten by every `jal` and by every branch instruction ending in
`al`.

```ic10
jal firstFunction
# ra now points here
```

Because `ra` is just a register, nested calls must save and restore it.

```ic10
outer:
push ra
jal inner
pop ra
j ra

inner:
# work here
j ra
```

Without `push ra` and `pop ra`, the call to `inner` would overwrite the return
address needed by `outer`.

## Stack Memory

The stack can hold 512 numeric values. Each IC10 chip has its own stack, and some
devices, such as the Logical Sorter, also have stack memory.

The key stack instructions are:

| Instruction | Effect |
| --- | --- |
| `push value` | Store `value` at `sp`, then increment `sp`. |
| `pop register` | Load the value at `sp - 1` into `register`, then decrement `sp`. |
| `peek register` | Load the value at `sp - 1` without changing `sp`. |
| `poke address value` | Store `value` at a specific stack address on the current IC. |
| `get register device address` | Read stack memory from another device. |
| `put device address value` | Write stack memory on another device. |
| `getd register id address` | Read stack memory from a specific `ReferenceId`. |
| `putd id address value` | Write stack memory to a specific `ReferenceId`. |
| `clr device` | Clear stack memory on a device. |
| `clrd id` | Clear stack memory on a specific `ReferenceId`. |

`sp` may be read and written directly.

Stack pointer bounds:

| Operation | Required `sp` range |
| --- | --- |
| `push` | `0` through `511`, inclusive. |
| `peek` or `pop` | `1` through `512`, inclusive. |

## Stack Persistence

Stack memory is persistent on logic chips. If a script pushes values, then that
code is removed, the values remain on that chip.

That persistence does not automatically transfer to another chip that receives
the same program. Each chip's stack must be initialized individually.

If a script uses the stack for call frames or saved values, starting with a stack
clear can be useful:

```ic10
clr db
```

`clr db` can error if the chip is not inserted in an IC housing, such as when
inserted directly in an Air Conditioner.

## Traversing the Stack

You can traverse the stack by managing `sp` yourself.

```ic10
# Traverse indices {min value} through {max value} - 1.
move sp {min value}

loop:
add sp sp 1
peek r0
# use the value in r0
blt sp {max value} loop
```

You can also let `pop` decrement `sp`:

```ic10
move sp {max value}
add sp sp 1

loop:
pop r0
# use the value in r0
bgt sp {min value} loop
```

The second loop is more compact because `pop` handles the decrement.

## Yield, Sleep, and HCF

`yield` pauses execution for one tick.

```ic10
yield
```

`sleep` pauses execution for the given number of seconds.

```ic10
sleep 5
```

`hcf` means "halt and catch fire." Treat it as a hard stop/error tool, not as
normal control flow.

## Typical Loop Shape

The most common IC10 structure is:

```ic10
alias sensor d0
alias device d1
alias reading r0

start:
yield
l reading sensor Temperature
bgt reading 298.15 turnOff
blt reading 293.15 turnOn
j start

turnOn:
s device On 1
j start

turnOff:
s device On 0
j start
```

This is the Schmitt trigger pattern: turn on below a low threshold, turn off
above a high threshold, and leave the device unchanged between the thresholds.
