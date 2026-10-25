use clap::{CommandFactory, Parser, Subcommand};

mod bus;
use bus::Bus;
mod comm;

mod powmon;
use powmon::*;
mod config;
use config::*;

mod manifest;
mod tools;
use std::process::exit;
use tools::Error;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Device Status
    Status {
        /// Print a JSON table (used by the iocc TUI)
        #[arg(long)]
        json: bool,
    },
    /// Voltage read
    Voltage,
    /// Current read
    Amps,
    /// Power read
    Power,
    /// Read Shunt voltage
    Shunt,
    /// Keep printing readings until interrupted (used by the TUI live panel)
    Watch {
        /// Milliseconds between samples
        #[arg(long, default_value_t = 200)]
        interval_ms: u64,
        /// Print one JSON metrics object per line
        #[arg(long)]
        json: bool,
    },
    /// Read voltage, current, power and shunt voltage (configures and calibrates first)
    Read {
        /// Print JSON metrics (used by the iocc TUI)
        #[arg(long)]
        json: bool,
    },
    /// Set the INA226 operating mode
    Mode {
        /// PowerDown, ShuntVoltageTriggered, BusVoltageTriggered, ShuntAndBusTriggered,
        /// PowerDown2, ShuntVoltageContinuous, BusVoltageContinuous, ShuntAndBusContinuous
        mode: String,
        /// Keep the mode after the command exits (write it to config.yaml)
        #[arg(long)]
        save: bool,
    },
    /// Calibrate for a maximum expected current
    Calibrate {
        /// Maximum expected current in Amps (ex. 5.0)
        #[arg(long)]
        max: f64,
    },
    /// Reset chip
    Reset,
}

fn main() {
    // Answered before anything else: the TUI asks for it without opening the device
    if std::env::args().any(|a| a == manifest::MANIFEST_FLAG) {
        println!("{}", manifest::JSON);
        return;
    }

    // Plugins are not standalone, iocc sets IOCC_HOST when it starts them
    if std::env::var_os("IOCC_HOST").is_none() {
        eprintln!("iocc-powmon is a plugin and must be run through iocc: iocc powmon <command>");
        exit(1);
    }

    let cli = Cli::parse();
    let config = read_config();
    if config.is_verbose() {
        println!("[powmon] Configuration: \n{:?}", config);
    }

    match run(&cli, &config) {
        Ok(()) => {}
        Err(err) => {
            eprintln!("Error: {err}");
            exit(1);
        }
    }
}

fn open_device(config: &Powmon) -> Result<Bus, Error> {
    Bus::open(
        config.get_speed(),
        config.is_reset_on_open(),
        config.get_timeout(),
    )
}

/**
 * Prepares the chip (configuration + calibration) and returns the current LSB
 */
fn setup(bus: &mut Bus, config: &Powmon) -> Result<f64, Error> {
    let address = config.get_address();
    verify_hardware(bus, address)?;
    config_hardware(bus, address, config.get_chip_config())?;
    calibrate(bus, address, config.get_chip_config().maxcurrent)
}

