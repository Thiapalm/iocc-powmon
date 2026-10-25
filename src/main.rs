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
        /// Do not write the mode to config.yaml (temporary change)
        #[arg(long)]
        no_save: bool,
    },
    /// Calibrate for a maximum expected current
    Calibrate {
        /// Maximum expected current in Amps (ex. 5.0)
        #[arg(long)]
        max: f64,
        /// Do not write maxcurrent to config.yaml (temporary change)
        #[arg(long)]
        no_save: bool,
    },
    /// Reset chip
    Reset,
}

fn main() {
    // Answered before anything else: the TUI asks for it without opening the device.
    // The saved settings become the action defaults, so the TUI reopens with them.
    if std::env::args().any(|a| a == manifest::MANIFEST_FLAG) {
        let chip = read_config_quiet().get_chip_config();
        println!("{}", manifest::json(&chip));
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
        Commands::Mode { no_save, .. } => {
            let mode = new_mode.expect("mode parsed above");
            let mut chip = config.get_chip_config();
            chip.mode = mode;
            config_hardware(&mut dev, address, chip)?;
            println!("Mode: {}", mode.name());
            if *no_save {
                println!("(temporary; without --no-save it is kept in config.yaml)");
            } else {
                save("mode", mode.name())?;
            }
        }
        Commands::Calibrate { max, no_save } => {
            verify_hardware(&mut dev, address)?;
            let current_lsb = calibrate(&mut dev, address, *max)?;
            println!("Calibrated for {max} A (current LSB {current_lsb:.9} A)");
            if *no_save {
                println!("(temporary; without --no-save it is kept in config.yaml)");
            } else {
                save("maxcurrent", &max.to_string())?;
            }
        }
        Commands::Reset => {
            reset(&mut dev, address)?;
            println!("Reset!");
        }
    }
    Ok(())
}

/**
 * Persists a chip setting in the shared config.yaml and reports where it went.
 * Called after the hardware accepted the change, so the file matches the chip
 * when the command ends.
 */
fn save(key: &str, value: &str) -> Result<(), Error> {
    let path = save_chip_entry(key, value).map_err(Error::InvalidValue)?;
    println!("Saved in {}", path.display());
    Ok(())
}
