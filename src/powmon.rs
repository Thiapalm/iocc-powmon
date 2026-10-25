use crate::comm::*;
use crate::tools::*;
use serde::Deserialize;

const INTERNAL_SCALING: f64 = 0.00512;
const SHUNT_LSB: f64 = 0.0000025; // in Volts (2.5 uV)
const VOLTAGE_LSB: f64 = 0.00125; // in Volts (1.25 mV)
const POWER_LSB_FACTOR: f64 = 25.0; // power LSB = 25 * current LSB
const SHUNT_RESISTOR: f64 = 0.002; // in Ohms (2 mOhm)

const MANUFACTURER: u16 = 0x5449;
const DIE_ID: u16 = 0x2260;

/// Enum used to identify ina226 registers
#[allow(dead_code)]
enum Register {
    Configuration = 0x00,
    ShuntVoltage = 0x01,
    BusVoltage = 0x02,
    Power = 0x03,
    Current = 0x04,
    Calibration = 0x05,
    MaskEnable = 0x06,
    Alert = 0x07,
    Manufacturer = 0xFE,
    DieId = 0xFF,
}

/// Enum used to determine the number of averages to be used
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum InaAverage {
    _1 = 0x00,
    _4 = 0x01,
    #[default]
    _16 = 0x02,
    _64 = 0x03,
    _128 = 0x04,
    _256 = 0x05,
    _512 = 0x06,
    _1024 = 0x07,
}

/// Set the conversion time for the Bus voltage measurement
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InaVbusct {
    _140_us = 0x00,
    _204_us = 0x01,
    _332_us = 0x02,
    _588_us = 0x03,
    #[default]
    _1_1_ms = 0x04,
    _2_116_ms = 0x05,
    _4_156_ms = 0x06,
    _8_244_ms = 0x07,
}

/// Set the conversion time for the Shunt Bus voltage measurement
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InaVshct {
    _140_us = 0x00,
    _204_us = 0x01,
    _332_us = 0x02,
    _588_us = 0x03,
    #[default]
    _1_1_ms = 0x04,
    _2_116_ms = 0x05,
    _4_156_ms = 0x06,
    _8_244_ms = 0x07,
}

/// Used to set the ina226 mode of operation
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Default)]
pub enum InaMode {
    PowerDown = 0x00,
    ShuntVoltageTriggered = 0x01,
    BusVoltageTriggered = 0x02,
    ShuntAndBusTriggered = 0x03,
    PowerDown2 = 0x04,
    ShuntVoltageContinuous = 0x05,
    #[default]
    BusVoltageContinuous = 0x06,
    ShuntAndBusContinuous = 0x07,
}

/// Used to choose the function of alert pin
#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MaskEnable {
    ShuntOverVoltage = 1 << 15,
    ShuntUnderVoltage = 1 << 14,
    BusOverVoltage = 1 << 13,
    BusUnderVoltage = 1 << 12,
    PowerOverLimit = 1 << 11,
    ConversionReady = 1 << 10,
    AlertFunctionFlag = 1 << 4,
    ConversionReadyFlag = 1 << 3,
    MathOverflowFlag = 1 << 2,
    AlertPolarityBit = 1 << 1,
    AlertLatchEnable = 1,
}

impl InaMode {
    pub const ALL: [InaMode; 8] = [
        InaMode::PowerDown,
        InaMode::ShuntVoltageTriggered,
        InaMode::BusVoltageTriggered,
        InaMode::ShuntAndBusTriggered,
        InaMode::PowerDown2,
        InaMode::ShuntVoltageContinuous,
        InaMode::BusVoltageContinuous,
        InaMode::ShuntAndBusContinuous,
    ];

    pub fn name(self) -> &'static str {
        match self {
            InaMode::PowerDown => "PowerDown",
            InaMode::ShuntVoltageTriggered => "ShuntVoltageTriggered",
            InaMode::BusVoltageTriggered => "BusVoltageTriggered",
            InaMode::ShuntAndBusTriggered => "ShuntAndBusTriggered",
            InaMode::PowerDown2 => "PowerDown2",
            InaMode::ShuntVoltageContinuous => "ShuntVoltageContinuous",
            InaMode::BusVoltageContinuous => "BusVoltageContinuous",
            InaMode::ShuntAndBusContinuous => "ShuntAndBusContinuous",
        }
    }
}

