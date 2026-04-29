# Instruction Reference: Core Operations

This page covers utility, math, trigonometry, stack, device, slot, reagent, and
batch instructions. Conditional, bitwise, comparison, and branch instructions
are in [`5-instruction-reference-conditions.md`](5-instruction-reference-conditions.md).

## Utility

| Instruction | Description |
| --- | --- |
| `alias str r?|d?` | Labels a register or device reference with a name. Device aliases also affect IC base screw labels. |
| `define str num` | Creates a constant name that is replaced with the provided numeric value. |
| `hcf` | Halt and catch fire. |
| `sleep a(r?|num)` | Pauses execution on the IC for `a` seconds. |
| `yield` | Pauses execution for one tick. |

Examples:

```ic10
alias dAutoHydro1 d0
alias vTemperature r0

define ultimateAnswer 42
move r0 ultimateAnswer
```

## Mathematical

| Instruction | Description |
| --- | --- |
| `abs r? a(r?|num)` | Store the absolute value of `a`. |
| `add r? a(r?|num) b(r?|num)` | Store `a + b`. |
| `ceil r? a(r?|num)` | Store the smallest integer greater than `a`. |
| `div r? a(r?|num) b(r?|num)` | Store `a / b`. |
| `pow r? a(r?|num) b(r?|num)` | Store `a` raised to `b`, following IEEE-754 floating-point behavior. |
| `exp r? a(r?|num)` | Store `e^a`. |
| `floor r? a(r?|num)` | Store the largest integer less than `a`. |
| `log r? a(r?|num)` | Store the natural logarithm of `a`. |
| `max r? a(r?|num) b(r?|num)` | Store the larger of `a` and `b`. |
| `min r? a(r?|num) b(r?|num)` | Store the smaller of `a` and `b`. |
| `mod r? a(r?|num) b(r?|num)` | Store `a mod b`. This is not the same as C-style `%`. |
| `move r? a(r?|num)` | Store a number or register value into a register. |
| `mul r? a(r?|num) b(r?|num)` | Store `a * b`. |
| `rand r?` | Store a random value `x` where `0 <= x < 1`. |
| `round r? a(r?|num)` | Store `a` rounded to the nearest integer. |
| `sqrt r? a(r?|num)` | Store the square root of `a`. |
| `sub r? a(r?|num) b(r?|num)` | Store `a - b`. |
| `trunc r? a(r?|num)` | Store `a` with its fractional part removed. |
| `lerp r? a(r?|num) b(r?|num) c(r?|num)` | Linearly interpolate from `a` to `b` by ratio `c`, clamped to `0..1`. |

Examples:

```ic10
define negativeNumber -10
abs r0 negativeNumber

add r0 r0 1

define num1 10
define num2 20
add r0 num1 num2

define floatNumber 10.3
ceil r0 floatNumber

move r0 42
```

`mod` examples:

```ic10
mod r0 10 20  # r0 = 10
mod r1 22 20  # r1 = 2
mod r2 22 -20 # r2 = 18
mod r2 22 -10 # r2 = 18
mod r2 -7 4   # r2 = 1
mod r2 -7 9   # r2 = 2
```

## Mathematical / Trigonometric

All angles are in radians.

| Instruction | Description |
| --- | --- |
| `acos r? a(r?|num)` | Store the angle whose cosine is `a`. |
| `asin r? a(r?|num)` | Store the angle whose sine is `a`. |
| `atan r? a(r?|num)` | Store the angle whose tangent is `a`. |
| `atan2 r? a(r?|num) b(r?|num)` | Store the angle whose tangent is `a / b`, where `a` is y and `b` is x. |
| `cos r? a(r?|num)` | Store the cosine of angle `a`. |
| `sin r? a(r?|num)` | Store the sine of angle `a`. |
| `tan r? a(r?|num)` | Store the tangent of angle `a`. |

## Stack

