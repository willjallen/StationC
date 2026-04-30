# StationC / StationOS: A distributed C-like runtime for Stationeers IC10

## 1. Project summary

This project builds a C-like programming language and runtime for **Stationeers**, targeting vanilla IC10 chips. The goal is to let a player write one high-level monolithic program, then compile it into a distributed bytecode image that can be executed by many identical IC10 worker chips inside the game.

The intended user-facing model is close to CUDA:

```c
__kernel float SumBatteryRatioChunk(DeviceGroup batteries, int chunkSize)
{
    int startIndex = job_index() * chunkSize;
    int endIndex = min(startIndex + chunkSize, group_count(batteries));

    float sum = 0.0f;

    for (int deviceIndex = startIndex; deviceIndex < endIndex; deviceIndex += 1)
    {
        DeviceRef battery = group_get(batteries, deviceIndex);
        sum += device_read(battery, Ratio);
    }

    return sum;
}

__host task Main()
{
    DeviceGroup batteries = discover("StructureBattery");

    while (true)
    {
        int jobCount = ceil_div(group_count(batteries), 8);

        Future<float> partialSums =
            SumBatteryRatioChunk<<<jobCount>>>(batteries, 8);

        float totalBatteryRatio = await_sum(partialSums);
        float averageBatteryRatio = totalBatteryRatio / max(1, group_count(batteries));

        sleep_ticks(1);
    }
}
```

The programmer writes ordinary C-like code with explicit `__host` tasks and `__kernel` worker functions. The compiler splits that monolithic source into ROM bytecode segments. At runtime, a host IC10 chip discovers available worker chips, assigns them jobs, points each worker at a ROM segment, and collects results. To scale the system, the player copies more identical worker chips into the data network, runs discovery/bootstrap again, and the host scheduler uses the additional workers automatically.

The runtime is not trying to generate or rewrite IC10 source code dynamically. Instead, it uses IC10 as the physical substrate for a tiny distributed virtual machine. IC10 source programs remain small and fixed. The large user program lives as numeric bytecode in ROM-bank chip stacks.

---

## 2. Background: what Stationeers is

Stationeers is a systems-heavy space-station construction and survival game. Players build and manage bases involving atmospherics, electrical systems, manufacturing, agriculture, and other interacting simulation systems. The Steam page describes Stationeers as a game about construction and management of a space station, with complex atmospheric, electrical, manufacturing, agriculture, and gravitational systems that require constant management. It also explicitly advertises automation through integrated circuits and assembly code. ([Steam Store][1])

A central gameplay mechanic is automation. Devices expose logic fields such as `On`, `Open`, `Setting`, `Pressure`, `Temperature`, `Ratio`, `PrefabHash`, `NameHash`, and many others. These fields can be read and written by logic chips and IC10 scripts. In vanilla Stationeers, the most powerful automation tool is the **Integrated Circuit**, usually called an IC10 chip.

---

## 3. Background: what IC10 is

IC10 is the in-game programmable integrated circuit system. It is often described by players as MIPS-like assembly. It has registers, labels, jumps, branches, arithmetic instructions, device load/store instructions, batch device instructions, and stack-memory instructions. The IC10 instruction reference lists MIPS-like arithmetic and branching operations such as `move`, `add`, `sub`, `mul`, `div`, `j`, `jal`, `beq`, `bne`, `blt`, `bgt`, etc. It also includes Stationeers-specific operations like `l` / `s` for device logic IO, `lb` / `sb` for batch IO, and `get` / `put` / `getd` / `putd` for stack access. ([stationeers-wiki.com][2])

An IC10 script normally runs inside one IC housing. The housing has configurable screw pins named `d0` through `d5`; these can point to nearby devices. There is also `db`, which refers to the device or housing the IC is mounted in. The wiki says ICs can interact with up to six devices via `d0` to `d5`, as well as the attached device via `db`, and that other IC housings can be selected as devices, allowing multi-IC programs. ([stationeers-wiki.com][3])

IC10 is not normal MIPS. It has MIPS-like control flow, but it also has direct game-world operations. For example:

```ic10
l r0 d0 Temperature   # read Temperature from device d0
s d1 On r0            # write r0 to On on device d1
lb r2 HASH("StructureBattery") Ratio Average
sb HASH("StructureWallLight") On 1
get r3 d2 10          # read stack cell 10 from device d2
put d2 10 r3          # write r3 to stack cell 10 on device d2
```

The `l` instruction loads a logic field from a device into a register, and `s` stores a register value into a device logic field. Batch instructions can load or store values across many devices matching prefab/name hashes. The documentation describes `lb` as loading a logic field from all output-network devices of a given type using a batch mode such as Average, Sum, Minimum, or Maximum, and `sb` as storing a value to all matching devices. ([stationeers-wiki.com][2])

---

## 4. Vanilla IC10 constraints

The entire motivation for this project is that IC10 is powerful but tiny.

The current IC10 documentation states that an IC chip program is limited to **128 lines**, **90 characters per line**, and **4096 bytes** total. It also notes that each character takes one byte and line breaks take two bytes. ([stationeers-wiki.com][4])

Each IC10 chip has a stack. The IC10 wiki describes the stack as memory that can hold **512 values**. Each IC10 chip has its own stack, and some devices also have stacks. The stack can be accessed through instructions like `push`, `pop`, `peek`, `poke`, `get`, `getd`, `put`, and `putd`. The same documentation says stack memory is persistent on logic chips: values can remain after the code that wrote them is removed. ([stationeers-wiki.com][3])

IC10 execution is tick-based. The instruction reference says `yield` pauses execution for one tick. Community and Steam discussion sources describe the chip as executing up to **128 instructions per world simulation tick**, or until it reaches `yield`; a Steam guide describes a Stationeers tick as half a second and notes that devices update up to twice per second. ([stationeers-wiki.com][2])

So, nominally:

```text
1 world tick ≈ 0.5 seconds
1 IC10 chip ≈ 128 executed IC10 instructions per tick
1 IC10 chip ≈ 256 executed IC10 instructions per second
```

This is not enough to run a normal high-level language directly. A single IC10 chip cannot contain a large source program, cannot execute many instructions per second, and cannot hold much local memory. However, the constraints are per-chip. If a system has 32 worker ICs, then it has 32 independent instruction budgets and 32 independent stacks. The project exploits that.

The safest scheduling model for StationC is: each active IC10 receives a
per-chip tick slice, but the order between chips is not part of the language
contract. Public knowledge strongly suggests IC10 chips are not instruction-
interleaved with each other and are not intentionally run as a distributed
thread pool. The practical model is sequential tick slices in an unspecified
internal order. StationC must therefore exploit per-chip throughput without
depending on one IC10 housing running before another.

---

## 5. Important vanilla affordances

The project relies on a few IC10 features that make a distributed runtime possible.

First, stacks are persistent and can be read or written from other chips/devices. The IC10 instruction documentation says `get` reads a stack value from a provided device, `put` writes a value to a device stack, `getd` reads a stack value by direct device id, and `putd` writes a stack value by direct device id. ([stationeers-wiki.com][2])

Second, devices can be addressed by `ReferenceId`. The wiki says direct reference instructions can address a specific device via its `ReferenceId`, including `getd`, `putd`, `ld`, and `sd`. It also says batch instructions can address devices by prefab hash and name hash, and all logic-readable devices contain `PrefabHash` and `NameHash`. ([stationeers-wiki.com][3])

Third, the data network can be scanned. The Advanced IC10 Programming page describes using `get r0 db:0 index` to read a `ReferenceId` for the device at a given network index. If there is no such device at that index, it returns `-1`. The page also notes that the order is not determined by anything specific, though it may remain stable until the network changes. Once a `ReferenceId` is found, the code can read logic properties such as `PrefabHash` to identify the device type. ([stationeers-wiki.com][5])

