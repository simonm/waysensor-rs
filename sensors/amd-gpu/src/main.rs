use clap::Parser;
use waysensor_rs_core::{GlobalConfig, Sensor, IconStyle};
use std::io::{self, Write};
use std::time::Duration;
use tokio::time;

use waysensor_rs_amd_gpu::AmdgpuSensor;

#[derive(Parser)]
#[command(name = "waysensor-rs-amd-gpu")]
#[command(about = "AMD GPU metrics sensor for waysensor-rs")]
#[command(version)]
struct Args {
    /// Path to gpu_metrics file (auto-detect if not specified)
    #[arg(short, long)]
    file: Option<String>,

    /// Update interval in milliseconds
    #[arg(short, long, default_value = "1000")]
    interval: u64,

    /// Temperature warning threshold (Celsius)
    #[arg(long, default_value = "80")]
    temp_warning: u16,

    /// Temperature critical threshold (Celsius)
    #[arg(long, default_value = "90")]
    temp_critical: u16,

    /// Output format: compact, detailed, minimal, power, activity
    #[arg(long, default_value = "compact")]
    format: String,

    /// One-shot mode (don't loop)
    #[arg(short, long)]
    once: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

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
    
    if args.verbose {
        eprintln!("Starting waysensor-rs-amd-gpu...");
    }
    
    let mut amdgpu_sensor = AmdgpuSensor::new(
        args.file,
        args.temp_warning,
        args.temp_critical,
        args.format,
        args.verbose,
    )?;
    
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
    
    // Check availability if requested
    if args.check {
        match amdgpu_sensor.check_availability() {
            Ok(()) => {
                println!("AMD GPU sensor is available");
                return Ok(());
            }
            Err(e) => {
                eprintln!("AMD GPU sensor is not available: {}", e);
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
    
    // Load sensor-specific configuration from global config
    if let Some(amd_gpu_config) = global_config.sensors.get("amd-gpu") {
        if let serde_json::Value::Object(map) = amd_gpu_config {
            for (key, value) in map {
                config = config.with_custom(key.clone(), value.clone());
            }
        }
    }
    
    amdgpu_sensor.configure(config)?;
    
    if args.once {
        let output = amdgpu_sensor.read()?;
        println!("{}", serde_json::to_string(&output)?);
    } else {
        let mut interval = time::interval(Duration::from_millis(args.interval));
        
        loop {
            interval.tick().await;
            
            match amdgpu_sensor.read() {
                Ok(output) => {
                    println!("{}", serde_json::to_string(&output)?);
                    io::stdout().flush()?;
                }
                Err(e) => {
                    if args.verbose {
                        eprintln!("Error reading GPU metrics: {}", e);
                    }
                    // Output error state in waybar format
                    let error_output = waysensor_rs_core::WaybarOutput {
                        text: "GPU Error".to_string(),
                        tooltip: Some(format!("Error: {}", e)),
                        class: Some("error".to_string()),
                        percentage: None,
                    };
                    println!("{}", serde_json::to_string(&error_output)?);
                    io::stdout().flush()?;
                }
            }
        }
    }
    
    Ok(())
}