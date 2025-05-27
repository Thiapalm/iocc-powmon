use clap::{Parser, Subcommand};

mod comm;
use comm::*;

mod powmon;
use powmon::*;
mod config;
use config::*;

mod tools;
use std::process::exit;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Device Status
    Status,
    /// Voltage read
    Voltage,
    /// Current read
    Amps,
    /// Power read
    Power,
    /// Read Shunt voltage
    Shunt,
    /// Reset chip
    Reset,
}

fn main() {
    let cli = Cli::parse();
    let config = read_config();
    if config.is_verbose() {
        println!("[powmon] Configuration: \n{:?}", config);
    }

    let mut dev = match start_device(
        config.get_speed(),
        config.is_reset_on_open(),
        config.get_timeout(),
    ) {
        Ok(x) => x,
        Err(err) => {
            println!("Error: {err}, check USB connection");
            exit(1);
        }
    };

    match &cli.command {
        Some(Commands::Status) => match verify_hardware(&mut dev, config.get_address()) {
            Ok(()) => {
                println!("Device Operational !");
                let (x, y) = read_configuration(&mut dev, config.get_address());
                if config.is_verbose() {
                    println!("[powmon] Configuration: {:?} 0b{:16b}", x, y);
                }
            }
            Err(x) => {
                println!("Error: {}", x);
            }
        },
        Some(Commands::Voltage) => {
            config_hardware(&mut dev, config.get_address(), config.get_chip_config());
            calibrate(
                &mut dev,
                config.get_address(),
                config.get_chip_config().maxcurrent.clone(),
            );
            let result = read_volts(&mut dev, config.get_address());
            println!("{} V", result);
        }
        Some(Commands::Amps) => {
            config_hardware(&mut dev, config.get_address(), config.get_chip_config());
            calibrate(
                &mut dev,
                config.get_address(),
                config.get_chip_config().maxcurrent.clone(),
            );
            let result = read_amps(&mut dev, config.get_address());
            println!("{} A", result);
        }
        Some(Commands::Power) => {
            config_hardware(&mut dev, config.get_address(), config.get_chip_config());
            calibrate(
                &mut dev,
                config.get_address(),
                config.get_chip_config().maxcurrent.clone(),
            );
            let result = read_power(&mut dev, config.get_address());
            println!("{} W", result);
        }
        Some(Commands::Shunt) => {
            config_hardware(&mut dev, config.get_address(), config.get_chip_config());
            calibrate(
                &mut dev,
                config.get_address(),
                config.get_chip_config().maxcurrent.clone(),
            );
            let result = read_shunt(&mut dev, config.get_address());
            println!("{} V", result);
        }
        Some(Commands::Reset) => {
            reset(&mut dev, config.get_address());
            println!("Reset!");
        }
        None => {}
    }
}