Fourth, IC10 can do normal arithmetic and branching. That makes it possible to write a tiny interpreter in IC10. The interpreter can fetch numeric bytecode from ROM stacks, decode it, execute it, and branch within the virtual program.

---

## 6. Core objective

The objective is to build a vanilla-compatible distributed programming system with these properties:

1. The user writes one C-like monolithic program.
2. The program can be longer and more complex than one IC10 chip’s source limit.
3. The compiler emits bytecode stored in ROM-bank IC stacks.
4. A host IC discovers workers and devices at runtime.
5. Worker ICs are homogeneous: each worker runs the same small VM firmware.
6. The host schedules jobs by pointing workers at ROM segments.
7. Adding more workers increases throughput without changing the high-level program.
8. The high-level implementation details of hardware discovery, worker assignment, mailboxes, ROM addressing, and synchronization are opaque to the programmer.
9. The system remains vanilla: no mods, no external runtime process while the game is running, and no assumption that IC10 can rewrite another chip’s source code.

“Arbitrary length” means: not bounded by one IC10 source file. The program is still finite and bounded by the number of ROM banks, RAM banks, workers, and time the player is willing to allocate. But the architecture is extensible: more ROM chips provide more program storage, more RAM/heap chips provide more data storage, and more worker chips provide more parallel execution capacity.

---

## 7. High-level solution

The system is a distributed virtual machine built out of IC10 chips.

There are four classes of physical chips:

```text
Host chip
    Runs the scheduler, discovery system, job queue, and optionally host-side bytecode.

Worker chips
    All run identical VM firmware.
    Wait for jobs.
    Execute bytecode segments from ROM.
    Return results.

ROM-bank chips
    Hold compiled bytecode as numeric stack data.
    They are mostly passive storage.

RAM/heap-bank chips
    Hold mutable runtime memory, device tables, job queues, futures, result buffers, and heap allocations.
```

The player writes:

```text
main.sc10
```

The compiler emits:

```text
bootstrap.ic10
host_firmware.ic10
worker_firmware.ic10
rom_loader_000.ic10
rom_loader_001.ic10
...
manifest.json
disassembly.txt
```

The in-game system is then:

```text
                  +---------------------+
                  | Host Kernel IC      |
                  | scheduler/runtime   |
                  +----------+----------+
                             |
          ---------------------------------------
          |                  |                  |
          v                  v                  v
+----------------+  +----------------+  +----------------+
| Worker IC      |  | Worker IC      |  | Worker IC      |
| same VM        |  | same VM        |  | same VM        |
+----------------+  +----------------+  +----------------+
          |                  |                  |
          ---------------------------------------
                             |
        +--------------------+--------------------+
        |                                         |
        v                                         v
+---------------+                         +---------------+
| ROM banks     |                         | RAM/heap banks |
| bytecode      |                         | runtime data   |
+---------------+                         +---------------+
```

The host does not send IC10 source to workers. It sends **job descriptors**:

```text
run this ROM bank
start at this program counter
use these arguments
write your result here
return when done
```

The worker VM reads that ROM segment and executes it.

---

## 8. Why not compile C to MIPS/IC10 directly?

A normal C-to-MIPS pipeline is the wrong abstraction for this target.

Ordinary C assumes a conventional machine:

```text
byte-addressed memory
normal stack frames
normal pointers
function calls
heap allocation
load/store RAM
compiler-controlled calling convention
```

IC10 gives something different:

```text
tiny source budget
small register file
512-value stack per chip
device logic IO
batch device IO
ReferenceId device IO
network discovery
tick-yielded execution
persistent stack data
```

IC10 is MIPS-like, but the important operations are not normal CPU operations. The useful instructions are things like “read this device’s pressure,” “write all named lights,” “read another chip’s stack cell,” or “scan a data network.” A generic C-to-MIPS backend would emit low-level scalar instructions and would not understand Stationeers-specific device and scheduling semantics.

Therefore the language should be C-like, but the compiler should target a custom **StationOS bytecode**, not real MIPS.

The pipeline should be:

```text
StationC source
    -> parser
    -> typed AST
    -> intermediate representation
    -> StationOS bytecode
    -> ROM stack image
    -> IC10 loader scripts
```

Not:

```text
C source
    -> MIPS assembly
    -> IC10 transpilation
```

---

## 9. Language overview: StationC

The high-level language should be called something like `StationC`, `C10`, or `SC10`. This document will call it **StationC**.

StationC is a restricted C-like language with CUDA-like execution annotations and Stationeers-specific intrinsics.

It should feel like C:

```c
int count = group_count(batteries);

for (int index = 0; index < count; index += 1)
{
    DeviceRef battery = group_get(batteries, index);
    total += device_read(battery, Ratio);
}
```

It should feel CUDA-like for parallel work:

```c
Future<float> partialSums =
    SumBatteryRatioChunk<<<jobCount>>>(batteries, CHUNK_SIZE);

float total = await_sum(partialSums);
```

It should expose Stationeers as first-class concepts:

```c
DeviceGroup batteries = discover("StructureBattery");
DeviceGroup vents = discover_named("StructureActiveVent", "Farm");

float pressure = device_read(sensor, Pressure);
batch_write(vents, On, 1);
```

---

## 10. StationC execution spaces

StationC has two main executable spaces.

### 10.1 Host tasks

A host task runs on the host runtime. Host tasks can discover hardware, launch kernels, await futures, manage persistent global state, and perform high-level policy logic.

```c
__host task Main()
{
    while (true)
    {
        refresh_discovery_if_needed();

        // launch work
        // await results
        // decide control actions

        sleep_ticks(1);
    }
}
```

A `task` is not an ordinary C function. It is a resumable coroutine. The runtime stores its program counter, local variables, wait state, pending futures, and wake tick.

### 10.2 Worker kernels

A kernel is a function that can run on any homogeneous worker IC.

```c
__kernel float SumBatteryRatioChunk(DeviceGroup batteries, int chunkSize)
{
    int startIndex = job_index() * chunkSize;
    int endIndex = min(startIndex + chunkSize, group_count(batteries));

    float sum = 0.0f;

    for (int deviceIndex = startIndex; deviceIndex < endIndex; deviceIndex += 1)
    {
        DeviceRef battery = group_get(batteries, deviceIndex);
        sum += device_read(battery, Ratio);
    }

    return sum;
}
```

A kernel compiles to a ROM bytecode segment. The host launches kernels by constructing job descriptors pointing at that segment. Workers execute the segment independently.

Kernel restrictions in the MVP:

```text
no malloc
no recursion
no function pointers
no arbitrary pointer arithmetic
no spawning other kernels
no awaiting futures
bounded locals
simple loops only
explicit result return
```

These restrictions can be loosened later.

---

## 11. CUDA-like launch model

StationC should use CUDA-style kernel launch syntax:

```c
KernelName<<<jobCount>>>(arguments);
```

Meaning:

```text
Create jobCount independent jobs.
Each job runs the same kernel bytecode segment.
Each job receives job_index() and job_count().
The host scheduler assigns jobs to idle worker ICs.
```

Example:

```c
int jobCount = ceil_div(group_count(batteries), CHUNK_SIZE);

Future<float> partialSums =
    SumBatteryRatioChunk<<<jobCount>>>(batteries, CHUNK_SIZE);

float total = await_sum(partialSums);
```

Inside the kernel:

```c
int currentJob = job_index();
int totalJobs = job_count();
```

These are not magic CPU registers. They are values from the job descriptor / worker mailbox.

---

## 12. StationC type system