impl std::str::FromStr for InaMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        InaMode::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(s.trim()))
            .ok_or_else(|| {
                let names: Vec<&str> = InaMode::ALL.iter().map(|m| m.name()).collect();
                Error::InvalidValue(format!(
                    "unknown mode '{s}', valid modes: {}",
                    names.join(", ")
                ))
            })
    }
}

impl InaAverage {
    pub const ALL: [InaAverage; 8] = [
        InaAverage::_1,
        InaAverage::_4,
        InaAverage::_16,
        InaAverage::_64,
        InaAverage::_128,
        InaAverage::_256,
        InaAverage::_512,
        InaAverage::_1024,
    ];

    pub fn label(self) -> &'static str {
        match self {
            InaAverage::_1 => "1",
            InaAverage::_4 => "4",
            InaAverage::_16 => "16",
            InaAverage::_64 => "64",
            InaAverage::_128 => "128",
            InaAverage::_256 => "256",
            InaAverage::_512 => "512",
            InaAverage::_1024 => "1024",
        }
    }
}

impl InaVbusct {
    pub const ALL: [InaVbusct; 8] = [
        InaVbusct::_140_us,
        InaVbusct::_204_us,
        InaVbusct::_332_us,
        InaVbusct::_588_us,
        InaVbusct::_1_1_ms,
        InaVbusct::_2_116_ms,
        InaVbusct::_4_156_ms,
        InaVbusct::_8_244_ms,
    ];

    pub fn label(self) -> &'static str {
        match self {
            InaVbusct::_140_us => "140 us",
            InaVbusct::_204_us => "204 us",
            InaVbusct::_332_us => "332 us",
            InaVbusct::_588_us => "588 us",
            InaVbusct::_1_1_ms => "1.1 ms",
            InaVbusct::_2_116_ms => "2.116 ms",
            InaVbusct::_4_156_ms => "4.156 ms",
            InaVbusct::_8_244_ms => "8.244 ms",
        }
    }
}

impl InaVshct {
    pub const ALL: [InaVshct; 8] = [
        InaVshct::_140_us,
        InaVshct::_204_us,
        InaVshct::_332_us,
        InaVshct::_588_us,
        InaVshct::_1_1_ms,
        InaVshct::_2_116_ms,
        InaVshct::_4_156_ms,
        InaVshct::_8_244_ms,
    ];

