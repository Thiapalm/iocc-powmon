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
    let dev = match mcp2221::Handle::open_first(&config) {
        Ok(x) => x,
        Err(err) => {
            return Err(err);
        }
    };
    Ok(dev)
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
#[allow(dead_code)]
pub fn write_device(
    dev: &mut mcp2221::Handle,
    address: u8,
    register: u8,
    data: &mut Vec<u8>,
) -> Result<(), mcp2221::Error> {
    let mut my_data = vec![register];
    my_data.append(data);
    dev.write_read_address(address, &mut my_data, &mut [0u8])
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
#[allow(dead_code)]
pub fn read_device(
    dev: &mut mcp2221::Handle,
    address: u8,
    register: u8,
    expected: usize,
) -> Result<Vec<u8>, mcp2221::Error> {
    //println!("addr: {:#02x} reg {:#02x}", addr, register);
    let mut read = vec![0; expected];
    let mut reg: [u8; 1] = [0; 1];
    reg[0] = register;
    match dev.write_read_address(address, &mut reg, &mut read) {
        Ok(()) => Ok(read),
        Err(x) => Err(x),
    }
}

/**
 * Wrapper to read u16 from any given register
 * Inputs:
 *   dev - mcp2221 handle
 *   address - Device address in 7bit format
 *   register - Register that will receive the data
 *
 * Output:
 *   2Byte array containing the data
 */
pub fn read_register_u16(dev: &mut mcp2221::Handle, address: u8, reg: u8) -> [u8; 2] {
    read_device(dev, address, reg, 2)
        .unwrap()
        .try_into()
        .expect("Unable to convert")
    //let mut result: [u8; 2] = [0; 2];
    // let mut read: [u8; 1] = [0; 1];
    // read[0] = reg as u8;
    // let _ = dev.write_read_address(address, &mut read, &mut result);
    //result
}

/**
 * Wrapper to write u16 into any given register
 * Inputs:
 *   dev - mcp2221 handle
 *   address - Device address in 7bit format
 *   register - Register that will receive the data
 *   value - 16bit value to write
 *
 * Output:
 */
pub fn write_register_u16(dev: &mut mcp2221::Handle, address: u8, reg: u8, value: u16) {
    let mut result = vec![(value & 0x00FF) as u8, ((value & 0xFF00) >> 8) as u8];
    let _ = write_device(dev, address, reg, &mut result);

    // let mut result: [u8; 3] = [0; 3];
    // result[0] = reg as u8;
    // result[1] = (value & 0x00FF) as u8;
    // result[2] = ((value & 0xFF00) >> 8) as u8;
    // let _ = dev.write_read_address(address, &mut result, &mut [0u8]);
}