The MVP type system should be intentionally small.

Primitive types:

```c
int
float
bool
```

Opaque runtime handle types:

```c
DeviceRef
DeviceGroup
Future<T>
CommandBuffer
RamPtr<T>
RomPtr<T>
```

Compile-time symbolic types:

```c
LogicField
PrefabHash
NameHash
BatchMode
```

At the physical IC10 level, most values are numeric cells. The compiler may treat `int`, `float`, and `bool` as separate static types, but the runtime representation can initially be one numeric cell. The IC10 wiki notes that registers and constants are usually decimal numeric values and that integers are not a distinct real CPU-style type; values are represented in floating-point-like numeric cells, with large integers at risk of rounding. ([stationeers-wiki.com][3])

Unsupported in MVP:

```text
char
strings at runtime
structs by value
unions
varargs
malloc/free
standard libc
full C pointer arithmetic
function pointers
```

Strings should exist only at compile time for things like:

```c
discover("StructureBattery")
discover_named("StructureGasSensor", "Farm")
```

The compiler should hash them or map them to runtime constants.

The C `volatile` keyword itself is not part of the MVP source language.
StationC still has volatile-like operations: `device_read`, `device_write`,
`batch_read`, `batch_write`, and other world-facing calls are effectful
operations. The compiler must not remove, merge, hoist, sink, or reorder those
operations as if they were ordinary memory loads and stores.

---

## 13. Memory model

StationC must not pretend to have normal C memory. It has explicit memory spaces.

### 13.1 ROM

ROM stores bytecode and constants. Physically, ROM is one or more IC stack memories preloaded with numeric values.

```c
rom const float LowBatteryRatio = 0.25f;
```

### 13.2 RAM

RAM stores mutable runtime data. Physically, RAM is one or more IC stacks used as heap/storage banks.

```c
ram float partialSums[64];
```

### 13.3 Persistent state

Persistent variables survive across ticks and are stored in RAM bank cells.

```c
persistent float LastAverageBatteryRatio = 0.0f;
persistent int SolarSearchDirection = 1;
```

### 13.4 Local state

Local variables live in worker/host VM registers or stack-frame slots.

```c
float sum = 0.0f;
```

### 13.5 Shared state

Shared state is visible to host and workers, but access must be controlled by the runtime.

```c
shared float WorkerResults[64];
```

For MVP safety, workers should write only to exclusive result slots assigned by
the host. Defined StationC behavior requires one of:

1. Exclusive ownership of the memory range for the duration of the job.
2. A runtime-mediated protocol such as a command buffer, mailbox state machine,
   or host-owned scheduler field.
3. A future explicit unsafe escape hatch for raw shared mutation.

Concurrent unsynchronized writes, and read-modify-write protocols over shared
cells, are outside the defined StationC memory model. The compiler must not emit
code that depends on those races resolving in a particular inter-IC order.

---

## 14. Device model

StationC treats devices as discovered runtime handles.

```c
DeviceGroup batteries = discover("StructureBattery");
DeviceGroup farmSensors = discover_named("StructureGasSensor", "Farm");

int count = group_count(batteries);
DeviceRef battery = group_get(batteries, 0);

float ratio = device_read(battery, Ratio);
device_write(light, On, 1);
```

The compiler/runtime must support:

```c
DeviceGroup discover(const char *prefabName);
DeviceGroup discover_named(const char *prefabName, const char *deviceName);

void refresh_discovery();
void refresh_discovery_if_needed();

int group_count(DeviceGroup group);
DeviceRef group_get(DeviceGroup group, int index);

float device_read(DeviceRef device, LogicField field);
void device_write(DeviceRef device, LogicField field, float value);

float batch_read(DeviceGroup group, LogicField field, BatchMode mode);
void batch_write(DeviceGroup group, LogicField field, float value);
```

The runtime implements discovery by scanning the data network with `get db:0 index`, reading each discovered device’s `ReferenceId`, then reading `PrefabHash` and `NameHash` to classify it. The Advanced IC10 docs describe this exact network-index-to-ReferenceId technique and explain that the returned `ReferenceId` can then be used to query `PrefabHash` and other logic properties. ([stationeers-wiki.com][5])

Device and batch reads are volatile world observations. Each call observes the
target at the moment that IC10 instruction executes. Repeating the same read may
produce a different value if the world, another IC, or the device itself has
changed. This is legal, but programmers and compiler passes must not assume
that repeated device reads are stable.

Suspicious:

```c
if (device_read(sensor, Temperature) > 300 &&
    device_read(sensor, Temperature) < 310)
{
    device_write(cooler, On, 1);
}
```

The two `device_read` calls are two separate volatile observations. If the
program wants a coherent sample, it should read once and reuse the local value:

```c
float temperature = device_read(sensor, Temperature);

if (temperature > 300 && temperature < 310)
{
    device_write(cooler, On, 1);
}
```

---

## 15. Command buffers for safe writes

Parallel reads are easy. Parallel writes are dangerous.

If multiple workers write the same device field at the same time, the result is order-dependent and hard to debug. The default model should therefore be:

```text
Sense phase:
    Workers read devices.

Compute phase:
    Workers compute desired actions.

Apply phase:
    Host or an apply-worker writes device changes deterministically.
```

StationC should support command buffers:

```c
__kernel void BuildVentCommands(
    DeviceGroup vents,
    float averagePressure,
    CommandBuffer commands
)
{
    int startIndex = job_index() * 8;
    int endIndex = min(startIndex + 8, group_count(vents));

    for (int deviceIndex = startIndex; deviceIndex < endIndex; deviceIndex += 1)
    {
        DeviceRef vent = group_get(vents, deviceIndex);
        command_write(commands, vent, On, averagePressure < 50.0f);
    }
}

__host task Main()
{
    CommandBuffer commands = command_buffer_create();

    Future<void> jobs =
        BuildVentCommands<<<jobCount>>>(FarmVents, averagePressure, commands);

    await_all(jobs);
    command_buffer_apply(commands);
}
```

The command buffer is a runtime data structure in RAM. Each worker gets an exclusive region of the buffer. The host applies actions after all workers finish.

---

## 16. StationOS bytecode

The compiler emits StationOS bytecode, not IC10 source.

A bytecode instruction should be simple enough that all worker chips can share the same interpreter, but rich enough to avoid wasting huge amounts of IC10 budget.

Use fixed-width instructions first:

```text
cell + 0 = opcode
cell + 1 = operand0
cell + 2 = operand1
cell + 3 = operand2
```

This wastes ROM cells but simplifies the interpreter. Since ROM can be scaled by adding bank chips, interpreter simplicity matters more than storage efficiency in v1.

Initial opcode set:

```text
0   NOP

1   LOAD_IMM        dst, immediate
2   MOVE            dst, src

10  ADD             dst, left, right
11  SUB             dst, left, right
12  MUL             dst, left, right
13  DIV             dst, left, right
14  MIN             dst, left, right
15  MAX             dst, left, right
16  ABS             dst, src

20  LT              dst, left, right
21  LE              dst, left, right
22  GT              dst, left, right
23  GE              dst, left, right
24  EQ              dst, left, right
25  NE              dst, left, right

30  JUMP            absolutePc
31  JUMP_IF_ZERO    cond, absolutePc
32  JUMP_IF_NONZERO cond, absolutePc

40  LOAD_LOCAL      dst, localSlot
41  STORE_LOCAL     localSlot, src
42  LOAD_RAM        dst, address
43  STORE_RAM       address, src
44  LOAD_ROM        dst, address

50  DEVICE_READ     dst, deviceRefRegister, logicField
51  DEVICE_WRITE    deviceRefRegister, logicField, valueRegister
52  BATCH_READ      dst, groupOrHash, logicField, batchMode
53  BATCH_WRITE     groupOrHash, logicField, valueRegister

60  GROUP_COUNT     dst, group
61  GROUP_GET       dst, group, indexRegister

70  LOAD_ARG        dst, argIndex
71  STORE_RESULT    resultIndex, src
72  JOB_INDEX       dst
73  JOB_COUNT       dst
74  WORKER_ID       dst

80  COMMAND_WRITE   commandBuffer, deviceRef, logicField, value

90  YIELD
91  RETURN
92  FAULT
```

