# Instruction Reference: Conditions, Bitwise Operations, and Branches

This page covers bitwise operations, boolean alternatives, comparisons,
selection, and all branch families.

## Bitwise Model

IC10 bitwise gates operate on the binary representation of values. They are not
logical gates over `0` and `1`.

Available bitwise gates:

```text
not and or xor nor
```

`nand` and `xnor` are not available as direct instructions.

Bitwise operations include the sign bit. Integer values are represented with 64
bits, and negative values follow two's-complement behavior.

```ic10
not r0 0
# r0 becomes -1

and r0 3 6
# 3 = %011
# 6 = %110
# r0 becomes %010, which is 2
```

Binary literals use `%`; underscores are ignored for readability.

```ic10
move r0 %0000_1111
```

## Bitwise Instructions

| Instruction | Description |
| --- | --- |
| `and r? a(r?|num) b(r?|num)` | Bitwise AND. Each result bit is `1` only if both input bits are `1`. |
| `nor r? a(r?|num) b(r?|num)` | Bitwise NOR. Each result bit is `1` only if both input bits are `0`. |
| `not r? a(r?|num)` | Bitwise NOT. Flips every bit. `not 1` is `-2`, not logical false. |
| `or r? a(r?|num) b(r?|num)` | Bitwise OR. Each result bit is `1` if either input bit is `1`. |
| `sla r? a(r?|num) b(r?|num)` | Arithmetic left shift. This is indistinguishable from `sll`. |
| `sll r? a(r?|num) b(r?|num)` | Logical left shift, filling rightmost bits with zero. |
| `sra r? a(r?|num) b(r?|num)` | Arithmetic right shift, filling leftmost bits with the sign bit. |
| `srl r? a(r?|num) b(r?|num)` | Logical right shift, filling leftmost bits with zero. |
| `xor r? a(r?|num) b(r?|num)` | Bitwise XOR. Each result bit is `1` when the input bits differ. |
| `ext r? source(r?|num) offset(r?|num) length(r?|num)` | Extract a bit field from `source` beginning at `offset` for `length` bits. Payload cannot exceed 53 bits. |
| `ins r? field(r?|num) offset(r?|num) length(r?|num)` | Insert a bit field into the destination register beginning at `offset` for `length` bits. Payload cannot exceed 53 bits. |

`ins` example:

```ic10
move r0 $DE0000EF
move r1 $ADBE
ins r0 r1 8 16 # r0 becomes $DEADBEEF
```

As of `2026-01-12`, the stable version has a bug in `ins` parameter order,
using `offset length field` instead of `field offset length`. The beta version
has the documented order.

## Logical Alternatives

For boolean-style logic, use other instructions instead of bitwise gates:

| Logical operation | IC10 pattern |
| --- | --- |
| NOT | `seqz r0 r1` |
| AND | `min r0 r1 r2` |
| OR | `max r0 r1 r2` |
| XOR | `sne r0 r1 r2`, for binary inputs only |

Derived patterns:

| Logical operation | Pattern |
| --- | --- |
| NAND | NOT of AND |
| NOR | NOT of OR |
| XNOR | NOT of XOR |

These are not perfect substitutes for all numeric inputs. Some produce
non-binary outputs, and negative values can behave differently. Devices that want
binary values treat `>= 1` as `1` and `< 1` as `0`.

## Selection

| Instruction | Description |
| --- | --- |
| `select r? a(r?|num) b(r?|num) c(r?|num)` | Store `b` if `a` is non-zero, otherwise store `c`. |

`select` is a ternary-like operation:

```ic10
move r0 0
select r1 r0 10 200 # r1 = 200

move r0 5
select r1 r0 10 200 # r1 = 10
```

## Device Pin Comparisons

| Instruction | Description |
| --- | --- |
| `sdns r? device(d?|r?|id)` | Store `1` if the device is not set, otherwise `0`. |
| `sdse r? device(d?|r?|id)` | Store `1` if the device is set, otherwise `0`. |

## Value Comparisons

Comparison instructions store `1` when the condition is true and `0` otherwise.

| Instruction | Condition |
| --- | --- |
| `sap r? a(r?|num) b(r?|num) c(r?|num)` | `1` if `abs(a - b) <= max(c * max(abs(a), abs(b)), float.epsilon * 8)`, otherwise `0`. |
| `sapz r? a(r?|num) b(r?|num)` | `1` if `abs(a) <= max(b * abs(a), float.epsilon * 8)`, otherwise `0`. |
| `seq r? a(r?|num) b(r?|num)` | `a == b`. |
| `seqz r? a(r?|num)` | `a == 0`. |
| `sge r? a(r?|num) b(r?|num)` | `a >= b`. |
| `sgez r? a(r?|num)` | `a >= 0`. |
| `sgt r? a(r?|num) b(r?|num)` | `a > b`. |
| `sgtz r? a(r?|num)` | `a > 0`. |
| `sle r? a(r?|num) b(r?|num)` | `a <= b`. |
| `slez r? a(r?|num)` | `a <= 0`. |
| `slt r? a(r?|num) b(r?|num)` | `a < b`. |
| `sltz r? a(r?|num)` | `a < 0`. |
| `sna r? a(r?|num) b(r?|num) c(r?|num)` | `1` if `abs(a - b) > max(c * max(abs(a), abs(b)), float.epsilon * 8)`, otherwise `0`. |
| `snan r? a(r?|num)` | `a` is `NaN`. |
| `snanz r? a(r?|num)` | `a` is not `NaN`. |
| `snaz r? a(r?|num) b(r?|num)` | `1` if `abs(a) > max(b * abs(a), float.epsilon)`, otherwise `0`. |
| `sne r? a(r?|num) b(r?|num)` | `a != b`. |
| `snez r? a(r?|num)` | `a != 0`. |

