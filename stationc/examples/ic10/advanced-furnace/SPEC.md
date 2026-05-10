# Advanced Furnace Panel Spec

This directory is spec-first. The IC10 scripts only support the devices listed
here. If the panel hardware changes, update this file first, then update the
scripts and scenario tests.

## Schema

Each row below follows this schema:

```text
label | prefab | hash | role | fields used
```

`hash` may be a literal Stationpedia value or the exact IC10 `HASH("...")`
expression used by the script.

## Housings

| label | prefab | hash | role | fields used |
| --- | --- | --- | --- | --- |
| AFCTRL | StructureCircuitHousing | -128473777 | control IC housing | On |
| AFSAFETY | StructureCircuitHousing | -128473777 | always-on safety IC housing | self only; never written |
| AFVIZLED | StructureCircuitHousing | -128473777 | reagent LED IC housing | On |
| AFVIZNUM | StructureCircuitHousing | -128473777 | numeric display IC housing | On |

Compact IC housings are not part of this spec. `AFSAFETY` is not controlled by
`AFMASTER`.

## Controlled Devices

| label | prefab | hash | role | fields used |
| --- | --- | --- | --- | --- |
| AF | StructureAdvancedFurnace | 545937711 | furnace | On, Activate, Open, SettingInput, SettingOutput, Temperature, Pressure, Contents reagents |
| AFVNT | StructureActiveVent | HASH("StructureActiveVent") | furnace vent | On |

## Inputs

| label | prefab | hash | role | fields used |
| --- | --- | --- | --- | --- |
| AFMASTER | StructureLogicSwitch2 | 321604921 | master power switch | Open |
| AFACT | StructureLogicButton | 491845673 | furnace activate button | Activate |
| AFMOLD | StructureLogicSwitch | 1220484876 | mold lever | Open |
| AFVNTL | StructureLogicSwitch | 1220484876 | vent lever | Open |
| AFGIN | StructureLogicDial | 554524804 | gas input setting | Setting |
| AFGOUT | StructureLogicDial | 554524804 | gas output setting | Setting |
| AFSFTRST | StructureLogicButton | 491845673 | safety trip reset button | Activate |
| AFSFTTST | StructureLogicButton | 491845673 | safety test button | Activate |

## Reagent Rows

| label | prefab | hash | role | fields used |
| --- | --- | --- | --- | --- |
| AF1L | LED | 1944485013 | row 1 reagent color | On, Color |
| AF2L | LED | 1944485013 | row 2 reagent color | On, Color |
| AF3L | LED | 1944485013 | row 3 reagent color | On, Color |
| AF4L | LED | 1944485013 | row 4 reagent color | On, Color |
| AF1Q | StructureConsoleLED1x2 | HASH("StructureConsoleLED1x2") | row 1 quantity | On, Setting, Mode |
| AF2Q | StructureConsoleLED1x2 | HASH("StructureConsoleLED1x2") | row 2 quantity | On, Setting, Mode |
| AF3Q | StructureConsoleLED1x2 | HASH("StructureConsoleLED1x2") | row 3 quantity | On, Setting, Mode |
| AF4Q | StructureConsoleLED1x2 | HASH("StructureConsoleLED1x2") | row 4 quantity | On, Setting, Mode |

Only medium LED displays are part of this spec.

## Status Displays

| label | prefab | hash | role | fields used |
| --- | --- | --- | --- | --- |
| AFT | StructureConsoleLED1x2 | HASH("StructureConsoleLED1x2") | furnace temperature | On, Setting, Mode |
| AFP | StructureConsoleLED1x2 | HASH("StructureConsoleLED1x2") | furnace pressure | On, Setting, Mode |
| AFIN | StructureConsoleLED1x2 | HASH("StructureConsoleLED1x2") | gas input setting display | On, Setting, Mode |
| AFOUT | StructureConsoleLED1x2 | HASH("StructureConsoleLED1x2") | gas output setting display | On, Setting, Mode |

## Room Lights

| label | prefab | hash | role | fields used |
| --- | --- | --- | --- | --- |
| AFLIGHT | LED | 1944485013 | furnace-room indicator light | On |
| AFLIGHT2 | StructureLight | -1860064656 | furnace-room light | On |

## Safety Devices

| label | prefab | hash | role | fields used |
| --- | --- | --- | --- | --- |
| AFTRIP | LED | 1944485013 | latched safety trip indicator | On |
| AFSFTALRT | StructureFlashingLight | -1535893860 | room-visible safety alert | On |

## Safety Rules

`AFSAFETY` must remain powered while the furnace panel exists. It never writes
`AF.On`.

| name | value | behavior |
| --- | --- | --- |
| MaxTemperature | 2273 | trips safety when `AF.Temperature` is greater than this |
| MaxPressure | 180000 | trips safety when `AF.Pressure` is greater than this |
| ResetTemperature | 2073 | reset is allowed only at or below this temperature |
| ResetPressure | 120000 | reset is allowed only at or below this pressure |
| VentOpen | 100 | safe furnace output setting |

When `AFMASTER` is off or safety is tripped, the panel forces
`AF.Activate = 0`, `AF.Open = 0`, `AF.SettingInput = 0`, and
`AF.SettingOutput = 100`.

`AFSFTTST` enters the same latched trip path as unsafe temperature or
pressure. While the trip latch is active, `AFTRIP` and `AFSFTALRT` are on,
and `AFCTRL` is off so it cannot fight `AFSAFETY`. Visualization chips remain
on while safety is tripped.

When `AFSFTRST` is pressed while temperature and pressure are both below their
reset thresholds, the safety chip clears `AFTRIP`, `AFSFTALRT`,
`AF.Activate`, `AF.Open`, `AF.SettingInput`, and `AF.SettingOutput` to `0`,
then turns `AFCTRL` back on.

`AFCTRL` also treats `AFSFTRST` as a zero-output hold so it cannot reapply dial
settings in the same tick that `AFSAFETY` re-enables it.

## Reagent Scan Order And Colors

Rows are filled in this fixed scan order:

| reagent | color id |
| --- | --- |
| Iron | 4 |
| Hydrocarbon | 7 |
| Carbon | 8 |
| Copper | 3 |
| Cobalt | 0 |
| Gold | 5 |
| Nickel | 2 |
| Silver | 6 |
| Lead | 1 |
| Silicon | 9 |
| Steel | 4 |

If more than four tracked reagents are present, all four reagent LEDs use color
`4` and `AF1Q` displays the count of detected tracked reagents.