Host-only bytecode can include:

```text
100 SPAWN_KERNEL
101 AWAIT
102 AWAIT_ALL
103 AWAIT_SUM
104 SLEEP_TICKS
105 DISCOVERY_REFRESH
106 COMMAND_BUFFER_CREATE
107 COMMAND_BUFFER_APPLY
```

In MVP, host code may be implemented with its own specialized firmware rather than fully interpreted. But the long-term design should allow the host to execute the same bytecode format for monolithic program logic.

---

## 17. ROM banks

A ROM bank is an IC chip stack used as read-only program storage.

A ROM loader script writes bytecode cells into a ROM chip stack:

```ic10
poke 0  1     # LOAD_IMM
poke 1  0     # dst r0
poke 2  10    # immediate 10
poke 3  0

poke 4  91    # RETURN
poke 5  0
poke 6  0
poke 7  0
```

After loading, the ROM chip can run an idle script. Its stack remains the ROM image.

Workers fetch bytecode with `getd`:

```ic10
getd Opcode RomReferenceId ProgramCounter
add ProgramCounter ProgramCounter 1
getd Operand0 RomReferenceId ProgramCounter
add ProgramCounter ProgramCounter 1
...
```

The IC10 wiki documents that `poke` stores a value into the current chip stack, `getd` reads another device’s stack by device id, and stack memory persists on logic chips. ([stationeers-wiki.com][2])

For v1, each job descriptor should include:

```text
romBankReferenceId
localEntryPc
localEndPc
```

This avoids the overhead of computing global address to ROM bank every instruction. Later, implement global ROM addressing:

```text
globalPc -> bankIndex = globalPc / 512
globalPc -> offset    = globalPc % 512
romRef   = RomBankTable[bankIndex]
```

---

## 18. RAM and heap banks

RAM banks are IC chip stacks used as mutable memory.

RAM bank uses:

```text
device table
group table
worker table
job queue
future table
result buffers
command buffers
persistent globals
heap allocations
task control blocks
```

Each RAM chip has 512 numeric cells. Multiple RAM banks are chained into a logical memory space:

```text
logicalAddress = bankIndex * 512 + offset
bankIndex = logicalAddress / 512
offset = logicalAddress % 512
```

For performance, job descriptors should pass direct RAM bank references and local offsets where possible.

Heap design for MVP:

```text
Bump allocator only.
No free.
Reset heap per control frame if desired.
Persistent allocations live in a separate region.
```

Heap metadata:

```text
HeapBankTable:
    bankCount
    bankRef[0..N]

HeapState:
    nextFreeLogicalAddress
    heapEndLogicalAddress
```

Later heap designs can add free lists, slab allocation, or fixed-size pools, but do not start there.

---

## 19. Worker firmware

All worker chips run the same IC10 script.

Worker responsibilities:

```text
advertise worker magic/version
wait for bootstrap assignment
wait for job descriptor
load job metadata
execute ROM bytecode
write results
mark job done/faulted
heartbeat while running
```

Worker stack ABI:

```text
0   workerMagic
1   workerVersion
2   workerId
3   workerState
4   heartbeat

10  jobId
11  jobState
12  romBankRef
13  entryPc
14  endPc
15  contextPointer
16  jobIndex
17  jobCount
18  arg0
19  arg1
20  arg2
21  arg3
22  resultPointer
23  result0
24  result1
25  faultCode
```

Worker states:

```text
0 = unconfigured
1 = idle
2 = jobReady
3 = running
4 = done
5 = faulted
```

Pseudo-code:

```text
worker_boot:
    poke stack[0] WORKER_MAGIC
    poke stack[1] WORKER_VERSION
    poke stack[3] UNCONFIGURED

wait_for_assignment:
    if stack[2] == 0:
        yield
        repeat
    stack[3] = IDLE

worker_loop:
    yield
    if stack[3] != JOB_READY:
        repeat

    stack[3] = RUNNING
    load job descriptor
    pc = entryPc

execute_loop:
    fetch opcode and operands from romBankRef at pc
    pc += 4
    dispatch opcode

    if opcode == RETURN:
        write result
        stack[3] = DONE
        goto worker_loop

    if opcode == FAULT or error:
        stack[25] = faultCode
        stack[3] = FAULTED
        goto worker_loop
```

The worker interpreter must fit within IC10 source limits. The v1 opcode set must be chosen with that constraint in mind.

---

## 20. Host firmware

The host chip is the scheduler and runtime coordinator.

Host responsibilities:

```text
load bootstrap-generated tables
maintain worker registry
maintain job queue
launch jobs
poll workers
collect results
wake futures/tasks
run host tasks
refresh hardware discovery when requested
apply command buffers
handle faults/timeouts
```

The host should not micromanage individual bytecode instructions. It should schedule entire ROM segments. This is essential for throughput.

Good:

```text
host assigns worker:
    run kernel K at ROM pc 500 with args A, B, C
worker runs until RETURN
host collects result
```

Bad:

```text
host sends one bytecode instruction at a time
worker executes one instruction
host sends next instruction
```

The latter would bottleneck on host scheduling.

---

## 21. Bootstrap firmware

The bootstrap IC configures the runtime.

Bootstrap responsibilities:

```text
scan network
find worker chips by stack magic
assign worker ids
find ROM banks
find RAM/heap banks
discover Stationeers devices
classify devices into groups
write device/group/worker tables into RAM
write runtime configuration to host
set host state to RUN
```

Workers identify themselves by stack signature:

```text
stack[0] = WORKER_MAGIC
stack[1] = WORKER_VERSION
```

Bootstrap scans the network:

```ic10
get ReferenceId db:0 Index
```

Then for each candidate:

```ic10
getd Magic ReferenceId 0
if Magic == WORKER_MAGIC:
    register worker
else:
    l PrefabHash ReferenceId PrefabHash
    l NameHash ReferenceId NameHash
    classify device
```

The network scan pattern is directly supported by the Advanced IC10 docs: `get r0 db:0 index` can return a device `ReferenceId`, and that `ReferenceId` can then be used to read properties like `PrefabHash`. ([stationeers-wiki.com][5])

Bootstrap output tables:

```text
RuntimeHeader:
    magic
    version
    workerCount
    romBankCount
    ramBankCount
    deviceCount
    groupCount

WorkerTable:
    workerId
    referenceId
    state
    version
    flags

RomBankTable:
    bankIndex
    referenceId
    loadedCellCount

RamBankTable:
    bankIndex
    referenceId
    usageFlags

DeviceTable:
    deviceIndex
    referenceId
    prefabHash
    nameHash
    flags

GroupTable:
    groupId
    prefabHash
    nameHash
    startIndex
    count
```

The compiler can assume the player provides enough ROM chips. Bootstrap still needs to discover them and record their `ReferenceId`s so workers can fetch bytecode.

---

## 22. Scheduling model

The scheduler manages a dynamic number of homogeneous workers.

Core scheduler loop:

```text
while true:
    poll all workers
    collect completed results
    mark futures done
    wake host tasks whose futures/sleeps are complete
    run a slice of host task bytecode
    enqueue newly requested jobs
    assign queued jobs to idle workers
    yield
```