The `ap`/`na` approximate comparison pair uses this formula:

```text
abs(a - b) <= max(c * max(abs(a), abs(b)), float.epsilon * 8)
```

This is comparable to Python's `math.isclose`.

Example:

```ic10
sap r0 100 101 0.01 # true because 100 and 101 differ by no more than 1%
```

## Unconditional Branching

| Instruction | Description |
| --- | --- |
| `j int` | Jump execution to an absolute line number or label. |
| `jal int` | Jump to an absolute line number or label and store the next line number in `ra`. |
| `jr int` | Relative jump by line count. |

Examples:

```ic10
j 0
j label

label:
# code here
```

```ic10
jal average
j ra
```

## Device Pin Branching

| Instruction | Description |
| --- | --- |
| `bdnvl device(d?|r?|id) logicType a(r?|num)` | Branch to `a` if the device is not valid for loading the logic type. |
| `bdnvs device(d?|r?|id) logicType a(r?|num)` | Branch to `a` if the device is not valid for storing the logic type. |
| `bdns d? a(r?|num)` | Branch to `a` if device `d` is not set. |
| `bdnsal d? a(r?|num)` | Branch to `a` if device `d` is not set and store next line in `ra`. |
| `bdse d? a(r?|num)` | Branch to `a` if device `d` is set. |
| `bdseal d? a(r?|num)` | Branch to `a` if device `d` is set and store next line in `ra`. |
| `brdns d? a(r?|num)` | Relative branch to `a` if device `d` is not set. |
| `brdse d? a(r?|num)` | Relative branch to `a` if device `d` is set. |

Example:

```ic10
bdseal d0 32
bdseal d0 HarvestCrop
```

## Comparison Branching

| Instruction | Description |
| --- | --- |
| `bap a(r?|num) b(r?|num) c(r?|num) d(r?|num)` | Branch to `d` if `a` approximately equals `b` using tolerance `c`. |
| `brap a(r?|num) b(r?|num) c(r?|num) d(r?|num)` | Relative branch to `d` if `a` approximately equals `b` using tolerance `c`. |
| `bapal a(r?|num) b(r?|num) c(r?|num) d(r?|num)` | Branch and link for approximate equality. |
| `bapz a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a` approximately equals zero using tolerance `b`. |
| `brapz a(r?|num) b(r?|num) c(r?|num)` | Relative branch to `c` if `a` approximately equals zero using tolerance `b`. |
| `bapzal a(r?|num) b(r?|num) c(r?|num)` | Branch and link if `a` approximately equals zero using tolerance `b`. |
| `beq a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a == b`. |
| `breq a(r?|num) b(r?|num) c(r?|num)` | Relative branch to `c` if `a == b`. |
| `beqal a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a == b` and store next line in `ra`. |
| `beqz a(r?|num) b(r?|num)` | Branch to `b` if `a == 0`. |
| `breqz a(r?|num) b(r?|num)` | Relative branch to `b` if `a == 0`. |
| `beqzal a(r?|num) b(r?|num)` | Branch to `b` if `a == 0` and store next line in `ra`. |
| `bge a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a >= b`. |
| `brge a(r?|num) b(r?|num) c(r?|num)` | Relative branch to `c` if `a >= b`. |
| `bgeal a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a >= b` and store next line in `ra`. |
| `bgez a(r?|num) b(r?|num)` | Branch to `b` if `a >= 0`. |
| `brgez a(r?|num) b(r?|num)` | Relative branch to `b` if `a >= 0`. |
| `bgezal a(r?|num) b(r?|num)` | Branch to `b` if `a >= 0` and store next line in `ra`. |
| `bgt a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a > b`. |
| `brgt a(r?|num) b(r?|num) c(r?|num)` | Relative branch to `c` if `a > b`. |
| `bgtal a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a > b` and store next line in `ra`. |
| `bgtz a(r?|num) b(r?|num)` | Branch to `b` if `a > 0`. |
| `brgtz a(r?|num) b(r?|num)` | Relative branch to `b` if `a > 0`. |
| `bgtzal a(r?|num) b(r?|num)` | Branch to `b` if `a > 0` and store next line in `ra`. |
| `ble a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a <= b`. |
| `brle a(r?|num) b(r?|num) c(r?|num)` | Relative branch to `c` if `a <= b`. |
| `bleal a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a <= b` and store next line in `ra`. |
| `blez a(r?|num) b(r?|num)` | Branch to `b` if `a <= 0`. |
| `brlez a(r?|num) b(r?|num)` | Relative branch to `b` if `a <= 0`. |
| `blezal a(r?|num) b(r?|num)` | Branch to `b` if `a <= 0` and store next line in `ra`. |
| `blt a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a < b`. |
| `brlt a(r?|num) b(r?|num) c(r?|num)` | Relative branch to `c` if `a < b`. |
| `bltal a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a < b` and store next line in `ra`. |
| `bltz a(r?|num) b(r?|num)` | Branch to `b` if `a < 0`. |
| `brltz a(r?|num) b(r?|num)` | Relative branch to `b` if `a < 0`. |
| `bltzal a(r?|num) b(r?|num)` | Branch to `b` if `a < 0` and store next line in `ra`. |
| `bna a(r?|num) b(r?|num) c(r?|num) d(r?|num)` | Branch to `d` if `a` is not approximately equal to `b` using tolerance `c`. |
| `brna a(r?|num) b(r?|num) c(r?|num) d(r?|num)` | Relative branch to `d` if `a` is not approximately equal to `b` using tolerance `c`. |
| `bnaal a(r?|num) b(r?|num) c(r?|num) d(r?|num)` | Branch and link for not-approximately-equal. |
| `bnan a(r?|num) b(r?|num)` | Branch to `b` if `a` is `NaN`. |
| `brnan a(r?|num) b(r?|num)` | Relative branch to `b` if `a` is `NaN`. |
| `bnaz a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a` is not approximately zero using tolerance `b`. |
| `brnaz a(r?|num) b(r?|num) c(r?|num)` | Relative branch to `c` if `a` is not approximately zero using tolerance `b`. |
| `bnazal a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a` is not approximately zero and store next line in `ra`. |
| `bne a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a != b`. |
| `brne a(r?|num) b(r?|num) c(r?|num)` | Relative branch to `c` if `a != b`. |
| `bneal a(r?|num) b(r?|num) c(r?|num)` | Branch to `c` if `a != b` and store next line in `ra`. |
| `bnez a(r?|num) b(r?|num)` | Branch to `b` if `a != 0`. |
| `brnez a(r?|num) b(r?|num)` | Relative branch to `b` if `a != 0`. |
| `bnezal a(r?|num) b(r?|num)` | Branch to `b` if `a != 0` and store next line in `ra`. |