    pub fn label(self) -> &'static str {
        match self {
            InaVshct::_140_us => "140 us",
            InaVshct::_204_us => "204 us",
            InaVshct::_332_us => "332 us",
            InaVshct::_588_us => "588 us",
            InaVshct::_1_1_ms => "1.1 ms",
            InaVshct::_2_116_ms => "2.116 ms",
            InaVshct::_4_156_ms => "4.156 ms",
            InaVshct::_8_244_ms => "8.244 ms",
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(default)]
pub struct Chipsetup {
    pub mode: InaMode,
    pub vbusct: InaVbusct,
    pub vscht: InaVshct,
    pub average: InaAverage,
    pub maxcurrent: f64,
}

impl Chipsetup {
    pub fn new() -> Self {
        Chipsetup {
            maxcurrent: 5.0,
            ..Chipsetup::default()
        }
    }
}

/**
* Reads the die id and manufacturer idfrom the device
*
* @param dev The device handler to read from
* @param address The address of the device
*
* @return Error if unable to read from the device or the die id or manufacturer id is invalid.
*/
pub fn verify_hardware(dev: &mut mcp2221::Handle, address: u8) -> Result<(), Error> {
    let die = read_register_u16(dev, address, Register::DieId as u8)?;
    if DIE_ID != die {
        return Err(Error::InvalidDie);
    }
    let manufacturer = read_register_u16(dev, address, Register::Manufacturer as u8)?;
    if MANUFACTURER != manufacturer {
        return Err(Error::InvalidManufacturer);
    }
    Ok(())
}

/**
 * Configures the hardware for a given setup
 *
 * @param dev The device handler to configure
 * @param address The address of the device
 * @param setup The setup to use
 */
pub fn config_hardware(
    dev: &mut mcp2221::Handle,
    address: u8,
    setup: Chipsetup,
) -> Result<(), Error> {
    let mut configuration = read_configuration(dev, address)?;
    // println!("Bits          : 0bFEDCBA9876543210");
    // println!("Config        : 0b{:16b}", configuration.1);
    configuration &= 0xFFF8;
    configuration |= setup.mode as u16;
    // println!("mode          : 0b{:16b}", DEFAULT_MODE as u16);
    // println!("Config Mode   : 0b{:16b} 3 bits (2-1-0)", configuration.1);
    configuration &= 0xFFC7;
    configuration |= (setup.vscht as u16) << 3;
    // println!("Vscht         : 0b{:16b}", (DEFAULT_VSCHT as u16) << 3);
    // println!("Config Vscht  : 0b{:16b} 3 bits (6-5-4)", configuration.1);
    configuration &= 0xFE3F;
    configuration |= (setup.vbusct as u16) << 6;
    // println!("vbusct        : 0b{:16b}", (DEFAULT_VBUSCT as u16) << 6);
    // println!("Config Vbusct : 0b{:16b} 3 bits (9-8-7)", configuration.1);
    configuration &= 0xF1FF;
    configuration |= (setup.average as u16) << 9;
    // println!("average       : 0b{:16b}", (DEFAULT_AVERAGE as u16) << 9);
    // println!("Config Average: 0b{:16b} 3 bits (C-B-A)", configuration.1);
    write_register_u16(dev, address, Register::Configuration as u8, configuration)
}

/**
 * Read the bus voltage from the device.
 *
 * @param dev The device handler to read from.
 * @param address The address of the device.
 *
 * @return The voltage in volts.
 */
pub fn read_volts(dev: &mut mcp2221::Handle, address: u8) -> Result<f64, Error> {
    let raw = read_register_u16(dev, address, Register::BusVoltage as u8)?;
    Ok(bus_volts(raw))
}

pub fn bus_volts(raw: u16) -> f64 {
    raw as f64 * VOLTAGE_LSB
}

/**
 * Read the shunt voltage from the device.
 *
 * @param dev The device handler to read from.
 * @param address The address of the device.
 *
 * @return The shunt voltage in Volts.
 */
pub fn read_shunt(dev: &mut mcp2221::Handle, address: u8) -> Result<f64, Error> {
    let raw = read_register_u16(dev, address, Register::ShuntVoltage as u8)?;
    Ok(shunt_volts(raw as i16))
}

/// Shunt voltage is a signed 16 bit value (two's complement)
pub fn shunt_volts(raw: i16) -> f64 {
    raw as f64 * SHUNT_LSB
}

/**
 * Read the current from the device.
 *
 * @param dev The device handler to read from.
 * @param address The address of the device.
 *
 * @return The current in Amps.
 */
pub fn read_amps(dev: &mut mcp2221::Handle, address: u8, current_lsb: f64) -> Result<f64, Error> {
    let raw = read_register_u16(dev, address, Register::Current as u8)?;
    Ok(current_amps(raw as i16, current_lsb))
}

/// Current is a signed 16 bit value scaled by the LSB chosen during calibration
pub fn current_amps(raw: i16, current_lsb: f64) -> f64 {
    raw as f64 * current_lsb
}

/**
 * Read the power from the device.
 *
 * @param dev The device handler to read from.
 * @param address The address of the device.
 *
 * @return The power in Watts.
 */
pub fn read_power(dev: &mut mcp2221::Handle, address: u8, current_lsb: f64) -> Result<f64, Error> {
    let raw = read_register_u16(dev, address, Register::Power as u8)?;
    Ok(power_watts(raw, current_lsb))
}

pub fn power_watts(raw: u16, current_lsb: f64) -> f64 {
    raw as f64 * POWER_LSB_FACTOR * current_lsb
}

/**
 * Read the device configuration register.
 *
 * @param dev The device handler to read from.
 * @param address The address of the device.
 *
 * @return The device configuration register.
 */
pub fn read_configuration(dev: &mut mcp2221::Handle, address: u8) -> Result<u16, Error> {
    read_register_u16(dev, address, Register::Configuration as u8)
}

/**
 * Reset the device.
 *
 * @param dev The device handler to reset
 * @param address The address of the device
 */
pub fn reset(dev: &mut mcp2221::Handle, address: u8) -> Result<(), Error> {
    let mut configuration = read_configuration(dev, address)?;
    configuration |= 1 << 15;
    write_register_u16(dev, address, Register::Configuration as u8, configuration)
}

/**
 * Calibrate the device.
 *
 * @param dev The device handler to calibrate
 * @param address The address of the device
 * @param expect_max_curr The expected maximum current in Amps
 */
pub fn calibrate(
    dev: &mut mcp2221::Handle,
    address: u8,
    expect_max_curr: f64,
) -> Result<f64, Error> {
    let (cal, current_lsb) = calibration(expect_max_curr)?;
    write_register_u16(dev, address, Register::Calibration as u8, cal)?;
    Ok(current_lsb)
}

/// Computes the calibration register and the current LSB for a maximum expected current
pub fn calibration(expect_max_curr: f64) -> Result<(u16, f64), Error> {
    if !expect_max_curr.is_finite() || expect_max_curr <= 0.0 {
        return Err(Error::InvalidValue(
            "max current must be greater than zero".into(),
        ));
    }
    let current_lsb = expect_max_curr / (1 << 15) as f64;
    let cal = INTERNAL_SCALING / (current_lsb * SHUNT_RESISTOR);
    if cal > u16::MAX as f64 {
        return Err(Error::InvalidValue(format!(
            "max current {expect_max_curr} A is too low for the shunt resistor"
        )));
    }
    Ok((cal as u16, current_lsb))
}

impl Chipsetup {
    /// Reads the configuration register bits back into a setup (maxcurrent comes from the config)
    pub fn from_register(configuration: u16, maxcurrent: f64) -> Chipsetup {
        Chipsetup {
            mode: InaMode::from_bits(configuration).unwrap_or_default(),
            vbusct: InaVbusct::from_bits(configuration).unwrap_or_default(),
            vscht: InaVshct::from_bits(configuration).unwrap_or_default(),
            average: InaAverage::from_bits(configuration).unwrap_or_default(),
            maxcurrent,
        }
    }