Each job record:

```text
Job:
    jobId
    state
    priority
    kernelId
    romBankRef
    entryPc
    endPc
    contextPointer
    jobIndex
    jobCount
    arg0
    arg1
    arg2
    arg3
    resultPointer
    assignedWorkerId
    faultCode
```

Job states:

```text
0 = free
1 = queued
2 = assigned
3 = running
4 = done
5 = faulted
6 = cancelled
```

Futures:

```text
Future:
    futureId
    state
    jobStart
    jobCount
    resultPointer
    reductionMode
    resultValue
```

For CUDA-like launches:

```c
Future<float> partialSums =
    SumBatteryRatioChunk<<<jobCount>>>(batteries, 8);
```

The compiler emits host bytecode that creates `jobCount` job records, all pointing to the same kernel ROM segment but with different `jobIndex` values.

---

## 23. Pipeline and out-of-order execution

The runtime should pursue parallelism aggressively, but safely.

Parallelism means independent IC10 tick slices and independent jobs, not a
guaranteed shared-memory thread model. StationC must treat inter-IC execution
order as unspecified. A program is correct only if any valid ordering of active
worker slices produces the same StationC-visible result, except for explicitly
volatile world observations.

There are three levels of parallelism:

### 23.1 Data parallel kernels

The compiler/runtime can split loops over device groups into chunks:

```c
SumBatteryRatioChunk<<<jobCount>>>(batteries, 8);
```

Each chunk is independent, so jobs can run concurrently.

### 23.2 Job DAG scheduling

The compiler should represent host work as a dependency graph:

```text
discover devices
    -> read battery chunks
        -> reduce battery chunks
            -> decide power policy
                -> build command buffer
                    -> apply command buffer
```

Jobs whose dependencies are satisfied can run immediately. Jobs whose dependencies are not satisfied remain pending.

This gives out-of-order behavior in the useful sense: the runtime does not have to execute the source file linearly when there are independent kernels. It can launch independent work early and overlap worker execution.

### 23.3 Long control pipelines across ticks

Stationeers device state often updates at tick boundaries. A safe automation pattern is:

```text
tick N:
    read sensors
    compute desired actions
tick N+1:
    apply actions
tick N+2:
    observe consequences
```

The runtime should support phased scheduling:

```text
SENSE
COMPUTE
APPLY
YIELD
```

The Steam discussion notes that after turning on a device, a script may need to wait at least one world simulation tick before reading updated data; the same thread recommends yielding to allow device state to update. ([Steam Community][6])

Therefore the runtime should not try to “spin faster than the world.” It should use intra-tick instruction budgets for computation and inter-tick phases for world interaction.

Within one phase, external world reads should be treated as samples, not cached
facts. The compiler may reuse a value only after it has been stored in a
StationC local, temporary, or runtime-owned snapshot buffer.

---

## 24. Synchronization model

Do not rely on raw shared-memory locks in MVP.

IC10 does not expose an atomic compare-and-swap primitive. Therefore this is unsafe:

```text
if lock == 0:
    lock = myWorkerId
```

Two workers can observe the lock as free and both write ownership.

Instead:

1. Use exclusive output ranges.
2. Use command buffers.
3. Use host-owned scheduler state.
4. Use single-writer ownership rules.
5. Use sequence numbers in mailboxes.

Safe worker result layout:

```text
Worker job 0 writes ResultBuffer[0]
Worker job 1 writes ResultBuffer[1]
Worker job 2 writes ResultBuffer[2]
Host reduces ResultBuffer[0..N]
```

Mailbox writes should use state transitions:

```text
IDLE -> JOB_READY -> RUNNING -> DONE/FAULTED -> IDLE
```

If future versions need mutexes or semaphores, implement them as host/lock-server services, not as raw shared-memory test-and-set.

---

## 25. Repository layout

Recommended repository tree:

```text
stationc/
    README.md
    DESIGN.md
    ROADMAP.md
    LICENSE
    Cargo.toml
    Cargo.lock

    examples/
        battery_average.sc10
        farm_atmosphere.sc10
        solar_tracker.sc10
        power_shed.sc10
        command_buffer_demo.sc10

    docs/
        stationeers_context.md
        language_spec.md
        bytecode_spec.md
        runtime_abi.md
        compiler_architecture.md
        ic10_firmware.md
        bootstrap_protocol.md
        memory_model.md
        scheduling_model.md
        testing_strategy.md
        known_limitations.md

    src/
        lib.rs
        main.rs

        cli/
            mod.rs
            commands.rs

        compiler/
            mod.rs

            frontend/
                mod.rs
                lexer.rs
                parser.rs
                tokens.rs
                ast.rs
                diagnostics.rs
                source_map.rs

            sema/
                mod.rs
                symbols.rs
                types.rs
                type_checker.rs
                builtin_registry.rs
                stationeers_defs.rs
                validation.rs

            ir/
                mod.rs
                cfg.rs
                builder.rs
                pretty.rs
                verifier.rs

            analysis/
                mod.rs
                dependency_graph.rs
                escape_analysis.rs
                effect_analysis.rs
                kernel_partitioning.rs
                liveness.rs

            lower/
                mod.rs
                host.rs
                kernel.rs
                calls.rs
                memory.rs
                devices.rs
                futures.rs

            bytecode/
                mod.rs
                opcodes.rs
                instruction.rs
                encoder.rs
                decoder.rs
                disassembler.rs
                verifier.rs
                layout.rs

            runtime_model/
                mod.rs
                abi.rs
                memory_map.rs
                jobs.rs
                devices.rs
                groups.rs
                manifest.rs

            emit/
                mod.rs
                ic10_writer.rs
                firmware_templates.rs
                rom_loader.rs
                host_firmware.rs
                worker_firmware.rs
                bootstrap_firmware.rs
                manifest_writer.rs
                debug_map_writer.rs

            support/
                mod.rs
                hash.rs
                stationpedia_data.rs
                constants.rs

        sim/
            mod.rs

            ic10/
                mod.rs
                instruction.rs
                parser.rs
                program.rs
                registers.rs
                stack.rs
                vm.rs
                trace.rs

            world/
                mod.rs
                data_network.rs
                device.rs
                device_logic.rs
                ic_housing.rs
                prefab.rs
                scenario.rs
                tick.rs
                world.rs

            stationos/
                mod.rs
                bytecode_vm.rs
                memory.rs
                scheduler.rs
                worker.rs
                host.rs
                trace.rs

        tools/
            mod.rs
            format_manifest.rs
            dump_rom.rs
            split_loader_pages.rs
            stationpedia_import.rs

    firmware/
        worker.vm.ic10.tera
        host.kernel.ic10.tera
        bootstrap.ic10.tera
        rom_bank_idle.ic10.tera
        ram_bank_idle.ic10.tera

    tests/
        compiler.rs
        compiler/
            lexer.rs
            parser.rs
            type_checker.rs
            bytecode_encoder.rs
            disassembler.rs
            hash.rs

        sim.rs
        sim/
            ic10/
                arithmetic.rs
                bitwise.rs
                branching.rs
                cli.rs
                comparisons.rs
                execution.rs
                parser.rs
                stack.rs
            world/
                tick_budget.rs
                multi_chip.rs
                device_io.rs
            stationos/
                kernel_launch.rs
                await_sum.rs
                command_buffer.rs
                scheduler_scaling.rs

        fixtures/
            golden/
                sources/
                    simple_arithmetic.sc10
                    simple_kernel.sc10
                    battery_average.sc10
                expected/
                    simple_arithmetic.disasm
                    simple_kernel.manifest.json

    out/
        .gitkeep
```

