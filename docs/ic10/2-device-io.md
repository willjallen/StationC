# Device I/O

IC10 communicates with the world by loading values from devices into internal
registers and storing register values back to devices. This page covers direct
device pins, slot access, batch access, direct `ReferenceId` access, and cable
network channels.

## Device Pins

An IC housing exposes six configurable device pins:

```text
d0 d1 d2 d3 d4 d5
```

The chip can also use:

```text
db
```

`db` refers to the device where the chip is installed.

Set `d0` through `d5` with a screwdriver on the IC housing. The script cannot
select the devices by itself. Aliasing a device pin only changes how the pin is
displayed; it does not bind the pin.

```ic10
alias Door d0
s Door Open 0
```

The example is readable, but the player still needs to configure `d0` to the
actual door.

Device pins can also point at other IC housings. This lets multiple ICs
coordinate: one IC can publish a value such as `Setting`, and another IC can
read that value through its own configured device pin.

## Direct Load and Store

Use `l` to read a device logic value into a register.

```ic10
l r0 d0 Temperature
```

This reads `Temperature` from the device selected on `d0` and stores it in `r0`.

Use `s` to write a register or number to a device logic value.

```ic10
s d0 On r0
s d0 Open 0
```

The first writes the value of `r0` to `d0.On`. The second writes literal `0` to
`d0.Open`, which closes devices that use `Open` as a boolean.

Not every device supports every logic value. If a device does not have the
requested value, the IC will report an error. The Stationpedia and configuration
cartridge are the practical way to discover valid values for a specific device.

## Slot Load and Store

Some devices expose slots. Use `ls` and `ss` for slot logic values.

```ic10
ls r0 d0 2 Occupied
```

This reads the `Occupied` slot value from slot index `2` of the device on `d0`.

```ic10
alias robot d0
alias charge r10
ls charge robot 0 Charge
```

This reads an AIMeE-style robot charge value from slot `0` into `r10`.

General slot conventions:

| Slot | Common role |
| --- | --- |
| `0` | Import |
| `1` | Export |
| `2` | Inside machine |

Exceptions exist. Filtration units and other special devices can use slots
differently.

## Reagent Access

`lr` loads reagent quantities from a device using a reagent mode and reagent
hash.

Reagent modes:

| Mode | Number | Meaning |
| --- | --- | --- |
| `Contents` | `0` | Current contents. |
| `Required` | `1` | Required reagent amount. |
| `Recipe` | `2` | Recipe reagent amount. |

`rmap` maps a reagent hash to the prefab hash a device expects for that reagent.
For example, on an autolathe, an iron reagent hash can map to the prefab hash
for `ItemIronIngot`.

## Batch I/O

Batch instructions address devices by prefab hash instead of by `d0` through
`d5`. A prefab hash is an integer generated from the prefab name. Use
`HASH("Name")` or copy the hash from the Stationpedia.

```ic10
lb r0 HASH("StructureBattery") Ratio Average
sb HASH("StructureWallLight") On 1
```

All devices that can be read with logic contain at least `PrefabHash` and
`NameHash` logic values.

### Batch Read Modes

Batch reads use a mode to combine multiple matching device values.

| Name | Number | Result |
| --- | --- | --- |
| `Average` | `0` | Average of matching values. |
| `Sum` | `1` | Sum of matching values. |
| `Minimum` | `2` | Minimum matching value. |
| `Maximum` | `3` | Maximum matching value. |

The word or number can be used.

```ic10
lb r0 HASH("StructureWallLight") On Sum
lb r1 HASH("StructureGasSensor") Temperature 0
```

### Batch by Name

`lbn` and `sbn` add a name hash so only devices of a specific prefab and name
are affected.

```ic10
lbn r0 HASH("StructureGasSensor") HASH("Sensor 1") Temperature Average
sbn HASH("StructureWallLight") HASH("Grow Light") On 1
```