    /// Decoded configuration register fields, as (name, value) pairs for display
    pub fn describe(&self, address: u8, configuration: u16) -> Vec<(String, String)> {
        vec![
            ("Address".into(), format!("{address:#04X}")),
            ("Configuration".into(), format!("{configuration:#06X}")),
            (
                "Mode".into(),
                format!("{} ({:#05b})", self.mode.name(), self.mode as u16),
            ),
            ("Vbus CT".into(), self.vbusct.label().into()),
            ("Vshunt CT".into(), self.vscht.label().into()),
            ("Averages".into(), self.average.label().into()),
            ("Max current".into(), format!("{:.3} A", self.maxcurrent)),
        ]
    }
}

impl InaMode {
    pub fn from_bits(configuration: u16) -> Option<InaMode> {
        InaMode::ALL
            .into_iter()
            .find(|mode| *mode as u16 == configuration & 0x7)
    }
}

impl InaVbusct {
    pub fn from_bits(configuration: u16) -> Option<InaVbusct> {
        InaVbusct::ALL
            .into_iter()
            .find(|v| *v as u16 == (configuration >> 6) & 0x7)
    }
}

impl InaVshct {
    pub fn from_bits(configuration: u16) -> Option<InaVshct> {
        InaVshct::ALL
            .into_iter()
            .find(|v| *v as u16 == (configuration >> 3) & 0x7)
    }
}

impl InaAverage {
    pub fn from_bits(configuration: u16) -> Option<InaAverage> {
        InaAverage::ALL
            .into_iter()
            .find(|v| *v as u16 == (configuration >> 9) & 0x7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaling() {
        assert!((bus_volts(0x2000) - 10.24).abs() < 1e-9);
        assert!((shunt_volts(-40) + 0.0001).abs() < 1e-12);
        assert!((current_amps(-1000, 0.000_152_587_890_625) + 0.152_587_890_625).abs() < 1e-12);
        assert!((power_watts(100, 0.001) - 2.5).abs() < 1e-12);
    }

    #[test]
    fn calibration_is_consistent_with_readings() {
        let (cal, lsb) = calibration(5.0).unwrap();
        assert_eq!(cal, 16777);
        assert!((lsb - 5.0 / 32768.0).abs() < 1e-15);
        // 1 A read back with that LSB
        let raw = (1.0 / lsb).round() as i16;
        assert!((current_amps(raw, lsb) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn calibration_rejects_invalid_max_current() {
        assert!(calibration(0.0).is_err());
        assert!(calibration(-1.0).is_err());
        assert!(calibration(f64::NAN).is_err());
        assert!(calibration(0.000001).is_err());
    }

    #[test]
    fn mode_names_round_trip() {
        for mode in InaMode::ALL {
            assert_eq!(mode.name().parse::<InaMode>().unwrap(), mode);
        }
        assert!("busvoltagecontinuous".parse::<InaMode>().is_ok());
        assert!("nope".parse::<InaMode>().is_err());
    }
}
