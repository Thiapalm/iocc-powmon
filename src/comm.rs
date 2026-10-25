use crate::tools::Error;
use embedded_hal::blocking::i2c::Write as _;
use std::time::Duration;

/**
 * Initializes the I2C connection towards a given device.
 * Inputs:
 *   speed - Bus speed in Hz (ex. 400000 Hz)
 *   reset - Reset the chip when starting communication or not
 *   timeout - communication timeout in milliseconds (ex. 10ms)
 *
 * Output:
 *   Error code if there was a problem od a mcp2221 Handle
 */
pub fn start_device(
    speed: u32,
    reset: bool,
    timeout: u64,
) -> Result<mcp2221::Handle, mcp2221::Error> {
    let mut config = mcp2221::Config::default();
    config.i2c_speed_hz = speed;
    config.reset_on_open = reset;
    // For talking to a peripheral we might want a higher timeout, but for
    // scanning the bus, a short timeout is good since it allows us to scan all
    // addresses more quickly.
    config.timeout = Duration::from_millis(timeout);
    mcp2221::Handle::open_first(&config)
}

/**
 * Write data to a given device.
 * Inputs:
 *   dev - mcp2221 handle
 *   address - Device address in 7bit format
 *   register - Register that will receive the data
 *   data - Data to write in u8 format
 *
 * Output:
 *   Error code if there was a problem while writing
 */
pub fn write_device(
    dev: &mut mcp2221::Handle,
    address: u8,
    register: u8,
    data: &[u8],
) -> Result<(), Error> {
    let mut payload = Vec::with_capacity(data.len() + 1);
    payload.push(register);
    payload.extend_from_slice(data);
    dev.write(address, &payload)?;
    Ok(())
}

/**
 * Read data from a given device.
 * Inputs:
 *   dev - mcp2221 handle
 *   address - Device address in 7bit format
 *   register - Register that will receive the data
 *   expected - number of bytes to read
 *
 * Output:
 *   Error code if there was a problem while writing or a vector of u8 containing the data
 */
pub fn read_device(
    dev: &mut mcp2221::Handle,
    address: u8,
    register: u8,
    expected: usize,
) -> Result<Vec<u8>, Error> {
    let mut read = vec![0; expected];
    dev.write_read_address(address, &[register], &mut read)?;
    Ok(read)
}
