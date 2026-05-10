# Grow Lab Spec

This directory is spec-first. The IC10 script only supports the devices listed
here. If the grow lab hardware changes, update this file first, then update the
script and scenario tests.

## Schema

Each row below follows this schema:

```text
label | prefab | hash | role | fields used
```

`hash` may be a literal Stationpedia value or the exact IC10 `HASH("...")`
expression used by the script.

## Housing

| label | prefab | hash | role | fields used |
| --- | --- | --- | --- | --- |
| GLCTRL | StructureCircuitHousing | -128473777 | grow lab light IC housing | self only; never written |

## Inputs

| label | prefab | hash | role | fields used |
| --- | --- | --- | --- | --- |
| GLDLSNSR | StructureDaylightSensor | 1076425094 | upward-facing daylight sensor | Mode, Activate, SolarAngle, ReferenceId |

## Controlled Devices

| label | prefab | hash | role | fields used |
| --- | --- | --- | --- | --- |
| GLHPSTN | StructureHydroponicsStation | 1441767298 | hydroponics stations with grow lights | On |

All controlled hydroponics stations must be named exactly `GLHPSTN`. The script
uses one named batch write and ignores other hydroponics stations.

## Sensor Contract

`GLDLSNSR` must face upward toward the sky. The script forces daylight sensor
`Mode = 0`, so `SolarAngle` is the default absolute angle relative to the
sensor face: `0` near noon, `90` near sunrise or sunset, and `180` near
midnight. Compass rotation and connector direction are not part of the contract.

Wall-mounted, ceiling-mounted, horizontal-mode, and vertical-mode sensors are
not supported.

## Light Rules

The default constants target Mars-like timing:

| name | value | behavior |
| --- | --- | --- |
| DayMinutes | 20 | full light/dark cycle length |
| LightMinutes | 12.5 | target total crop light per cycle |

The script computes:

```text
LightAngle = 180 * LightMinutes / DayMinutes
```

With the defaults, `LightAngle = 112.5`.

Because the crops are under a window, natural daylight counts as crop light.
When `GLDLSNSR.Activate > 0`, hydroponics station lights are off. When the
sensor is dark but `GLDLSNSR.SolarAngle < LightAngle`, all `GLHPSTN`
hydroponics stations are turned on to supplement the light window. When
`SolarAngle >= LightAngle`, all `GLHPSTN` hydroponics stations are turned off
to preserve the dark period.