| Instruction | Description |
| --- | --- |
| `clr d?` | Clear stack memory for the provided device. |
| `clrd id(r?|num)` | Clear stack memory for the device with the provided `ReferenceId`. |
| `get r? device(d?|r?|id) address(r?|num)` | Read stack memory at `address` from the provided device into a register. |
| `getd r? id(r?|id) address(r?|num)` | Read stack memory at `address` from the device with the provided `ReferenceId`. |
| `peek r?` | Store the value at the top of the current IC stack without decrementing `sp`. |
| `poke address(r?|num) value(r?|num)` | Store `value` at `address` in the current IC stack. |
| `pop r?` | Store the top stack value into a register and decrement `sp`. |
| `push a(r?|num)` | Push `a` to the current IC stack at `sp` and increment `sp`. |
| `put device(d?|r?|id) address(r?|num) value(r?|num)` | Write `value` to stack memory at `address` on the provided device. |
| `putd id(r?|id) address(r?|num) value(r?|num)` | Write `value` to stack memory at `address` on the device with the provided `ReferenceId`. |

## Slot / Logic

| Instruction | Description |
| --- | --- |
| `l r? device(d?|r?|id) logicType` | Load a device logic value into a register. |
| `lr r? device(d?|r?|id) reagentMode int` | Load a reagent value for `Contents` (`0`), `Required` (`1`), or `Recipe` (`2`). |
| `ls r? device(d?|r?|id) slotIndex logicSlotType` | Load a slot logic value into a register. |
| `s device(d?|r?|id) logicType r?` | Store a register value to a device logic value. |
| `ss device(d?|r?|id) slotIndex logicSlotType r?` | Store a register value to a slot logic value. |
| `rmap r? d? reagentHash(r?|num)` | Map a reagent hash to the prefab hash expected by the device. |

Examples:

```ic10
l r0 d0 Setting
l r1 d5 Pressure

alias Sensor d0
l r0 Sensor Temperature

ls r0 d0 2 Occupied

alias robot d0
alias charge r10
ls charge robot 0 Charge

s d0 Setting r0
```

## Direct ReferenceId Logic

`ld` and `sd` are direct reference instructions. `sd` can be used with a
`ReferenceId`:

```ic10
sd r1 Mode 1
```

Use direct reference access when a script has obtained a device's unique
`ReferenceId`, commonly by batch-reading the `ReferenceId` logic value from a
named device.

```ic10
lbn r1 HASH("StructureLogicSorter") HASH("Sorter Corn") ReferenceId Maximum
ble r1 ninf ra
sd r1 Mode 1
```

Slot access by direct reference ID is not available.

## Slot / Logic / Batched

Batch instructions address all matching devices on the output network by prefab
hash and, optionally, name hash.

| Instruction | Description |
| --- | --- |
| `lb r? deviceHash logicType batchMode` | Load a logic value from all matching prefab devices using `Average`, `Sum`, `Minimum`, or `Maximum`. |
| `lbn r? deviceHash nameHash logicType batchMode` | Load a logic value from matching prefab devices with a matching name hash. |
| `lbns r? deviceHash nameHash slotIndex logicSlotType batchMode` | Load a slot logic value from matching prefab devices with a matching name hash. |
| `lbs r? deviceHash slotIndex logicSlotType batchMode` | Load a slot logic value from matching prefab devices. |
| `sb deviceHash logicType r?` | Store a register value to a logic value on all matching prefab devices. |
| `sbn deviceHash nameHash logicType r?` | Store a register value to a logic value on matching prefab devices with a matching name hash. |
| `sbs deviceHash slotIndex logicSlotType r?` | Store a register value to a slot logic value on all matching prefab devices. |

Examples:

```ic10
lb r0 HASH("StructureWallLight") On Sum
sb HASH("StructureWallLight") On 1
```

Batch modes:

| Name | Number |
| --- | --- |
| `Average` | `0` |
| `Sum` | `1` |
| `Minimum` | `2` |
| `Maximum` | `3` |

The batch store instructions are `sb`, `sbn`, and `sbs`; there is no `sbns`.