Branch targets can be line numbers or labels for absolute branches, and line
offsets for relative branches.
For the approximate branch-and-link rows, the suffix family is the important
part: `ap` means approximately equal, and `na` means not approximately equal.

## Conditional Function Cheat Sheet

Condition families:

| Suffix | Description | Branch | Branch and link | Relative branch | Set register |
| --- | --- | --- | --- | --- | --- |
| unconditional | Always | `j` | `jal` | `jr` | |
| `eq` | `a == b` | `beq` | `beqal` | `breq` | `seq` |
| `eqz` | `a == 0` | `beqz` | `beqzal` | `breqz` | `seqz` |
| `ge` | `a >= b` | `bge` | `bgeal` | `brge` | `sge` |
| `gez` | `a >= 0` | `bgez` | `bgezal` | `brgez` | `sgez` |
| `gt` | `a > b` | `bgt` | `bgtal` | `brgt` | `sgt` |
| `gtz` | `a > 0` | `bgtz` | `bgtzal` | `brgtz` | `sgtz` |
| `le` | `a <= b` | `ble` | `bleal` | `brle` | `sle` |
| `lez` | `a <= 0` | `blez` | `blezal` | `brlez` | `slez` |
| `lt` | `a < b` | `blt` | `bltal` | `brlt` | `slt` |
| `ltz` | `a < 0` | `bltz` | `bltzal` | `brltz` | `sltz` |
| `ne` | `a != b` | `bne` | `bneal` | `brne` | `sne` |
| `nez` | `a != 0` | `bnez` | `bnezal` | `brnez` | `snez` |
| `nan` | `a == NaN` | `bnan` | | `brnan` | `snan` |
| `nanz` | `a != NaN` | | | | `snanz` |
| `dns` | Device is not set | `bdns` | `bdnsal` | `brdns` | `sdns` |
| `dse` | Device is set | `bdse` | `bdseal` | `brdse` | `sdse` |
| `ap` | `a` approximately equals `b` | `bap` | `bapal` | `brap` | `sap` |
| `apz` | `a` approximately equals zero | `bapz` | `bapzal` | `brapz` | `sapz` |
| `na` | `a` not approximately equals `b` | `bna` | `bnaal` | `brna` | `sna` |
| `naz` | `a` not approximately equals zero | `bnaz` | `bnazal` | `brnaz` | `snaz` |

Rules:

- All `b-` commands use the target line or label as the last argument.
- All `s-` commands use the destination register as the first argument.
- All `br-` commands use the relative jump count as the last argument.
- Approximate commands take an extra tolerance argument.

Example relative branch:

```ic10
breq r0 r1 3 # if r0 == r1, jump three lines forward
```
