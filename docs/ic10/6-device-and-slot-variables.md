# Device and Slot Variables

Device variables are the `logicType` names used by `l`, `s`, `lb`, `sb`, and
related instructions. Slot variables are the `logicSlotType` names used by `ls`,
`ss`, `lbs`, `lbns`, and related instructions.

Availability is device-specific. Use the Stationpedia or a configuration
cartridge to confirm which values a device supports.

## Common Device Variables

| Variable | Meaning |
| --- | --- |
| `Activate` | `1` if the device is activated, usually meaning running; otherwise `0`. |
| `AirRelease` | Listed device variable. |
| `Charge` | Current charge the device has. |
| `ClearMemory` | Set to `1` to clear counter memory such as `ExportCount`; resets itself to `0` when triggered. |
| `CompletionRatio` | Listed device variable. |
| `ElevatorLevel` | Listed device variable. |
| `ElevatorSpeed` | Listed device variable. |
| `Error` | `1` if the device is in an error state; otherwise `0`. |
| `ExportCount` | Number of items exported since the last `ClearMemory`. |
| `Filtration` | Current filtration state; for example `1` when hardsuit filtration is on. |
| `Harvest` | Performs one harvest action on plant-based machinery when set. |
| `Horizontal` | Listed device variable. |
| `HorizontalRatio` | Listed device variable. |
| `Idle` | Listed device variable. |
| `ImportCount` | Listed device variable. |
| `Lock` | Listed device variable. |
| `Maximum` | Listed device variable. |
| `Mode` | Listed device variable. |
| `On` | Common boolean on/off value. |
| `Open` | Common boolean open/closed value. |
| `Output` | Listed device variable. |
| `Plant` | Performs one planting action on plant-based machinery when set. |
| `PositionX` | Listed device variable. |
| `PositionY` | Listed device variable. |
| `PositionZ` | Listed device variable. |
| `Power` | Listed device variable. |
| `PowerActual` | Listed device variable. |
| `PowerPotential` | Listed device variable. |
| `PowerRequired` | Listed device variable. |
| `Pressure` | Pressure, usually in kPa for atmospherics. |
| `PressureExternal` | Listed device variable. |
| `PressureInteral` | Listed with this spelling in IC10. |
| `PressureSetting` | Listed device variable. |
| `Quantity` | Total quantity in the device. |
| `Ratio` | Context-specific ratio from `0` to `1`. |
| `RatioCarbonDioxide` | Carbon dioxide ratio in a device atmosphere. |
| `RatioNitrogen` | Nitrogen ratio in a device atmosphere. |
| `RatioOxygen` | Oxygen ratio in a device atmosphere. |
| `RatioPollutant` | Pollutant ratio in a device atmosphere. |
| `RatioVolatiles` | Volatiles ratio in a device atmosphere. |
| `RatioWater` | Water ratio in a device atmosphere. |
| `Reagents` | Listed device variable. |
| `RecipeHash` | Listed device variable. |
| `ReferenceId` | Unique identifier for a device within a save. |
| `RequestHash` | Listed device variable. |
| `RequiredPower` | Listed device variable. |
| `Setting` | Meaning varies by device; LED displays and consoles show this value. |
| `SolarAngle` | Solar angle of the device. |
| `Temperature` | Temperature, in Kelvin. Celsius is Kelvin minus `273.15`. |
| `TemperatureSettings` | Listed device variable. |
| `TotalMoles` | Listed device variable. |
| `VelocityMagnitude` | Listed device variable. |
| `VelocityRelativeX` | Listed device variable. |
| `VelocityRelativeY` | Listed device variable. |
| `VelocityRelativeZ` | Listed device variable. |
| `Vertical` | Vertical setting of the device. |
| `VerticalRatio` | Ratio of vertical setting for the device. |
| `Volume` | Device atmosphere volume. |

Examples:

```ic10
l r0 d0 Activate # r0 = 1 if active, otherwise 0
s d0 Harvest 1   # perform one harvest action
s d0 Plant 1     # plant one crop
l r0 d0 SolarAngle
```

## Data Network Colors

`Data Network` values correspond to physical spray-can colors and can be used by
IC10 scripts or logic circuits to set colors on components such as lights.

| Value | Color | Hex |
| --- | --- | --- |
| `0` | Blue | `#212AA5` |
| `1` | Gray | `#7B7B7B` |
| `2` | Green | `#3F9B39` |
| `3` | Orange | `#FF662B` |
| `4` | Red | `#E70200` |
| `5` | Yellow | `#FFBC1B` |
| `6` | White | `#E7E7E7` |
| `7` | Black | `#080908` |
| `8` | Brown | `#633C2B` |
| `9` | Khaki | `#63633F` |
| `10` | Pink | `#E41C99` |
| `11` | Purple | `#732CA7` |

## Slot Variables

General slot convention:

| Slot | Common role |
| --- | --- |
| `0` | Import |
| `1` | Export |
| `2` | Inside machine |

Exceptions exist, especially for filtration units.

| Slot variable | Meaning |
| --- | --- |
| `Occupied` | `1` when the slot is occupied, otherwise `0`. |
| `OccupantHash` | Listed slot variable. |
| `Quantity` | Quantity in the slot. |
| `Damage` | Listed slot variable. |
| `Efficiency` | Listed slot variable. |
| `FilterType` | Type of filter installed in the slot. |
| `Health` | Listed slot variable. |
| `Growth` | Numerical growth stage for crops. |
| `Pressure` | Listed slot variable. |
| `Temperature` | Listed slot variable. |
| `Charge` | Listed slot variable. |
| `ChargeRatio` | Listed slot variable. |
| `Class` | Listed slot variable. |
| `PressureWaste` | Listed slot variable. |
| `PressureAir` | Listed slot variable. |
| `MaxQuantity` | Listed slot variable. |
| `Mature` | `1` when a crop is mature, otherwise `0`. |
| `ReferenceId` | Unique identifier for a device within a save. |

Examples:

```ic10
ls r0 d0 2 Occupied
ls vOccupied dThisVictim 2 Occupied

ls r0 d0 0 Growth

ls r0 d0 0 Mature
ls vMature dThisVictim 0 Mature
```

## FilterType Values

`FilterType` reports the kind of filter installed in a filtration slot.

| Value | Filter |
| --- | --- |
| `1` | Oxygen |
| `2` | Nitrogen |
| `4` | Carbon Dioxide |
| `8` | Volatiles |
| `16` | Pollutants |
| `32` | Water |
| `64` | Nitrous Oxide |
| `16384` | Hydrogen |
| `65536` | Polluted Water |
| `131072` | Hydrazine |
| `524288` | Alcohol |
| `1048576` | Helium |
| `2097152` | Liquid Sodium Chloride |
| `4194304` | Silanol |
| `16777216` | Hydrochloric Acid |
| `67108864` | Ozone |

Example:

```ic10
ls r0 db 0 FilterType
```
