# IC10 Simulator Support

This table tracks how much of the IC10 reference in this directory is covered by
the StationC simulator. It is a coverage document, not a separate language
specification.

Status values:

| Status | Meaning |
| --- | --- |
| `Yes` | Parsed, executed, and covered by focused tests. |
| `Partial` | Useful subset exists, but some documented behavior is missing. |
| `No` | Documented behavior is not implemented. |
| `N/A` | The feature is documentation-only or does not execute. |

## Source and Program State

| Feature | Documented surface | Support | Notes |
| --- | --- | --- | --- |
| Comments and blank lines | `#` comments, empty lines | Yes | Inline comments and source line tracing are covered. |
| Labels | `label:` and label jump targets | Yes | Multiple labels may target one instruction. |
| Register aliases | `alias name r?` | Yes | Register aliases work in all value/register positions. |
| Device aliases | `alias name d?` | Yes | Aliases resolve to pins in simulator code; UI screw-label effects are out of scope. |
| Constants | `define name num` | Yes | Numeric constants are substituted by symbol resolution. |
| Registers | `r0` through `r15`, `ra`, `sp` | Yes | `ra` and `sp` are writable. |
| Indirect registers | `rr0`, `rrr1`, etc. | Yes | Invalid resolved indexes fault. |
| Indirect device pins | `dr0` | Yes | Resolved value must be `0` through `5`. |
| Numeric literals | decimal, hex `$`, binary `%`, `nan`, `pinf`, `ninf` | Yes | Binary underscores are accepted. |
| Hash literals | `HASH("Name")` | Yes | Uses CRC-32. |
| Dynamic logic type constants | `LogicType.*` | Partial | Common logic fields and `Channel0` through `Channel7` are present; the full enum is not. |

## Execution Control

| Feature | Instructions | Support | Notes |
| --- | --- | --- | --- |
| Tick yield | `yield` | Yes | Stops the current IC10 tick. |
| Hard fault | `hcf` | Yes | Reported as a typed runtime fault. |
| Sleep | `sleep` | No | No multi-tick sleep timer is implemented. |
| Instruction budget | 128-instruction default tick budget | Yes | Standalone and world simulators support caller-provided budgets. |
| Program halt | program counter reaches end | Yes | Reported as `Halt`. |

## Arithmetic and Selection

| Feature | Instructions | Support | Notes |
| --- | --- | --- | --- |
| Moves and random | `move`, `rand` | Yes | `rand` is deterministic for fresh simulator instances. |
| Basic arithmetic | `add`, `sub`, `mul`, `div`, `mod` | Yes | `mod` follows the documented IC10 examples. |
| Numeric helpers | `abs`, `ceil`, `floor`, `max`, `min`, `round`, `sqrt`, `trunc` | Yes | Uses Rust `f64` behavior where applicable. |
| Exponential functions | `exp`, `log`, `pow` | Yes | Uses Rust `f64` behavior. |
| Trigonometry | `acos`, `asin`, `atan`, `atan2`, `cos`, `sin`, `tan` | Yes | Angles are radians. |
| Selection | `select` | Yes | Non-zero condition selects the true operand. |

## Bitwise and Comparison Operations

| Feature | Instructions | Support | Notes |
| --- | --- | --- | --- |
| Bitwise gates | `and`, `nor`, `not`, `or`, `xor` | Yes | Operands must be exact integer values. |
| Shifts | `sla`, `sll`, `sra`, `srl` | Yes | Invalid shift operands fault. |
| Bit fields | `ext`, `ins` | Yes | Length must be `0` through `53`, and the selected range must fit in 64 bits. |
| Binary comparisons | `seq`, `sne`, `sge`, `sgt`, `sle`, `slt` | Yes | Store `1` or `0`. |
| Zero comparisons | `seqz`, `snez`, `sgez`, `sgtz`, `slez`, `sltz` | Yes | Store `1` or `0`. |
| Approximate comparisons | `sap`, `sna`, `sapz`, `snaz` | Yes | Uses the formulas documented in the reference. |
| NaN predicates | `snan`, `snanz` | Yes | Store `1` or `0`. |
| Device predicates | `sdns`, `sdse` | Yes | Unset pins and unknown `ReferenceId` targets are reported as predicate values, not runtime faults. |

## Branches and Calls

