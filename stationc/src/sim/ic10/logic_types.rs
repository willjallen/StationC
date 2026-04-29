//! Built-in IC10 logic type constants.

const LOGIC_TYPE_PREFIX: &str = "LogicType.";

const LOGIC_TYPES: &[&str] = &[
    "Activate",
    "Charge",
    "ClearMemory",
    "CompletionRatio",
    "Error",
    "ExportCount",
    "Horizontal",
    "HorizontalRatio",
    "ImportCount",
    "Lock",
    "Maximum",
    "Mode",
    "On",
    "Open",
    "Power",
    "PowerActual",
    "PowerPotential",
    "PowerRequired",
    "Pressure",
    "PressureExternal",
    "PressureInteral",
    "PressureSetting",
    "Ratio",
    "RatioCarbonDioxide",
    "RatioNitrogen",
    "RatioOxygen",
    "RatioPollutant",
    "RatioVolatiles",
    "RatioWater",
    "ReferenceId",
    "RequiredPower",
    "Setting",
    "Temperature",
    "TemperatureSettings",
    "Vertical",
    "VerticalRatio",
    "Channel0",
    "Channel1",
    "Channel2",
    "Channel3",
    "Channel4",
    "Channel5",
    "Channel6",
    "Channel7",
];

pub(super) fn value_from_symbol(symbol: &str) -> Option<f64> {
    let name = symbol.strip_prefix(LOGIC_TYPE_PREFIX)?;
    let index = LOGIC_TYPES
        .iter()
        .position(|candidate| *candidate == name)?;
    #[allow(clippy::cast_precision_loss)]
    Some(index as f64)
}

pub(super) fn name_from_value(value: f64) -> Option<&'static str> {
    if !value.is_finite() || value.fract() != 0.0 || value < 0.0 {
        return None;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let index = value as usize;
    LOGIC_TYPES.get(index).copied()
}
