use clap::Parser;
use waysensor_rs_core::{GlobalConfig, Sensor, IconStyle};
use std::io::{self, Write};
use std::time::Duration;
use tokio::time;

use waysensor_rs_thermal::ThermalSensor;

#[derive(Parser)]
#[command(name = "waysensor-rs-thermal")]
#[command(about = "Thermal sensor for waysensor-rs")]
#[command(version)]
struct Args {
    /// Thermal zone to monitor (auto-detect if not specified)
    #[arg(short = 'z', long)]
    zone: Option<String>,

    /// Update interval in milliseconds
    #[arg(short = 't', long, default_value = "2000")]
    interval: u64,

    /// Warning threshold (°C)
    #[arg(short, long, default_value = "75")]
    warning: f64,

    /// Critical threshold (°C)
    #[arg(short, long, default_value = "90")]
    critical: f64,

    /// One-shot mode (don't loop)
    #[arg(short, long)]
    once: bool,
    
    /// List available thermal zones
    #[arg(long)]
    list_zones: bool,

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
    
    // Handle list zones mode
    if args.list_zones {
        println!("🌡️  Available Thermal Sensors");
        println!("=============================\n");
        
        let mut found_any = false;
        
        // Check thermal_zone interface
        let thermal_dir = "/sys/class/thermal";
        if let Ok(entries) = std::fs::read_dir(thermal_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("thermal_zone") {
                        let type_path = format!("{}/{}/type", thermal_dir, name);
                        let temp_path = format!("{}/{}/temp", thermal_dir, name);
                        
                        if let (Ok(zone_type), Ok(temp)) = (
                            std::fs::read_to_string(&type_path),
                            std::fs::read_to_string(&temp_path)
                        ) {
                            let zone_type = zone_type.trim();
                            let temp_millidegrees: i32 = temp.trim().parse().unwrap_or(0);
                            let temp_celsius = temp_millidegrees as f64 / 1000.0;
                            
                            println!("{:<30} {:<25} {:.1}°C", name, zone_type, temp_celsius);
                            found_any = true;
                        }
                    }
                }
            }
        }
        
        // Check hwmon interface
        if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let hwmon_path = entry.path();
                if let Ok(hwmon_entries) = std::fs::read_dir(&hwmon_path) {
                    for hwmon_entry in hwmon_entries.flatten() {
                        let file_name = hwmon_entry.file_name();
                        if let Some(name) = file_name.to_str() {
                            if name.starts_with("temp") && name.ends_with("_input") {
                                let temp_path = hwmon_entry.path();
                                
                                // Try to read temperature and label
                                if let Ok(temp_str) = std::fs::read_to_string(&temp_path) {
                                    if let Ok(temp_millidegrees) = temp_str.trim().parse::<i32>() {
                                        let temp_celsius = temp_millidegrees as f64 / 1000.0;
                                        
                                        // Try to get a label
                                        let label_path = temp_path.with_file_name(
                                            name.replace("_input", "_label")
                                        );
                                        let label = if let Ok(label_str) = std::fs::read_to_string(&label_path) {
                                            label_str.trim().to_string()
                                        } else {
                                            // Try to get hwmon device name
                                            let name_path = hwmon_path.join("name");
                                            if let Ok(device_name) = std::fs::read_to_string(&name_path) {
                                                format!("{} {}", device_name.trim(), name.replace("_input", ""))
                                            } else {
                                                format!("hwmon {}", name.replace("_input", ""))
                                            }
                                        };
                                        
                                        let display_path = temp_path.to_string_lossy();
                                        println!("{:<30} {:<25} {:.1}°C", 
                                            display_path.chars().rev().take(30).collect::<String>().chars().rev().collect::<String>(),
                                            label, 
                                            temp_celsius
                                        );
                                        found_any = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if !found_any {
            println!("No thermal sensors found.");
        }
        
        return Ok(());
    }
    
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
    
    let mut thermal_sensor = ThermalSensor::new(
        args.zone,
        args.warning,
        args.critical,
    )?;
    
    // Check availability if requested
    if args.check {
        match thermal_sensor.check_availability() {
            Ok(()) => {
                println!("Thermal sensor is available");
                return Ok(());
            }
            Err(e) => {
                eprintln!("Thermal sensor is not available: {}", e);
                std::process::exit(1);
            }
        }
    }
    
    // Load global configuration and apply command line overrides
    let global_config = match GlobalConfig::load() {
        Ok(config) => {
            eprintln!("DEBUG: Loaded config with icon_style: {:?}", config.icon_style);
            config
        }
        Err(e) => {
            eprintln!("DEBUG: Failed to load config: {}, using default", e);
            GlobalConfig::default()
        }
    };
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
    
    thermal_sensor.configure(config)?;
    
    if args.once {
        let output = thermal_sensor.read()?;
        println!("{}", serde_json::to_string(&output)?);
    } else {
        let mut interval = time::interval(Duration::from_millis(args.interval));
        
        loop {
            interval.tick().await;
            
            match thermal_sensor.read() {
                Ok(output) => {
                    println!("{}", serde_json::to_string(&output)?);
                    io::stdout().flush()?;
                }
                Err(e) => {
                    eprintln!("Error reading thermal sensor: {}", e);
                }
            }
        }
    }
    
    Ok(())
}