//! waysensor-rs-cpu: CPU usage monitoring binary for Waybar.
//!
//! This binary provides CPU usage monitoring for Waybar status bars.
//! It outputs JSON-formatted data compatible with Waybar's custom modules.

use clap::Parser;
use waysensor_rs_core::{GlobalConfig, IconStyle, Sensor, SensorConfig};
use waysensor_rs_cpu::CpuSensor;
use std::io::{self, Write};
use std::process;
use std::time::Duration;
use tokio::time;

/// Command-line arguments for the CPU sensor.
#[derive(Parser)]
#[command(name = "waysensor-rs-cpu")]
#[command(about = "CPU usage sensor for waysensor-rs")]
#[command(version)]
#[command(author)]
struct Args {
    /// Update interval in milliseconds (minimum 100ms)
    #[arg(short, long, default_value = "1000", value_parser = validate_interval)]
    interval: u64,

    /// Warning threshold percentage (0-100)
    #[arg(short, long, default_value = "70", value_parser = validate_percentage)]
    warning: u8,

    /// Critical threshold percentage (0-100, must be > warning)
    #[arg(short, long, default_value = "90", value_parser = validate_percentage)]
    critical: u8,

    /// One-shot mode (output once and exit)
    #[arg(short, long)]
    once: bool,

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

    /// Verify hardware/dependencies are available (validates /proc/stat access, permissions, etc.) and exit
    #[arg(long)]
    check: bool,

    /// Generate example config file and exit
    #[arg(long)]
    generate_config: bool,
}

/// Validate that the interval is at least 100ms.
fn validate_interval(s: &str) -> Result<u64, String> {
    let interval = s.parse::<u64>()
        .map_err(|_| "Interval must be a positive integer".to_owned())?;
    
    if interval < SensorConfig::MIN_UPDATE_INTERVAL {
        return Err(format!(
            "Interval must be at least {}ms", 
            SensorConfig::MIN_UPDATE_INTERVAL
        ));
    }
    
    Ok(interval)
}

/// Validate that the percentage is between 0 and 100.
fn validate_percentage(s: &str) -> Result<u8, String> {
    let percentage = s.parse::<u8>()
        .map_err(|_| "Percentage must be a number between 0-100".to_owned())?;
    
    if percentage > 100 {
        return Err("Percentage must be between 0-100".to_owned());
    }
    
    Ok(percentage)
}

/// Main entry point for the CPU sensor.
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
            process::exit(1);
        }
        return Ok(());
    }
    
    // Validate that critical > warning
    if args.critical <= args.warning {
        eprintln!("Error: Critical threshold ({}) must be greater than warning threshold ({})", 
                  args.critical, args.warning);
        process::exit(1);
    }
    
    // Create the CPU sensor
    let mut cpu_sensor = match CpuSensor::new(args.warning, args.critical) {
        Ok(sensor) => sensor,
        Err(e) => {
            eprintln!("Failed to create CPU sensor: {}", e);
            process::exit(1);
        }
    };
    
    // Check availability if requested
    if args.check {
        match cpu_sensor.check_availability() {
            Ok(()) => {
                println!("CPU sensor is available");
                return Ok(());
            }
            Err(e) => {
                eprintln!("CPU sensor is not available: {}", e);
                process::exit(1);
            }
        }
    }
    
    // Load global configuration and apply command line overrides
    let global_config = GlobalConfig::load().unwrap_or_default();
    let mut config = global_config.to_sensor_config()
        .with_update_interval(Duration::from_millis(args.interval))
        .apply_color_overrides(
            args.icon_color.clone(),
            args.text_color.clone(),
            args.tooltip_label_color.clone(),
            args.tooltip_value_color.clone(),
        );
    
    // Override icon style only if explicitly provided
    if let Some(icon_style) = args.icon_style {
        config = config.with_icon_style(icon_style);
    }
    
    cpu_sensor.configure(config)?;
    
    if args.once {
        // One-shot mode: read once and exit
        match cpu_sensor.read() {
            Ok(output) => {
                println!("{}", serde_json::to_string(&output)?);
            }
            Err(e) => {
                eprintln!("Error reading CPU stats: {}", e);
                process::exit(1);
            }
        }
    } else {
        // Continuous mode: loop and output readings
        let mut interval = time::interval(Duration::from_millis(args.interval));
        
        loop {
            interval.tick().await;
            
            // Regular sensor reading
            match cpu_sensor.read() {
                Ok(output) => {
                    println!("{}", serde_json::to_string(&output)?);
                    io::stdout().flush()?;
                }
                Err(e) => {
                    eprintln!("Error reading CPU stats: {}", e);
                    // Continue running on errors, just log them
                }
            }
        }
    }
    
    Ok(())
}