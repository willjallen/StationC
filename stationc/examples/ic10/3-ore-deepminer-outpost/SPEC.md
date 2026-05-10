# 3-Ore Deep Miner Outpost IC10 Spec

This project defines the IC10 automation local to one remote Mars deep-miner
outpost. Reusable solar tracking is intentionally split into
[`../solar-tracker/SPEC.md`](../solar-tracker/SPEC.md).

## Target Build

- 1 electric deep miner.
- 1 known three-ore deep-mining region.
- 3 electric centrifuges.
- 3 ore sorters, 3 ore stackers, 3 ore SDB silos.
- 1 dirty ore SDB buffer and 1 reject/misc SDB silo.
- 1 local light group for maintenance/collection visits.
- 1 proximity sensor for approach/occupancy lighting.

The outpost uses the separate `solar-tracker` project for enclosed programmable
solar panels. Stacker settings are manual: set each ore stacker to `Mode = 0`,
`Setting = 50`, and `On = 1`.

## Topology

```text
POWER

  [solar-tracker project]
          |
          v
  [station batteries / manual transformer]
          |
          v
  [DMOMINE] [DMOCF x3] [sorters] [stackers] [silos] [lights]


MATERIAL

  DMOMINE -> DMODIRTY -> DMOCF d0/d1/d2 -> mixed ore

  mixed ore -> DMOSORA -> DMOSTKA -> DMOOREA
          overflow -> DMOSORB -> DMOSTKB -> DMOOREB
          overflow -> DMOSORC -> DMOSTKC -> DMOOREC
          overflow -> DMOREJ
```

## Scripts

| file | housing | role |
| --- | --- | --- |
| `dmocent.ic10` | `DMOCENTIC` | Controls the three centrifuge eject cycles. |
| `dmostation.ic10` | `DMOSTATIONIC` | Turns local lights on when a player is nearby. |

There is no power-management IC in this version. The power system is a fixed
budget: solar generation, storage, and process load are sized at build time.
A future power IC is only justified if measured play shows brownout oscillation
or a need for remote/manual process lockout.

## Labels

Repeated labels are intentional batch targets.

| label | count | device |
| --- | ---: | --- |
| `DMOCENTIC` | 1 | centrifuge IC housing |
| `DMOSTATIONIC` | 1 optional | station lighting IC housing |
| `DMOMINE` | 1 | deep miner |
| `DMOCF` | 3 | centrifuges |
| `DMODIRTY` | 1 | dirty ore SDB buffer |
| `DMOSORA` | 1 | Ore A sorter |
| `DMOSORB` | 1 | Ore B sorter |
| `DMOSORC` | 1 | Ore C sorter |
| `DMOSTKA` | 1 | Ore A stacker |
| `DMOSTKB` | 1 | Ore B stacker |
| `DMOSTKC` | 1 | Ore C stacker |
| `DMOOREA` | 1 | Ore A SDB silo |
| `DMOOREB` | 1 | Ore B SDB silo |
| `DMOOREC` | 1 | Ore C SDB silo |
| `DMOREJ` | 1 | reject/misc SDB silo |
| `DMOPROX` | 1 | proximity sensor, direct-pinned to `DMOSTATIONIC.d0` |
| `DMOLIGHT` | 1+ | local visit/maintenance lights |

## Device Fields

| target | prefab(s) | fields |
| --- | --- | --- |
| `DMOCF` | `StructureCentrifuge` | `On`, `Open`, `Reagents` |
| `DMOPROX` | proximity sensor, direct pin | `Activate`, `Setting` |
| `DMOLIGHT` | `LED`, `StructureLight` | `On` |

Sorter filters are configured manually with a computer and sorter motherboard.
Runtime IC10 does not configure ore filters.

## Network Contract

`DMOCENTIC` uses direct pins:

| pin | device |
| --- | --- |
| `d0` | first centrifuge |
| `d1` | second centrifuge |
| `d2` | third centrifuge |
| `d3` | unused |
| `d4` | unused |
| `d5` | unused |

`DMOSTATIONIC` uses direct pins:

| pin | device |
| --- | --- |
| `d0` | proximity sensor |
| `d1`-`d5` | unused |

`DMOSTATIONIC` must also see all lights labelled `DMOLIGHT` on its data
network.

## `dmocent.ic10`

Controls the centrifuges assigned to `d0`, `d1`, and `d2`.

Constants:

| constant | default | purpose |
| --- | ---: | --- |
| `FullThreshold` | 380 | opens a closed centrifuge at or above this reagent count |
| `EmptyThreshold` | 0 | closes an open centrifuge at or below this reagent count |

Behavior:

- Writes `On = 1` to all three centrifuges each tick.
- If a centrifuge is already open, that centrifuge is handled before any new
  centrifuge is opened.
- At most one centrifuge is intentionally open at a time.
- Closed centrifuges are checked in pin order: `d0`, then `d1`, then `d2`.
- Missing direct pin writes `DMOCENTIC.Setting = -1`.

Debug values:

| `DMOCENTIC.Setting` | meaning |
| ---: | --- |
| `0` | all centrifuges closed and below threshold |
| `10` | `d0` is open/ejecting |
| `11` | `d1` is open/ejecting |
| `12` | `d2` is open/ejecting |
| `-1` | at least one required centrifuge pin is unset |

## `dmostation.ic10`

Turns the local light group on while the proximity sensor detects a player.

Constants:

| constant | default | purpose |
| --- | ---: | --- |
| `RangeMeters` | 50 | proximity sensor detection range |

Behavior:

- Writes `DMOPROX.Setting = 50`.
- Reads `DMOPROX.Activate`.
- Writes `On` to all `LED` and `StructureLight` devices named `DMOLIGHT`.
- Writes `DMOSTATIONIC.Setting = 1` when lights are on, `0` when off.
- Missing proximity sensor turns lights off and writes `DMOSTATIONIC.Setting = -1`.

The proximity sensor detects players in a configurable spherical range. This is
the preferred approach for lighting the outpost from outside; a motion sensor is
grid-cell sized and is better suited to doors or small trigger zones.

## Manual Setup

Stackers:

```text
DMOSTKA: Mode = 0, Setting = 50, On = 1
DMOSTKB: Mode = 0, Setting = 50, On = 1
DMOSTKC: Mode = 0, Setting = 50, On = 1
```

Sorter cascade:

```text
mixed ore
  -> DMOSORA main -> DMOSTKA -> DMOOREA
  -> DMOSORA overflow -> DMOSORB
  -> DMOSORB main -> DMOSTKB -> DMOOREB
  -> DMOSORB overflow -> DMOSORC
  -> DMOSORC main -> DMOSTKC -> DMOOREC
  -> DMOSORC overflow -> DMOREJ
```

Solar:

Use the reusable `solar-tracker` project and its labels (`STTRACKIC`,
`STDLSNSR`, `STSP`) for the enclosed solar array.

## Validation

Repository validation targets:

- all `.ic10` files parse with `stationc sim ic10 <file> --ticks 0`
- every script stays under 128 lines
- every script line stays at or below 90 characters

Current script sizes:

| file | lines |
| --- | ---: |
| `dmocent.ic10` | 79 |
| `dmostation.ic10` | 21 |