fn run(cli: &Cli, config: &Powmon) -> Result<(), Error> {
    let address = config.get_address();
    let Some(command) = &cli.command else {
        Cli::command()
            .print_help()
            .map_err(|e| Error::InvalidValue(e.to_string()))?;
        println!();
        return Ok(());
    };

    // Validated before opening the device so a typo does not touch the hardware
    let new_mode = match command {
        Commands::Mode { mode, .. } => Some(mode.parse::<InaMode>()?),
        _ => None,
    };

    let mut dev = open_device(config)?;
    match command {
        Commands::Status { json } => {
            verify_hardware(&mut dev, address)?;
            let configuration = read_configuration(&mut dev, address)?;
            let setup =
                Chipsetup::from_register(configuration, config.get_chip_config().maxcurrent);
            if *json {
                println!("{}", manifest::status_table(&setup, address, configuration));
            } else {
                println!("Device Operational !");
                for (name, value) in setup.describe(address, configuration) {
                    println!("{name:>14}: {value}");
                }
                if config.is_verbose() {
                    println!("[powmon] Configuration: {configuration:#06X} 0b{configuration:16b}");
                }
            }
        }
        Commands::Voltage => println!("{} V", read_volts(&mut dev, address)?),
        Commands::Amps => {
            let current_lsb = setup(&mut dev, config)?;
            println!("{} A", read_amps(&mut dev, address, current_lsb)?);
        }
        Commands::Power => {
            let current_lsb = setup(&mut dev, config)?;
            println!("{} W", read_power(&mut dev, address, current_lsb)?);
        }
        Commands::Shunt => {
            setup(&mut dev, config)?;
            println!("{} V", read_shunt(&mut dev, address)?);
        }
        Commands::Read { json } => {
            let current_lsb = setup(&mut dev, config)?;
            let volts = read_volts(&mut dev, address)?;
            let amps = read_amps(&mut dev, address, current_lsb)?;
            let watts = read_power(&mut dev, address, current_lsb)?;
            let shunt = read_shunt(&mut dev, address)?;
            if *json {
                println!("{}", manifest::metrics(volts, amps, watts, shunt));
            } else {
                println!("Bus voltage : {volts:.3} V");
                println!("Current     : {amps:.3} A");
                println!("Power       : {watts:.3} W");
                println!("Shunt       : {shunt:.6} V");
            }
        }
        Commands::Watch { interval_ms, json } => {
            let current_lsb = setup(&mut dev, config)?;
            loop {
                let volts = read_volts(&mut dev, address)?;
                let amps = read_amps(&mut dev, address, current_lsb)?;
                let watts = read_power(&mut dev, address, current_lsb)?;
                let shunt = read_shunt(&mut dev, address)?;
                if *json {
                    println!("{}", manifest::metrics(volts, amps, watts, shunt));
                } else {
                    println!("{volts:.3} V  {amps:.3} A  {watts:.3} W  shunt {shunt:.6} V");
                }
                std::thread::sleep(std::time::Duration::from_millis((*interval_ms).max(10)));
            }
        }
        Commands::Mode { save, .. } => {
            let mode = new_mode.expect("mode parsed above");
            let mut chip = config.get_chip_config();
            chip.mode = mode;
            config_hardware(&mut dev, address, chip)?;
            println!("Mode: {}", mode.name());
            if *save {
                save_mode(&mode)?;
            } else {
                println!("(temporary; use --save to keep it after this command)");
            }
        }
        Commands::Calibrate { max } => {
            verify_hardware(&mut dev, address)?;
            let current_lsb = calibrate(&mut dev, address, *max)?;
            println!("Calibrated for {max} A (current LSB {current_lsb:.9} A)");
        }
        Commands::Reset => {
            reset(&mut dev, address)?;
            println!("Reset!");
        }
    }
    Ok(())
}

/**
 * Persists the mode in the shared config.yaml by editing only the
 * `powmon:` -> `chip:` -> `mode:` line, keeping comments and formatting
 */
fn save_mode(mode: &InaMode) -> Result<(), Error> {
    let Some(path) = std::env::var_os("IOCC_CONFIG") else {
        return Err(Error::InvalidValue(
            "IOCC_CONFIG not set, cannot save the mode".into(),
        ));
    };
    let path = std::path::PathBuf::from(path);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| Error::InvalidValue(format!("cannot read {}: {e}", path.display())))?;

    let indent_of = |line: &str| line.len() - line.trim_start().len();
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let mut in_powmon = false;
    let mut chip_indent = None;
    let mut updated = false;

    for line in &mut lines {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = indent_of(line);
        if indent == 0 {
            in_powmon = trimmed.starts_with("powmon:");
            chip_indent = None;
            continue;
        }
        if !in_powmon {
            continue;
        }
        match chip_indent {
            None => {
                if trimmed.starts_with("chip:") {
                    chip_indent = Some(indent);
                }
            }
            Some(chip) if indent <= chip => chip_indent = None,
            Some(_) if trimmed.starts_with("mode:") => {
                let prefix = &line[..indent];
                let comment = line
                    .find('#')
                    .map(|i| format!(" {}", &line[i..]))
                    .unwrap_or_default();
                *line = format!("{prefix}mode: {}{comment}", mode.name());
                updated = true;
                break;
            }
            Some(_) => {}
        }
    }

    if !updated {
        return Err(Error::InvalidValue(
            "no powmon.chip.mode entry in config.yaml, mode not saved".into(),
        ));
    }
    let mut content = lines.join("\n");
    if text.ends_with('\n') {
        content.push('\n');
    }
    std::fs::write(&path, content)
        .map_err(|e| Error::InvalidValue(format!("cannot write {}: {e}", path.display())))?;
    println!("Saved in {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_only_the_mode_line() {
        let dir = std::env::temp_dir().join(format!("iocc-powmon-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.yaml");
        std::fs::write(
            &path,
            "iocc:\n  verbose: true\npowmon:\n  chip:\n    # keep me\n    mode: BusVoltageContinuous # old\n    average: _16\nont:\n  port: a\n",
        )
        .unwrap();
        // SAFETY: only this test touches IOCC_CONFIG
        unsafe { std::env::set_var("IOCC_CONFIG", &path) };
        save_mode(&InaMode::ShuntAndBusContinuous).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("mode: ShuntAndBusContinuous # old"));
        assert!(
            text.contains("# keep me") && text.contains("ont:") && text.contains("verbose: true")
        );
        unsafe { std::env::remove_var("IOCC_CONFIG") };
        let _ = std::fs::remove_dir_all(&dir);
    }
}
