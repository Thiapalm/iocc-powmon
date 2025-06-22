use crate::comm::*;
use crate::tools::*;
use byteorder::{BigEndian, ByteOrder};
use serde::Deserialize;

const INTERNAL_SCALING: f64 = 0.00512;
const SHUNT_LSB: f64 = 0.0000025; // in Volts (2.5 uV)
const VOLTAGE_LSB: f64 = 0.00125; // in Volts (1.25 mV)
const CURRENT_LSB: f64 = 0.001; // in Amps (1 mA)
const POWER_LSB: f64 = 0.025; // in Watts (25 mW)

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
#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InaAverage {
    _1 = 0x00,
    _4 = 0x01,
    _16 = 0x02,
    _64 = 0x03,
    _128 = 0x04,
    _256 = 0x05,
    _512 = 0x06,
    _1024 = 0x07,
}

/// Set the conversion time for the Bus voltage measurement
#[allow(non_camel_case_types, dead_code)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InaVbusct {
    _140_us = 0x00,
    _204_us = 0x01,
    _332_us = 0x02,
    _588_us = 0x03,
    _1_1_ms = 0x04,
    _2_116_ms = 0x05,
    _4_156_ms = 0x06,
    _8_244_ms = 0x07,
}

/// Set the conversion time for the Shunt Bus voltage measurement
#[allow(non_camel_case_types, dead_code)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InaVshct {
    _140_us = 0x00,
    _204_us = 0x01,
    _332_us = 0x02,
    _588_us = 0x03,
    _1_1_ms = 0x04,
    _2_116_ms = 0x05,
    _4_156_ms = 0x06,
    _8_244_ms = 0x07,
}

/// Used to set the ina226 mode of operation
#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize)]
pub enum InaMode {
    PowerDown = 0x00,
    ShuntVoltageTriggered = 0x01,
    BusVoltageTriggered = 0x02,
    ShuntAndBusTriggered = 0x03,
    PowerDown2 = 0x04,
    ShuntVoltageContinuous = 0x05,
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

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Chipsetup {
    pub mode: InaMode,
    pub vbusct: InaVbusct,
    pub vscht: InaVshct,
    pub average: InaAverage,
    pub maxcurrent: f64,
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
    let die = read_register_u16(dev, address, Register::DieId as u8);
    let manufact = read_register_u16(dev, address, Register::Manufacturer as u8);

    if DIE_ID != BigEndian::read_u16(&die) {
        return Err(Error::InvalidDie);
    }
    if MANUFACTURER != BigEndian::read_u16(&manufact) {
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
pub fn config_hardware(dev: &mut mcp2221::Handle, address: u8, setup: Chipsetup) {
    let mut configuration = read_configuration(dev, address);
    // println!("Bits          : 0bFEDCBA9876543210");
    // println!("Config        : 0b{:16b}", configuration.1);
    configuration.1 &= 0xFFF8;
    configuration.1 |= setup.mode as u16;
    // println!("mode          : 0b{:16b}", DEFAULT_MODE as u16);
    // println!("Config Mode   : 0b{:16b} 3 bits (2-1-0)", configuration.1);
    configuration.1 &= 0xFFC7;
    configuration.1 |= (setup.vscht as u16) << 3;
    // println!("Vscht         : 0b{:16b}", (DEFAULT_VSCHT as u16) << 3);
    // println!("Config Vscht  : 0b{:16b} 3 bits (6-5-4)", configuration.1);
    configuration.1 &= 0xFE3F;
    configuration.1 |= (setup.vbusct as u16) << 6;
    // println!("vbusct        : 0b{:16b}", (DEFAULT_VBUSCT as u16) << 6);
    // println!("Config Vbusct : 0b{:16b} 3 bits (9-8-7)", configuration.1);
    configuration.1 &= 0xF1FF;
    configuration.1 |= (setup.average as u16) << 9;
    // println!("average       : 0b{:16b}", (DEFAULT_AVERAGE as u16) << 9);
    // println!("Config Average: 0b{:16b} 3 bits (C-B-A)", configuration.1);
    write_register_u16(dev, address, Register::Configuration as u8, configuration.1);
}

/**
 * Read the bus voltage from the device.
 *
 * @param dev The device handler to read from.
 * @param address The address of the device.
 *
 * @return The voltage in volts.
 */
pub fn read_volts(dev: &mut mcp2221::Handle, address: u8) -> f64 {
    let result = read_register_u16(dev, address, Register::BusVoltage as u8);
    let result = BigEndian::read_u16(&result);
    result as f64 * VOLTAGE_LSB
}

/**
 * Read the shunt voltage from the device.
 *
 * @param dev The device handler to read from.
 * @param address The address of the device.
 *
 * @return The shunt voltage in Volts.
 */
pub fn read_shunt(dev: &mut mcp2221::Handle, address: u8) -> f64 {
    let result = read_register_u16(dev, address, Register::ShuntVoltage as u8);
    let result = BigEndian::read_u16(&result);
    result as f64 * SHUNT_LSB
}

/**
 * Read the current from the device.
 *
 * @param dev The device handler to read from.
 * @param address The address of the device.
 *
 * @return The current in Amps.
 */
pub fn read_amps(dev: &mut mcp2221::Handle, address: u8) -> f64 {
    let result = read_register_u16(dev, address, Register::Current as u8);
    let result = BigEndian::read_u16(&result);
    result as f64 * CURRENT_LSB
}

/**
 * Read the power from the device.
 *
 * @param dev The device handler to read from.
 * @param address The address of the device.
 *
 * @return The power in Watts.
 */
pub fn read_power(dev: &mut mcp2221::Handle, address: u8) -> f64 {
    let result = read_register_u16(dev, address, Register::Power as u8);
    let result = BigEndian::read_u16(&result);
    result as f64 * POWER_LSB
}

/**
 * Read the device configuration register.
 *
 * @param dev The device handler to read from.
 * @param address The address of the device.
 *
 * @return The device configuration register.
 */
pub fn read_configuration(dev: &mut mcp2221::Handle, address: u8) -> ([u8; 2], u16) {
    let configuration = read_register_u16(dev, address, Register::Configuration as u8);
    let u16configuration = BigEndian::read_u16(&configuration);
    (configuration, u16configuration)
}

/**
 * Reset the device.
 *
 * @param dev The device handler to reset
 * @param address The address of the device
 */
pub fn reset(dev: &mut mcp2221::Handle, address: u8) {
    let mut configuration = read_configuration(dev, address);
    configuration.1 |= 1 << 15;
    write_register_u16(dev, address, Register::Configuration as u8, configuration.1);
}

/**
 * Calibrate the device.
 *
 * @param dev The device handler to calibrate
 * @param address The address of the device
 * @param expect_max_curr The expected maximum current in Amps
 */
pub fn calibrate(dev: &mut mcp2221::Handle, address: u8, expect_max_curr: f64) {
    let cur_lsb = expect_max_curr / (1 << 15) as f64;
    let cal = (INTERNAL_SCALING / (cur_lsb * 0.002)) as u16;
    write_register_u16(dev, address, Register::Calibration as u8, cal);
}
