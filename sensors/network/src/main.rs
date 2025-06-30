use clap::Parser;
use waysensor_rs_core::{GlobalConfig, Sensor, IconStyle};
use std::io::{self, Write};
use std::time::Duration;
use tokio::time;

use waysensor_rs_network::NetworkSensor;

#[derive(Parser)]
#[command(name = "waysensor-rs-network")]
#[command(about = "Network bandwidth sensor for waysensor-rs")]
#[command(version)]
struct Args {
    /// Interface to monitor (auto-detect if not specified)
    #[arg(short, long)]
    interface: Option<String>,

    /// Update interval in milliseconds
    #[arg(short = 't', long, default_value = "1000")]
    interval: u64,

    /// Warning threshold (MB/s)
    #[arg(short, long, default_value = "50")]
    warning: u64,

    /// Critical threshold (MB/s)
    #[arg(short, long, default_value = "100")]
    critical: u64,

    /// Show total (up+down) instead of separate values
    #[arg(long)]
    total: bool,

    /// Show upload speed only
    #[arg(long)]
    upload_only: bool,

    /// Show download speed only
    #[arg(long)]
    download_only: bool,

    /// One-shot mode (don't loop)
    #[arg(short, long)]
    once: bool,
    
    /// Detect and list active network interfaces
    #[arg(long)]
    detect: bool,

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
    
    // Handle detection mode
    if args.detect {
        use waysensor_rs_network::auto_detect::{detect_active_interfaces, find_best_interface};
        
        println!("🌐 Network Interface Detection");
        println!("==============================\n");
        
        let interfaces = detect_active_interfaces()?;
        
        println!("{:<15} {:<10} {:<6} {:<6} {:<10} {:<10} {:<10}", 
                 "Interface", "Type", "Up", "IP", "RX Packets", "TX Packets", "Score");
        println!("{}", "-".repeat(75));
        
        for iface in &interfaces {
            println!("{:<15} {:<10} {:<6} {:<6} {:<10} {:<10} {:<10.1}", 
                     iface.name,
                     format!("{:?}", iface.interface_type),
                     if iface.is_up { "✓" } else { "✗" },
                     if iface.has_ip { "✓" } else { "✗" },
                     iface.rx_packets,
                     iface.tx_packets,
                     iface.activity_score);
        }
        
        println!("\n🎯 Best interface: {}", find_best_interface()?);
        
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
    
    let mut network_sensor = NetworkSensor::new(
        args.interface,
        args.warning,
        args.critical,
        args.total,
        args.upload_only,
        args.download_only,
    )?;
    
    // Check availability if requested
    if args.check {
        match network_sensor.check_availability() {
            Ok(()) => {
                println!("Network sensor is available");
                return Ok(());
            }
            Err(e) => {
                eprintln!("Network sensor is not available: {}", e);
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
    
    network_sensor.configure(config)?;
    
    if args.once {
        // For one-shot mode, we need to wait a bit to calculate bandwidth
        tokio::time::sleep(Duration::from_millis(1000)).await;
        let output = network_sensor.read()?;
        println!("{}", serde_json::to_string(&output)?);
    } else {
        let mut interval = time::interval(Duration::from_millis(args.interval));
        
        loop {
            interval.tick().await;
            
            match network_sensor.read() {
                Ok(output) => {
                    println!("{}", serde_json::to_string(&output)?);
                    io::stdout().flush()?;
                }
                Err(e) => {
                    eprintln!("Error reading network stats: {}", e);
                }
            }
        }
    }
    
    Ok(())
}