The repository should be Rust-first for the compiler, simulator, CLI, and support tooling. All off-game implementation code should be written in Rust in one Cargo package with one user-facing `stationc` binary. Python should not be part of the planned implementation. The generated and handwritten in-game firmware remains IC10.

Use Rust's standard package shape:

```text
src/lib.rs   shared library root for compiler, simulators, and tools
src/main.rs  small binary entry point for the `stationc` CLI
```

Use one module-folder convention consistently under `src/`:

```text
folder/mod.rs      root of that module folder
folder/thing.rs    child module declared by folder/mod.rs
```

Every Rust module directory in the tree above has a `mod.rs`. Leaf modules are plain `.rs` files. Cargo discovers integration test crates from top-level `.rs` files directly under `tests/`; those harness files can declare ordinary nested modules such as `tests/sim/ic10/arithmetic.rs` to keep test layout aligned with `src/sim/ic10`.

The code should be segmented by responsibility with top-level modules. The compiler toolchain lives under `compiler/`. The simulator stack lives under `sim/`, with clear layers:

```text
sim::stationos
    uses sim::world
        uses sim::ic10
```

The IC10 simulator and world simulator must also be runnable independently through `stationc` subcommands.

---

## 26. CLI design

The CLI should support:

```bash
stationc build examples/battery_average.sc10 --out out/battery_average
stationc disasm out/battery_average/program.rom.json
stationc sim ic10 firmware/worker_firmware.ic10 --ticks 10 --trace
stationc sim world examples/worlds/basic.world.toml --ticks 100 --trace
stationc sim stationos examples/battery_average.sc10 --workers 8 --ticks 100
stationc emit-loaders examples/battery_average.sc10 --out out/battery_average
stationc verify out/battery_average/manifest.json
```

Build output:

```text
out/battery_average/
    bootstrap.ic10
    host_firmware.ic10
    worker_firmware.ic10
    rom_bank_idle.ic10
    ram_bank_idle.ic10

    rom_loader_000_page_000.ic10
    rom_loader_000_page_001.ic10
    rom_loader_001_page_000.ic10

    program.rom.json
    manifest.json
    disassembly.txt
    debug_map.json
    symbol_table.json
```

The `manifest.json` is critical. It describes how ROM addresses, functions, groups, globals, and runtime tables map to physical IC stacks.

---

## 27. Compiler pipeline

The compiler pipeline should be:

```text
Source text
    -> Lexer
    -> Parser
    -> AST
    -> Semantic analysis
    -> Effect analysis
    -> Host/kernel partitioning
    -> IR
    -> Bytecode
    -> ROM layout
    -> IC10 firmware/loader generation
    -> Manifest/debug maps
```

### 27.1 Lexer

Tokenizes C-like syntax.

Must support:

```text
identifiers
numbers
string literals for compile-time hashes
keywords
operators
punctuation
comments
```

### 27.2 Parser

Parses a restricted C-like grammar.

MVP statements:

```text
variable declarations
assignment
if / else
while
for
return
expression statements
kernel launches
```

MVP expressions:

```text
literals
variables
function calls
binary arithmetic
comparisons
boolean operators
array indexing for fixed local arrays
CUDA-style launch syntax
```

### 27.3 Semantic analysis

Build symbol tables and enforce restrictions.

Checks:

```text
__kernel functions cannot call await/spawn
__host tasks cannot directly use job_index()
device_read field names are valid LogicFields
discover() arguments are compile-time strings
kernel return type is representable in one or known number of cells
future types match kernel return types
unsupported C features are rejected cleanly
```

### 27.4 Effect analysis

Track whether functions read devices, write devices, read shared memory, write shared memory, launch jobs, or block.

This matters for scheduling and safe parallelism.

Effect examples:

```text
pure arithmetic
device read
device write
shared read
shared write
host blocking
kernel launch
```

### 27.5 Kernel partitioning

The compiler identifies all `__kernel` functions and emits each as a standalone ROM segment.

For each kernel:

```text
assign kernelId
emit bytecode
record romBank/localPc
record argument ABI
record return ABI
```

### 27.6 Host lowering

The compiler lowers host tasks into scheduler-aware bytecode.

A call like:

```c
Future<float> partialSums =
    SumBatteryRatioChunk<<<jobCount>>>(batteries, 8);
```

becomes:

```text
CREATE_FUTURE resultType=float count=jobCount
FOR jobIndex in 0..jobCount-1:
    CREATE_JOB kernel=SumBatteryRatioChunk
    SET_JOB_ARG0 batteries
    SET_JOB_ARG1 8
    SET_JOB_INDEX jobIndex
    SET_JOB_COUNT jobCount
    ENQUEUE_JOB
STORE future handle
```

A call like:

```c
float total = await_sum(partialSums);
```

becomes:

```text
AWAIT_FUTURE future
REDUCE_SUM future.resultBuffer -> total
```

---

## 28. Bytecode verifier

Before emitting ROM, verify:

```text
all jumps target valid instruction boundaries
all opcodes have legal operands
kernel does not use host-only opcodes
host does not use kernel-only opcodes unless allowed
all local slots are within frame size
all arg indices are valid
all result writes match return type
no ROM segment exceeds configured bank assumptions unless split
```

This verifier is essential because debugging inside Stationeers is painful.

---

## 29. Simulator

Build simulator layers before relying on in-game testing.

There are three simulator layers.

### 29.1 Standalone IC10 simulator

The IC10 simulator executes one IC10 program in isolation. It is the lowest simulator layer and should be useful on its own for testing handwritten firmware.

It should model:

```text
IC10 source parsing
labels and program counters
registers
numeric constants
basic arithmetic
branches and jumps
yield behavior
stack operations
simple device port abstractions
instruction counting
faults and traces
```

The IC10 simulator should expose an API that can execute one instruction at a time, run until `yield`, or run until an instruction budget is exhausted. When driven by the world simulator, the normal per-tick budget is 128 IC10 instructions.

Standalone command:

```bash
stationc sim ic10 firmware/worker_firmware.ic10 --ticks 10 --trace
```

### 29.2 World simulator

The world simulator executes a small, deterministic subset of Stationeers. It composes multiple IC10 chips and external devices into a ticked simulation.

It should model:

```text
world ticks
multiple IC10 housings
IC housing pins d0 through d5 and db
per-chip stack memory
device stack memory
data networks
ReferenceId addressing
PrefabHash and NameHash classification
logic fields
device reads and writes
scenario-defined mock device behavior
```

The world tick loop should run each active IC10 chip for up to 128 instructions or until it yields, then advance the subset of mock device behavior declared by the scenario. The world simulator should not attempt to recreate Stationeers' full device simulation. It needs to be deterministic, inspectable, and accurate enough to test firmware assumptions about timing, stacks, devices, and multi-chip communication.

The default world simulator should use a stable sequential IC10 order for
repeatable tests. It should also support alternative deterministic schedules
such as rotating order and seeded shuffle order. Those modes are not claims
about the exact game implementation; they are tools for finding firmware and
compiler assumptions that accidentally depend on a particular inter-IC order.

The world simulator should trace world-facing accesses:

```text
tick
actor IC ReferenceId
operation: read or write
target: device logic field or device stack address
```

Debug diagnostics should flag at least:

1. Multiple writes to the same world target in one tick.
2. Read/write overlap on the same world target in one tick.
3. Repeated volatile reads of the same logic field by the same IC in one tick.

These diagnostics are warnings for simulator and compiler development. They do
not mean every repeated read is forbidden; they identify code that needs an
explicit sampling decision.

Standalone command:

```bash
stationc sim world examples/worlds/basic.world.toml --ticks 100 --trace
```

### 29.3 StationOS simulator