This lets scripts control more than six devices without configuring IC housing
pins, but it requires the devices to be named with the Labeller.

### Batch Slot I/O

Batch slot reads and stores operate on slot logic values:

```ic10
lbs r? deviceHash slotIndex logicSlotType batchMode
lbns r? deviceHash nameHash slotIndex logicSlotType batchMode
sbs deviceHash slotIndex logicSlotType r?
```

There is no `sbns` instruction.

### Batch Read With No Matching Devices

When a batch read has no matching devices, it returns:

| Batch mode | Result |
| --- | --- |
| `Average` (`0`) | `nan` |
| `Sum` (`1`) | `0` |
| `Minimum` (`2`) | `0` in version `0.2.6091.26702`; older behavior may have been `pinf`. |
| `Maximum` (`3`) | `ninf` |

This matters for scripts that use batch reads as discovery. For example, when
searching for a specific named device, a `Maximum` read returning `ninf` means no
matching device was found.

## Direct ReferenceId Access

Every device has a unique `ReferenceId` within a save. Direct reference
instructions are an alternative to housing pins and batch operations.

One pattern is:

```ic10
# Get the ReferenceId for the sorter named "Sorter Corn".
lbn r1 HASH("StructureLogicSorter") HASH("Sorter Corn") ReferenceId Maximum
ble r1 ninf ra

# Use the ReferenceId to set that sorter's mode.
sd r1 Mode 1
```

The direct reference instruction family is:

| Instruction | Purpose |
| --- | --- |
| `ld` | Load a logic value from a specific `ReferenceId`. |
| `sd` | Store a logic value to a specific `ReferenceId`. |
| `clrd` | Clear stack memory on a specific `ReferenceId`. |
| `getd` | Read stack memory from a specific `ReferenceId`. |
| `putd` | Write stack memory to a specific `ReferenceId`. |

Direct reference access does not include slot access by reference ID.

## Choosing Device Access

| Access style | Best use |
| --- | --- |
| `d0`-`d5` pins | Reusable scripts where the installer chooses devices with the IC housing screws. |
| `db` | Scripts installed directly in a device or needing the IC housing itself. |
| Batch prefab hash | Control every device of a type on the output network. |
| Batch prefab plus name hash | Control or discover named devices without configuring pins. |
| `ReferenceId` | Keep talking to one exact device after discovering its unique ID. |

Pins are easiest to understand and safest for small scripts. Batch and
`ReferenceId` access remove the six-device limit but depend on hashes, names,
and network layout.

## Network References and Channels

Cable networks expose eight volatile channel logic values:

```text
Channel0 Channel1 Channel2 Channel3 Channel4 Channel5 Channel6 Channel7
```

Channels are read and written through a device plus connection number:

```ic10
l r0 d0:0 Channel0
s db:1 Channel0 r0
```

For an IC housing:

- `db:0` is the data connection.
- `db:1` is the power connection.

Connection numbers are device-specific and should be checked in the Stationpedia.
All device connections can be pipes, chutes, or cables, but only cable networks
have channels.

Channel values are volatile:

- They are destroyed if any part of the cable network is changed, removed, or
  added.
- They are destroyed when the world is exited.
- They default to `nan`, specifically a quiet not-a-number value.

Use channels for communication between networks, not long-term storage.
An IC can use channel references on any cable network attached to a device it can
reference, not only the IC's own local data network.

## Reading All Channels

This dynamic logic type loop reads all eight channels into `r0` through `r7`:

```ic10
move r15 LogicType.Channel0 # LogicType integer
move r14 0                  # pointer for indirect referencing

loop:
l rr14 db:0 r15
add r15 r15 1               # next channel
add r14 r14 1               # next register
ble r15 LogicType.Channel7 loop
```

`rr14` is the indirect register destination. When `r14` is `0`, `rr14` means
`r0`; when `r14` is `1`, it means `r1`, and so on.
