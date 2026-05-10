# Solar Tracker IC10 Spec

This project defines a reusable two-axis solar tracker for programmable solar
panels. It is intentionally independent of any base, outpost, or power-storage
design.

## Script

| file | housing | role |
| --- | --- | --- |
| `sttrack.ic10` | `STTRACKIC` | Tracks all named solar panels from one daylight sensor. |

## Labels

| label | count | device |
| --- | ---: | --- |
| `STTRACKIC` | 1 | solar tracker IC housing |
| `STDLSNSR` | 1 | daylight sensor |
| `STSP` | 1+ | tracked solar panels |

## Device Fields

| target | prefab(s) | fields |
| --- | --- | --- |
| `STDLSNSR` | `StructureDaylightSensor` | `Horizontal`, `Vertical`, `ReferenceId` |
| `STSP` | `StructureSolarPanel`, `StructureSolarPanelDual` | `On`, `Horizontal`, `Vertical` |

## Behavior

- Reads `STDLSNSR.Horizontal` and `STDLSNSR.Vertical`.
- Writes `Horizontal`, clamped `Vertical`, and `On = 1` to all `STSP` panels.
- Supports normal and dual programmable solar panel prefab names.
- Missing `STDLSNSR` leaves panels unchanged and writes `STTRACKIC.Setting = -1`.
- Normal operation writes the applied vertical panel angle to
  `STTRACKIC.Setting`.

## Constants

| constant | default | purpose |
| --- | ---: | --- |
| `HorizontalOffset` | 0 | build-orientation correction for panel yaw |
| `VerticalOffset` | 90 | converts sensor vertical reading to panel pitch |
| `VerticalMin` | 15 | lower clamp for panel pitch |
| `VerticalMax` | 165 | upper clamp for panel pitch |

## Deployment Notes

The daylight sensor should be mounted in the orientation expected by the
script. If the physical panel rack is rotated relative to the sensor, adjust
`HorizontalOffset` in the script.

## Validation

Repository validation targets:

- all `.ic10` files parse with `stationc sim ic10 <file> --ticks 0`
- every script stays under 128 lines
- every script line stays at or below 90 characters

Current script sizes:

| file | lines |
| --- | ---: |
| `sttrack.ic10` | 31 |
