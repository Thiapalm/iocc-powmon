use crate::comm::{read_device, start_device, write_device};
use crate::tools::Error;

/**
 * I2C access used by the plugin. When iocc sets IOCC_SOCKET (a running TUI, or
 * a command-line run that found one) all transactions go through the broker,
 * which owns the MCP2221. That allows monitoring to keep running while another
 * plugin uses the same bus. Without IOCC_SOCKET the plugin opens the device
 * itself, as before.
 */
pub enum Bus {
    Local(mcp2221::Handle),
    #[cfg(unix)]
    Remote(Client),
}

impl Bus {
    pub fn open(speed: u32, reset: bool, timeout: u64) -> Result<Bus, Error> {
        #[cfg(unix)]
        if let Some(path) = std::env::var_os("IOCC_SOCKET").filter(|p| !p.is_empty()) {
            return Client::connect(std::path::PathBuf::from(path)).map(Bus::Remote);
        }
        let _ = (speed, reset, timeout);
        start_device(speed, reset, timeout)
            .map(Bus::Local)
            .map_err(|e| Error::InvalidValue(format!("{e}, check USB connection")))
    }

    pub fn read(&mut self, address: u8, register: u8, count: usize) -> Result<Vec<u8>, Error> {
        match self {
            Bus::Local(dev) => read_device(dev, address, register, count),
            #[cfg(unix)]
            Bus::Remote(client) => client
                .request(serde_json::json!({
                    "op": "read",
                    "address": address,
                    "register": register,
                    "count": count,
                }))
                .map(|response| {
                    response["data"]
                        .as_array()
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(|v| v.as_u64().map(|v| v as u8))
                                .collect()
                        })
                        .unwrap_or_default()
                }),
        }
    }

    pub fn write(&mut self, address: u8, register: u8, data: &[u8]) -> Result<(), Error> {
        match self {
            Bus::Local(dev) => write_device(dev, address, register, data),
            #[cfg(unix)]
            Bus::Remote(client) => client
                .request(serde_json::json!({
                    "op": "write",
                    "address": address,
                    "register": register,
                    "data": data,
                }))
                .map(|_| ()),
        }
    }

    pub fn read_u16(&mut self, address: u8, register: u8) -> Result<u16, Error> {
        let data = self.read(address, register, 2)?;
        Ok(u16::from_be_bytes([data[0], data[1]]))
    }

    pub fn write_u16(&mut self, address: u8, register: u8, value: u16) -> Result<(), Error> {
        self.write(address, register, &value.to_be_bytes())
    }
}

#[cfg(unix)]
pub struct Client {
    writer: std::os::unix::net::UnixStream,
    reader: std::io::BufReader<std::os::unix::net::UnixStream>,
    next_id: u64,
}

#[cfg(unix)]
impl Client {
    fn connect(path: std::path::PathBuf) -> Result<Client, Error> {
        use std::os::unix::net::UnixStream;
        let writer = UnixStream::connect(&path).map_err(|e| {
            Error::InvalidValue(format!(
                "cannot connect to the iocc TUI broker at {}: {e}",
                path.display()
            ))
        })?;
        let reader = writer.try_clone().map_err(|e| {
            Error::InvalidValue(format!("cannot use the iocc TUI broker connection: {e}"))
        })?;
        Ok(Client {
            writer,
            reader: std::io::BufReader::new(reader),
            next_id: 1,
        })
    }

    fn request(&mut self, mut request: serde_json::Value) -> Result<serde_json::Value, Error> {
        use std::io::{BufRead, Write};
        let id = self.next_id;
        self.next_id += 1;
        request["id"] = id.into();
        writeln!(self.writer, "{request}")
            .map_err(|e| Error::InvalidValue(format!("broker write failed: {e}")))?;
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|e| Error::InvalidValue(format!("broker read failed: {e}")))?;
        if line.trim().is_empty() {
            return Err(Error::CommunicationErr);
        }
        let response: serde_json::Value = serde_json::from_str(line.trim())
            .map_err(|e| Error::InvalidValue(format!("invalid broker response: {e}")))?;
        if response["ok"] != true {
            return Err(Error::InvalidValue(
                response["error"]
                    .as_str()
                    .unwrap_or("broker reported an error")
                    .to_string(),
            ));
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_bus_is_used_without_socket() {
        // SAFETY: no other test reads IOCC_SOCKET
        unsafe { std::env::remove_var("IOCC_SOCKET") };
        // Opening only fails if there is no USB device, never with a panic
        let _ = Bus::open(400_000, false, 10);
    }

    #[cfg(unix)]
    #[test]
    fn missing_socket_reports_the_path() {
        // SAFETY: no other test reads IOCC_SOCKET
        unsafe { std::env::set_var("IOCC_SOCKET", "/tmp/iocc-not-running.sock") };
        let err = Bus::open(400_000, false, 10).err().unwrap().to_string();
        assert!(err.contains("iocc-not-running.sock"), "{err}");
        unsafe { std::env::remove_var("IOCC_SOCKET") };
    }
}