| Feature | Instructions | Support | Notes |
| --- | --- | --- | --- |
| Unconditional jumps | `j`, `jal`, `jr` | Yes | `jal` writes `ra`; `jr` is relative. |
| Binary comparison branches | `beq`, `bne`, `bge`, `bgt`, `ble`, `blt` | Yes | Absolute targets may be labels or numeric indexes. |
| Zero comparison branches | `beqz`, `bnez`, `bgez`, `bgtz`, `blez`, `bltz` | Yes | Absolute targets may be labels or numeric indexes. |
| Relative comparison branches | `breq`, `brne`, `brge`, `brgt`, `brle`, `brlt` | Yes | Offsets are relative to the current instruction. |
| Relative zero branches | `breqz`, `brnez`, `brgez`, `brgtz`, `brlez`, `brltz` | Yes | Offsets are relative to the current instruction. |
| Branch-and-link variants | `beqal`, `bneal`, `bgeal`, `bgtal`, `bleal`, `bltal`, zero variants | Yes | Link variants write `ra` before branching. |
| Approximate branches | `bap`, `bna`, `bapz`, `bnaz`, relative and link variants | Yes | Includes relative and branch-and-link variants. |
| NaN branches | `bnan`, `brnan` | Yes | No branch-and-link form is documented. |
| Device set branches | `bdns`, `bdnsal`, `bdse`, `bdseal`, `brdns`, `brdse` | Yes | Absolute, branch-and-link, and relative documented forms are covered. |
| Device validity branches | `bdnvl`, `bdnvs` | Yes | Checks load/store capability without faulting on unset or missing targets. |

## Stack

| Feature | Instructions | Support | Notes |
| --- | --- | --- | --- |
| Local stack | `push`, `pop`, `peek`, `poke` | Yes | Uses the IC10 instance stack and `sp`. |
| Remote stack read/write | `get`, `put`, `getd`, `putd` | Yes | Works through world device or housing stacks. |
| Stack clear | `clr`, `clrd` | Yes | Clears the addressed world device or housing stack. |
| Stack bounds | 512 cells | Yes | Invalid local and device stack addresses fault. |
| Stack persistence | stack survives across ticks | Partial | Simulator state persists while the world/IC instance exists; save/load persistence is not modeled. |

## Device and World I/O

| Feature | Instructions or surface | Support | Notes |
| --- | --- | --- | --- |
| Direct pins | `d0` through `d5` | Yes | World callers bind pins explicitly. |
| Mounted device | `db` | Yes | Targets the current IC housing body in the world simulator. |
| Direct logic load/store | `l`, `s` | Yes | Supports literal and dynamic logic fields. |
| Direct ReferenceId logic | `ld`, `sd` | Yes | Direct IDs may come from literals or registers. |
| Batch logic reads | `lb`, `lbn` | Yes | Supports prefab hash, optional name hash, dynamic fields, and all four modes. |
| Batch logic stores | `sb`, `sbn` | Yes | Supports prefab hash, optional name hash, dynamic fields, and dynamic values. |
| Slot logic | `ls`, `ss` | Yes | Supports pin, alias, `db`, indirect pin, dynamic slot index, and dynamic slot fields; direct `ReferenceId` slot access is rejected per the docs. |
| Batch slot logic | `lbs`, `lbns`, `sbs` | No | Slot state exists; batch slot traversal is not implemented. |
| Reagents | `lr`, `rmap` | No | Reagent storage and mapping are not modeled. |
| Device metadata | `ReferenceId`, `PrefabHash`, `NameHash` | Yes | Metadata fields are read-only. |
| Arbitrary logic fields | Stationpedia logic values | Partial | Tests can attach arbitrary numeric fields; device-specific behavior is not modeled. |
| Cable network channels | `d0:0`, `db:1`, `Channel0` through `Channel7` | No | Logic type constants exist, but connection operands and network storage do not. |
| Access trace | world-facing reads and writes | Yes | Logic, stack-address, and whole-stack writes are traced. |
| Schedule variation | stable, rotating, seeded shuffle | Yes | Used to expose ordering assumptions in world tests. |

## Known Unsupported Mnemonics

| Area | Mnemonics |
| --- | --- |
| Execution control | `sleep` |
| Slot I/O | `lbs`, `lbns`, `sbs` |
| Reagents | `lr`, `rmap` |
