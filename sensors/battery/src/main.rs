use clap::Parser;
use waysensor_rs_core::{GlobalConfig, Sensor, IconStyle};
use std::io::{self, Write};
use std::time::Duration;
use tokio::time;

use waysensor_rs_battery::BatterySensor;

#[derive(Parser)]
#[command(name = "waysensor-rs-battery")]
#[command(about = "Battery sensor for waysensor-rs")]
#[command(version)]
struct Args {
    /// Battery name (e.g., BAT0, BAT1). If not specified, auto-detect first battery
    #[arg(short, long)]
    battery: Option<String>,

    /// Update interval in milliseconds
    #[arg(short, long, default_value = "5000")]
    interval: u64,

    /// Warning threshold (percentage)
    #[arg(short, long, default_value = "20")]
    warning: u8,

    /// Critical threshold (percentage)
    #[arg(short, long, default_value = "10")]
    critical: u8,

    /// One-shot mode (don't loop)
    #[arg(short, long)]
    once: bool,

    /// List available batteries and exit
    #[arg(short, long)]
    list: bool,

    /// Icon style (nerdfont, fontawesome, ascii, none)
    #[arg(long)]
    icon_style: Option<IconStyle>,

    /// Icon color (hex format like "#7aa2f7")
    #[arg(long)]
    icon_color: Option<String>,

    /// Text color (hex format like "#c0caf5")
    #[arg(long)]
    text_color: Option<String>,

    /// Tooltip label color (hex format like "#bb9af7")
    #[arg(long)]
    tooltip_label_color: Option<String>,

    /// Tooltip value color (hex format like "#9ece6a")
    #[arg(long)]
    tooltip_value_color: Option<String>,

    /// Check sensor availability and exit
    #[arg(long)]
    check: bool,

    /// Generate example config file and exit
    #[arg(long)]
    generate_config: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Handle config generation
    if args.generate_config {
        if let Some(config_path) = GlobalConfig::default_config_path() {
            GlobalConfig::save_example_config_to_file(&config_path)?;
            println!("Generated example config at: {}", config_path.display());
            println!("\nYou can now edit this file to customize your default colors and settings.");
        } else {
            eprintln!("Could not determine config directory");
            std::process::exit(1);
        }
        return Ok(());
    }
    
    // Handle list command
    if args.list {
        match BatterySensor::list_available_batteries() {
            Ok(batteries) => {
                if batteries.is_empty() {
                    println!("No batteries found");
                } else {
                    println!("Available batteries:");
                    for battery in batteries {
                        println!("  {}", battery);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error listing batteries: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    // Validate thresholds
    if args.warning <= args.critical {
        eprintln!("Warning threshold must be greater than critical threshold");
        std::process::exit(1);
    }

    if args.critical == 0 || args.warning >= 100 {
        eprintln!("Thresholds must be between 1-99%, with warning > critical");
        std::process::exit(1);
    }

    // Create battery sensor
    let mut battery_sensor = match BatterySensor::new(args.battery.clone(), args.warning, args.critical) {
        Ok(sensor) => sensor,
        Err(e) => {
            eprintln!("Error initializing battery sensor: {}", e);
            
            // If no specific battery was requested, show available options
            if args.battery.is_none() {
                if let Ok(batteries) = BatterySensor::list_available_batteries() {
                    if !batteries.is_empty() {
                        eprintln!("Available batteries:");
                        for battery in batteries {
                            eprintln!("  {}", battery);
                        }
                        eprintln!("Try specifying a battery with --battery <name>");
                    }
                }
            }
            std::process::exit(1);
        }
    };
    
    // Check availability if requested
    if args.check {
        match battery_sensor.check_availability() {
            Ok(()) => {
                println!("Battery sensor is available");
                return Ok(());
            }
            Err(e) => {
                eprintln!("Battery sensor is not available: {}", e);
                std::process::exit(1);
            }
        }
    }
    
    // Load global configuration and apply command line overrides
    let global_config = GlobalConfig::load().unwrap_or_default();
    let mut config = global_config.to_sensor_config()
        .with_update_interval(Duration::from_millis(args.interval))
        .apply_color_overrides(
            args.icon_color,
            args.text_color,
            args.tooltip_label_color,
            args.tooltip_value_color,
        );
    
    // Override icon style only if explicitly provided
    if let Some(icon_style) = args.icon_style {
        config = config.with_icon_style(icon_style);
    }
    
    battery_sensor.configure(config)?;
    
    if args.once {
        match battery_sensor.read() {
            Ok(output) => {
                println!("{}", serde_json::to_string(&output)?);
            }
            Err(e) => {
                eprintln!("Error reading battery stats: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let mut interval = time::interval(Duration::from_millis(args.interval));
        
        loop {
            interval.tick().await;
            
            match battery_sensor.read() {
                Ok(output) => {
                    println!("{}", serde_json::to_string(&output)?);
                    io::stdout().flush()?;
                }
                Err(e) => {
                    eprintln!("Error reading battery stats: {}", e);
                    // Don't exit on read errors, just continue trying
                }
            }
        }
    }
    
    Ok(())
}