The StationOS simulator is the highest simulator layer. It validates the compiler output and distributed runtime behavior using the world and IC10 simulator layers below it.

It should model:

```text
ROM banks
RAM banks
worker VM execution
host scheduler
job queue
futures
device groups
mock devices and logic fields
instruction budget per tick
yield behavior
```

StationOS simulator features:

```text
run bytecode without IC10
simulate N workers
simulate device reads/writes
simulate command buffers
trace job execution
count VM instructions
estimate IC10 instruction budget
detect races in shared writes
detect volatile read patterns that look like accidental repeated samples
```

StationOS command:

```bash
stationc sim stationos examples/battery_average.sc10 --workers 16 --ticks 100 --trace
```

Output:

```text
ticks: 100
workers: 16
jobs launched: 800
jobs completed: 800
average worker utilization: 74%
host queue stalls: 3
estimated IC10 instructions/tick/worker: 93
```

---

## 30. Firmware templates

The IC10 firmware should be generated from templates.

Templates:

```text
worker.vm.ic10.tera
host.kernel.ic10.tera
bootstrap.ic10.tera
rom_bank_idle.ic10.tera
ram_bank_idle.ic10.tera
```

Keep handwritten IC10 as small as possible. Use Rust code generation to generate:

```text
opcode dispatch chains
constant definitions
ABI offsets
debug labels
```

Generated IC10 must respect vanilla limits:

```text
<= 128 lines
<= 90 chars per line
<= 4096 bytes
```

The IC10 program-size constraints are documented by the Integrated Circuit wiki page. ([stationeers-wiki.com][4])

If a generated firmware exceeds the limit, the compiler must fail loudly.

---

## 31. ROM loader generation

A ROM loader writes compiled bytecode into a ROM-bank chip’s stack.

Because one IC10 script is limited, each ROM bank may need multiple loader pages.

Example output:

```text
rom_loader_000_page_000.ic10
rom_loader_000_page_001.ic10
rom_loader_000_page_002.ic10
```

Each loader page:

```ic10
poke 0  1
poke 1  0
poke 2  10
poke 3  0
...
s db Setting 1
done:
yield
j done
```

Workflow:

```text
1. Insert ROM chip.
2. Flash loader page 0.
3. Run once.
4. Flash loader page 1.
5. Run once.
6. Repeat.
7. Flash rom_bank_idle.ic10 or leave final loader idle.
```

The tooling should print clear instructions:

```text
ROM bank 0 requires 3 loader pages.
Flash them in order.
After each page runs, Setting will show the page number.
```

---

## 32. Runtime ABI details

### 32.1 Worker stack ABI

```text
0   WORKER_MAGIC
1   WORKER_VERSION
2   workerId
3   workerState
4   heartbeat
5   supportedFeatureMask

10  jobId
11  jobState
12  romBankRef
13  entryPc
14  endPc
15  contextPointer
16  jobIndex
17  jobCount
18  arg0
19  arg1
20  arg2
21  arg3
22  resultPointer
23  result0
24  result1
25  faultCode
```

### 32.2 Host runtime header

```text
0   RUNTIME_MAGIC
1   RUNTIME_VERSION
2   state
3   workerCount
4   romBankCount
5   ramBankCount
6   deviceCount
7   groupCount
8   currentTick
9   faultCode
```

### 32.3 Job table entry

```text
offset + 0   jobId
offset + 1   state
offset + 2   priority
offset + 3   kernelId
offset + 4   romBankRef
offset + 5   entryPc
offset + 6   endPc
offset + 7   jobIndex
offset + 8   jobCount
offset + 9   arg0
offset + 10  arg1
offset + 11  arg2
offset + 12  arg3
offset + 13  resultPointer
offset + 14  assignedWorkerId
offset + 15  faultCode
```

### 32.4 Device table entry

```text
offset + 0   referenceId
offset + 1   prefabHash
offset + 2   nameHash
offset + 3   flags
```

### 32.5 Group table entry

```text
offset + 0   groupId
offset + 1   prefabHash
offset + 2   nameHash
offset + 3   deviceStart
offset + 4   deviceCount
```

---

## 33. Example end-to-end program

Source:

```c
#define CHUNK_SIZE 8

__kernel float SumBatteryRatioChunk(DeviceGroup batteries, int chunkSize)
{
    int startIndex = job_index() * chunkSize;
    int endIndex = min(startIndex + chunkSize, group_count(batteries));

    float sum = 0.0f;

    for (int deviceIndex = startIndex; deviceIndex < endIndex; deviceIndex += 1)
    {
        DeviceRef battery = group_get(batteries, deviceIndex);
        sum += device_read(battery, Ratio);
    }

    return sum;
}

__host task Main()
{
    DeviceGroup batteries = discover("StructureBattery");
    DeviceGroup noncriticalLights = discover_named("StructureWallLight", "Noncritical");

    while (true)
    {
        refresh_discovery_if_needed();

        int batteryCount = group_count(batteries);
        int jobCount = ceil_div(batteryCount, CHUNK_SIZE);

        Future<float> partialSums =
            SumBatteryRatioChunk<<<jobCount>>>(batteries, CHUNK_SIZE);

        float total = await_sum(partialSums);
        float average = total / max(1, batteryCount);

        if (average < 0.25f)
        {
            batch_write(noncriticalLights, On, 0);
        }
        else
        {
            batch_write(noncriticalLights, On, 1);
        }

        sleep_ticks(1);
    }
}
```

Compiler output:

```text
Kernel table:
    kernel 0 SumBatteryRatioChunk -> ROM bank 1, pc 0

Host task table:
    task 0 Main -> ROM bank 0, pc 0

ROM:
    bank 0: host task bytecode
    bank 1: worker kernel bytecode

Firmware:
    host_firmware.ic10
    worker_firmware.ic10
    bootstrap.ic10
    rom_loader_000_page_000.ic10
    rom_loader_001_page_000.ic10
```

Runtime behavior:

```text
1. Bootstrap scans network.
2. Bootstrap finds workers by WORKER_MAGIC.
3. Bootstrap finds batteries and noncritical lights.
4. Bootstrap builds group tables.
5. Host starts Main.
6. Main launches one SumBatteryRatioChunk job per battery chunk.
7. Host assigns jobs to idle workers.
8. Workers execute ROM bank 1, pc 0 with different job_index values.
9. Host collects partial sums.
10. Host reduces sum.
11. Host batch-writes lights.
12. Host sleeps one tick.
```

---

## 34. Development phases

### Phase 0: Research and IC10 validation

Deliverables:

```text
manual IC10 test: one chip writes another chip stack
manual IC10 test: worker magic detected by bootstrap
manual IC10 test: ROM bank loaded with poke
manual IC10 test: worker fetches opcode with getd
manual IC10 test: ReferenceId network scan works
```

Do this before building the compiler.

### Phase 1: Standalone IC10 simulator

Implement:

```text
IC10 source parser
labels and program counters
register file
basic arithmetic
branches and jumps
yield
stack operations
instruction tracing
```

Goal: run a tiny IC10 program:

```text
move r0 1
move r1 2
add r2 r0 r1
yield
```

Run it with:

```bash
stationc sim ic10 examples/ic10/add.ic10 --trace
```

### Phase 2: Minimal world simulator

Implement:

```text
world tick loop
multiple IC10 housings
IC housing pins d0 through d5 and db
per-chip stacks
device stacks
simple data network
ReferenceId lookup
basic logic fields
128 instructions per IC per world tick
```

Goal: simulate two IC10 chips and one simple device communicating through stack and logic IO across world ticks.

### Phase 3: Minimal StationOS bytecode VM in Rust

Implement:

```text
bytecode opcodes
encoder/decoder
StationOS VM
ROM/RAM model
simple arithmetic kernels
```

Goal: compile or manually define bytecode for:

```text
return arg0 + arg1 * arg2
```

Run it in the StationOS simulator.

### Phase 4: Worker IC10 VM MVP

Implement fixed IC10 worker firmware supporting:

```text
LOAD_ARG
LOAD_IMM
ADD
SUB
MUL
DIV
STORE_RESULT
RETURN
FAULT
```

Manually load ROM bytecode into a ROM chip. Have one worker execute it and write a result.

### Phase 5: Host scheduler MVP

Implement host firmware that can:

```text
find one worker
assign one job
wait for done
read result
display result in Setting
```

No compiler yet.

### Phase 6: Bootstrap MVP

Implement bootstrap that scans network, finds workers by magic, assigns worker IDs, and writes worker table.

### Phase 7: Minimal StationC compiler

Implement parser/type checker/codegen for:

```text
__kernel functions
arithmetic
return
kernel launch from host
await
```

No devices yet.

### Phase 8: Device intrinsics

Add:

```text
discover
group_count
group_get
device_read
device_write
batch_read
batch_write
```

Simulator should support scenario-defined mock devices rather than built-in
Stationeers device semantics.

### Phase 9: ROM/RAM scaling

Add:

```text
multiple ROM banks
multiple RAM banks
global logical addresses
manifest-driven loader generation
```

### Phase 10: Futures, reductions, command buffers

Add:

```text
Future<T>
await_all
await_sum
await_min
await_max
CommandBuffer
command_buffer_apply
```

### Phase 11: Optimization

Add:

```text
constant folding
dead code elimination
common subexpression elimination
simple register allocation
bytecode packing
kernel chunk-size tuning
scheduler utilization metrics
```

### Phase 12: Real examples

Implement examples:

```text
battery average / load shedding
farm atmosphere controller
solar tracking chunker
furnace monitor
display dashboard
```

---

## 35. Engineering rules for the coding agent

A coding agent implementing this project should follow these rules.

### Rule 1: Build the simulator first

Do not rely on in-game testing for compiler correctness. The simulator should catch malformed bytecode, invalid jumps, ABI mismatches, and scheduler bugs.

### Rule 2: Keep the IC10 firmware tiny

The firmware is the hardest part to fit. Prefer simple bytecode formats over clever compression until the interpreter is proven.

### Rule 3: Make every generated artifact inspectable

Always emit:

```text
manifest.json
debug_map.json
disassembly.txt
program.rom.json
```

A user must be able to inspect where every function and global went.

### Rule 4: Fail loudly on unsupported C

Do not silently miscompile unsupported C. Reject unsupported constructs with precise diagnostics.

### Rule 5: Treat device writes as effects

The compiler and runtime should know when code writes devices. Parallel write behavior must be explicit.

### Rule 6: Prefer data-parallel jobs

The runtime scales best when jobs are independent chunks. Avoid designing kernels that require synchronization with each other.

### Rule 7: Use fixed-width bytecode first

Instruction packing can come later. The v1 interpreter should be clear and robust.

### Rule 8: Every table needs a version and magic number

Worker tables, runtime headers, ROM banks, and RAM banks should have recognizable magic/version cells so bootstrap can reject incompatible chips.

### Rule 9: Assume network order is unstable

Discovery must classify devices by `ReferenceId`, `PrefabHash`, and `NameHash`, not by network index. The docs warn that network scan order is not specifically determined. ([stationeers-wiki.com][5])

### Rule 10: Never assume IC stack memory is zeroed

The Integrated Circuit wiki says stack/register state can persist across events and does not automatically reset in the way many players expect. Initialization code must explicitly clear or write all required runtime cells. ([stationeers-wiki.com][4])

---

## 36. Known limitations

This system cannot literally make vanilla IC10 execute arbitrary IC10 source strings at runtime. There is no documented IC10 instruction for rewriting another chip’s program source. The documented workflow for loading source code is through the IC editor/computer/laptop export flow, not through IC10 instructions. The runtime instead executes bytecode stored as numeric stack data. ([stationeers-wiki.com][4])

The system is bounded by physical in-game hardware:

```text
number of ROM chips
number of RAM/heap chips
number of workers
instruction budget per worker
host scheduling overhead
data network/device update timing
```

The system will be slow compared to a real computer. But compared to one vanilla IC10 script, it can be much larger and more parallel.

The host can become the bottleneck. Therefore the scheduler must dispatch coarse jobs, not individual bytecode instructions.

The worker VM can become the bottleneck. Therefore the compiler should prefer fewer, chunkier kernels over tiny jobs.

Device state may update only on simulation ticks. Therefore the runtime should use phased control loops and explicit `sleep_ticks` / `yield` behavior.

---

## 37. What success looks like

The first real success demo should be:

```text
1 host IC
1 bootstrap IC
1 ROM bank IC
1 RAM bank IC
N worker ICs
several battery devices or mocked devices
```

The user writes:

```c
Future<float> partialSums =
    SumBatteryRatioChunk<<<jobCount>>>(batteries, 8);

float total = await_sum(partialSums);
```

The compiler emits firmware and loaders.

In game:

```text
with 1 worker:
    job completion is slow

with 4 workers:
    same program completes faster

with 16 workers:
    same program completes faster again
```

The high-level program is unchanged. Only the physical worker count changes.

That is the entire point of the project.

---

## 38. Minimal MVP scope

For the first implementation, do not build everything.

Build exactly this:

```text
StationC:
    __kernel
    __host task Main
    int / float / bool
    if / for / while / return
    arithmetic
    kernel launch <<<N>>>
    Future<T>
    await_sum

Bytecode:
    LOAD_IMM
    LOAD_ARG
    ADD/SUB/MUL/DIV
    LT/GT/EQ
    JUMP/JZ/JNZ
    STORE_RESULT
    RETURN

Runtime:
    one ROM bank
    one RAM bank
    many workers
    fixed worker mailbox
    host job dispatch
    StationOS simulator
    minimal world simulator

No devices yet.
```

Then add devices:

```text
DeviceRef
DeviceGroup
discover
group_count
group_get
device_read
batch_write
```

Then add command buffers.

This order minimizes risk.

---

## 39. Final architecture statement

This project is not a C-to-IC10 transpiler.

It is:

```text
a C-like CUDA-style language
compiled to StationOS bytecode
stored in IC stack ROM banks
executed by homogeneous IC10 worker VMs
scheduled by a host IC10 microkernel
with runtime hardware discovery and scalable worker pools
```

The key idea is:

```text
Do not make workers know special jobs.
Make every worker run the same simple VM.
Then assign workers ROM segments to execute.
```

That gives the desired opacity:

```text
Programmer writes one monolithic C-like program.
Compiler partitions it into host tasks and worker kernels.
Runtime discovers however many workers exist.
Host schedules bytecode jobs onto those workers.
Adding chips increases parallel throughput.
```


[1]: https://store.steampowered.com/app/544550/Stationeers/?utm_source=chatgpt.com "Stationeers on Steam"
[2]: https://stationeers-wiki.com/IC10/instructions "IC10/instructions - Stationeers Community Wiki"
[3]: https://stationeers-wiki.com/IC10 "IC10 - Stationeers Community Wiki"
[4]: https://stationeers-wiki.com/Integrated_Circuit_%28IC10%29 "Integrated Circuit (IC10) - Stationeers Community Wiki"
[5]: https://stationeers-wiki.com/Advanced_IC10_Programming "Advanced IC10 Programming - Stationeers Community Wiki"
[6]: https://steamcommunity.com/app/544550/discussions/0/628941283089764091/ "Question for IC10 and equipment switching :: Stationeers General Discussions"
