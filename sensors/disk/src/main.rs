//! # waysensor-rs-disk CLI
//!
//! Advanced disk monitoring utility with sophisticated multi-disk support,
//! performance tracking, and predictive analytics.
//!
//! ## Features
//!
//! - **Single and multi-disk monitoring** - Monitor individual or multiple disks
//! - **Multiple display modes** - Flexible display strategies for multi-disk setups
//! - **Performance monitoring** - Track usage trends and predict issues
//! - **Inode monitoring** - Monitor inode usage in addition to disk space
//! - **Caching** - Configurable caching for improved performance
//! - **Comprehensive error handling** - Detailed error reporting and recovery

use clap::Parser;
use waysensor_rs_core::{GlobalConfig, Sensor, IconStyle};
use waysensor_rs_disk::{
    DiskSensorBuilder, MultiDiskSensor, DisplayMode, CacheConfig
};
use std::{
    io::{self, Write},
    time::Duration,
    path::PathBuf,
};

#[derive(Parser)]
#[command(name = "waysensor-rs-disk")]
#[command(about = "Advanced disk usage monitoring for waybar with multi-disk support and performance analytics")]
#[command(version)]
#[command(long_about = "waysensor-rs-disk provides sophisticated disk monitoring with support for multiple disks, \
                       usage trend tracking, inode monitoring, and predictive analytics. It can operate in \
                       various display modes and provides comprehensive error handling.")]
struct Args {
    /// Primary disk path to monitor (default: /)
    #[arg(short, long, default_value = "/")]
    path: String,

    /// Additional paths for multi-disk monitoring
    #[arg(long, help = "Additional disk paths to monitor (enables multi-disk mode)")]
    paths: Vec<String>,

    /// Warning threshold percentage (0-100)
    #[arg(short, long, default_value = "80", value_parser = clap::value_parser!(u8).range(0..=100))]
    warning: u8,

    /// Critical threshold percentage (0-100)
    #[arg(short, long, default_value = "95", value_parser = clap::value_parser!(u8).range(0..=100))]
    critical: u8,

    /// Show available space instead of used space
    #[arg(short, long, help = "Display available space percentage instead of used space")]
    available: bool,

    /// Display mode for multi-disk monitoring
    #[arg(short, long, default_value = "highest", 
          help = "Display mode: highest, combined, cycle, average, total")]
    display_mode: String,

    /// Enable inode monitoring
    #[arg(long, help = "Monitor inode usage in addition to disk space")]
    monitor_inodes: bool,

    /// Enable performance monitoring and trend tracking
    #[arg(long, help = "Enable performance monitoring and usage trend tracking")]
    performance_monitoring: bool,

    /// Cache maximum age in milliseconds
    #[arg(long, default_value = "5000", help = "Maximum age of cached data in milliseconds")]
    cache_max_age: u64,

    /// Enable aggressive caching
    #[arg(long, help = "Enable aggressive caching for better performance")]
    aggressive_cache: bool,

    /// Trend history size (number of data points)
    #[arg(long, default_value = "24", help = "Number of historical data points for trend analysis")]
    trend_history_size: usize,

    /// Run once and exit (for testing)
    #[arg(long, help = "Run once and exit, useful for testing")]
    once: bool,

    /// Update interval in milliseconds
    #[arg(short, long, default_value = "5000", help = "Update interval in milliseconds")]
    interval: u64,

    /// Icon style: nerdfont, fontawesome, ascii, none
    #[arg(long, help = "Icon style for display")]
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

    /// Generate example config file and exit
    #[arg(long)]
    generate_config: bool,

    /// List available disk mount points and exit
    #[arg(long, help = "List available disk mount points and exit")]
    list_disks: bool,

    /// Show detailed disk information and exit
    #[arg(long, help = "Show detailed information about monitored disks and exit")]
    info: bool,

    /// Test configuration and exit
    #[arg(long, help = "Test configuration and exit with status code")]
    test: bool,

    /// Verbose output for debugging
    #[arg(short, long, help = "Enable verbose output for debugging")]
    verbose: bool,

    /// JSON output format (always enabled for waybar compatibility)
    #[arg(long, hide = true)]
    json: bool,
}

/// List available disk mount points.
fn list_available_disks() -> Result<(), Box<dyn std::error::Error>> {
    println!("Available disk mount points:");
    println!("=============================");
    
    // Read /proc/mounts to find mounted filesystems
    let mounts = std::fs::read_to_string("/proc/mounts")?;
    let mut mount_points = Vec::new();
    
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let device = parts[0];
            let mount_point = parts[1];
            let fs_type = parts[2];
            let options = parts[3];
            
            // Skip virtual filesystems and special mounts
            if !device.starts_with('/') || 
               fs_type == "proc" || fs_type == "sysfs" || fs_type == "devtmpfs" ||
               fs_type == "tmpfs" || fs_type == "devpts" || fs_type == "cgroup" ||
               mount_point.starts_with("/proc") || mount_point.starts_with("/sys") ||
               mount_point.starts_with("/dev") {
                continue;
            }
            
            mount_points.push((device, mount_point, fs_type, options.contains("ro")));
        }
    }
    
    // Sort by mount point
    mount_points.sort_by(|a, b| a.1.cmp(b.1));
    
    for (device, mount_point, fs_type, readonly) in mount_points {
        let ro_flag = if readonly { " (RO)" } else { "" };
        println!("  {} -> {} [{}]{}", device, mount_point, fs_type, ro_flag);
    }
    
    println!();
    println!("Example usage:");
    println!("  waysensor-rs-disk --path /");
    println!("  waysensor-rs-disk --path / --paths /home /var");
    println!("  waysensor-rs-disk --paths / /home --display-mode combined");
    
    Ok(())
}

/// Show detailed information about specified disks.
fn show_disk_info(paths: &[String], verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Disk Information");
    println!("================");
    
    for path_str in paths {
        let path = PathBuf::from(path_str);
        
        if !path.exists() {
            println!("❌ {}: Path does not exist", path_str);
            continue;
        }
        
        match DiskSensorBuilder::new(&path)
            .monitor_inodes(true)
            .performance_monitoring(true)
            .build() {
            Ok(mut sensor) => {
                match sensor.read() {
                    Ok(output) => {
                        println!("✅ {}: {}", path_str, output.text);
                        if verbose {
                            if let Some(tooltip) = output.tooltip {
                                println!("   Details: {}", tooltip.replace('\n', "\n   "));
                            }
                        }
                    },
                    Err(e) => {
                        println!("❌ {}: Error - {}", path_str, e);
                    }
                }
            },
            Err(e) => {
                println!("❌ {}: Configuration error - {}", path_str, e);
            }
        }
        
        println!();
    }
    
    Ok(())
}

/// Test configuration and sensor availability.
fn test_configuration(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Configuration");
    println!("=====================");
    
    // Test icon style 
    if let Some(icon_style) = args.icon_style {
        println!("✅ Icon style: {:?}", icon_style);
    } else {
        println!("✅ Icon style: default (from config)");
    }
    
    // Test display mode parsing
    let display_mode = parse_display_mode(&args.display_mode)?;
    println!("✅ Display mode: {:?}", display_mode);
    
    // Test threshold validation
    if args.warning >= args.critical {
        println!("❌ Warning threshold ({}) must be less than critical threshold ({})", 
                 args.warning, args.critical);
        return Err("Invalid threshold configuration".into());
    }
    println!("✅ Thresholds: warning {}%, critical {}%", args.warning, args.critical);
    
    // Test paths
    let all_paths = if args.paths.is_empty() {
        vec![args.path.clone()]
    } else {
        let mut paths = vec![args.path.clone()];
        paths.extend(args.paths.clone());
        paths
    };
    
    for path_str in &all_paths {
        let path = PathBuf::from(path_str);
        if path.exists() {
            println!("✅ Path exists: {}", path_str);
        } else {
            println!("❌ Path does not exist: {}", path_str);
            return Err(format!("Path does not exist: {}", path_str).into());
        }
    }
    
    // Test sensor creation
    let cache_config = CacheConfig {
        max_age: Duration::from_millis(args.cache_max_age),
        aggressive: args.aggressive_cache,
    };
    
    if all_paths.len() == 1 {
        match DiskSensorBuilder::new(&all_paths[0])
            .warning_threshold(args.warning)
            .critical_threshold(args.critical)
            .show_available(args.available)
            .monitor_inodes(args.monitor_inodes)
            .cache_config(cache_config)
            .performance_monitoring(args.performance_monitoring)
            .trend_history_size(args.trend_history_size)
            .build() {
            Ok(sensor) => {
                println!("✅ Single disk sensor created: {}", sensor.name());
                
                // Test availability
                match sensor.check_availability() {
                    Ok(_) => println!("✅ Sensor availability check passed"),
                    Err(e) => {
                        println!("❌ Sensor availability check failed: {}", e);
                        return Err(e.into());
                    }
                }
            },
            Err(e) => {
                println!("❌ Failed to create single disk sensor: {}", e);
                return Err(e.into());
            }
        }
    } else {
        let paths: Vec<String> = all_paths.iter().map(|s| s.to_string()).collect();
        match MultiDiskSensor::new(
            paths,
            args.warning,
            args.critical,
            args.available,
            display_mode,
        ) {
            Ok(sensor) => {
                println!("✅ Multi-disk sensor created: {}", sensor.name());
                
                // Test availability
                match sensor.check_availability() {
                    Ok(_) => println!("✅ Sensor availability check passed"),
                    Err(e) => {
                        println!("❌ Sensor availability check failed: {}", e);
                        return Err(e.into());
                    }
                }
            },
            Err(e) => {
                println!("❌ Failed to create multi-disk sensor: {}", e);
                return Err(e.into());
            }
        }
    }
    
    println!("\n✅ All configuration tests passed!");
    Ok(())
}

/// Parse display mode from string.
fn parse_display_mode(mode: &str) -> Result<DisplayMode, Box<dyn std::error::Error>> {
    match mode.to_lowercase().as_str() {
        "highest" | "max" => Ok(DisplayMode::HighestUsage),
        "combined" | "combine" => Ok(DisplayMode::Combined),
        "cycle" | "cycling" => Ok(DisplayMode::Cycle { current: 0 }),
        "specific" => Ok(DisplayMode::Specific(0)), // Default to first disk
        _ => Err(format!("Invalid display mode: '{}'. Valid options: highest, combined, cycle, specific", mode).into()),
    }
}

/// Create a sensor based on command line arguments.
fn create_sensor(args: &Args) -> Result<Box<dyn Sensor<Error = waysensor_rs_core::SensorError>>, Box<dyn std::error::Error>> {
    
    let cache_config = CacheConfig {
        max_age: Duration::from_millis(args.cache_max_age),
        aggressive: args.aggressive_cache,
    };
    
    let sensor: Box<dyn Sensor<Error = waysensor_rs_core::SensorError>> = if args.paths.is_empty() {
        // Single disk monitoring
        Box::new(DiskSensorBuilder::new(&args.path)
            .warning_threshold(args.warning)
            .critical_threshold(args.critical)
            .show_available(args.available)
            .monitor_inodes(args.monitor_inodes)
            .cache_config(cache_config)
            .performance_monitoring(args.performance_monitoring)
            .trend_history_size(args.trend_history_size)
            .build()?)
    } else {
        // Multi-disk monitoring
        let display_mode = parse_display_mode(&args.display_mode)?;
        
        let mut paths = vec![args.path.clone()];
        for path in &args.paths {
            paths.push(path.clone());
        }
        
        Box::new(MultiDiskSensor::new(
            paths,
            args.warning,
            args.critical,
            args.available,
            display_mode,
        )?)
    };
    
    Ok(sensor)
}

/// Main monitoring loop.
fn run_monitoring_loop(mut sensor: Box<dyn Sensor<Error = waysensor_rs_core::SensorError>>, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
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
    
    // Add custom configuration
    if args.cache_max_age != 5000 {
        config = config.with_custom("cache_max_age_ms", serde_json::Value::Number(serde_json::Number::from(args.cache_max_age)));
    }
    
    if args.aggressive_cache {
        config = config.with_custom("aggressive_cache", serde_json::Value::Bool(true));
    }
    
    sensor.configure(config)?;
    
    if args.verbose {
        eprintln!("✅ Sensor configured: {}", sensor.name());
        eprintln!("🔄 Starting monitoring loop (interval: {}ms)", args.interval);
    }
    
    if args.once {
        // Run once and output result
        let output = sensor.read()?;
        println!("{}", serde_json::to_string(&output)?);
        return Ok(());
    }
    
    // Continuous monitoring loop
    let mut error_count = 0;
    const MAX_CONSECUTIVE_ERRORS: usize = 5;
    
    loop {
        match sensor.read() {
            Ok(output) => {
                println!("{}", serde_json::to_string(&output)?);
                io::stdout().flush()?;
                error_count = 0; // Reset error count on success
            },
            Err(e) => {
                error_count += 1;
                
                if args.verbose {
                    eprintln!("❌ Error reading sensor (attempt {}): {}", error_count, e);
                }
                
                // Create error output for waybar
                let error_output = waysensor_rs_core::WaybarOutput::from_str("Disk Error")
                    .with_tooltip(format!("Error: {}", e))
                    .with_class("error");
                
                println!("{}", serde_json::to_string(&error_output)?);
                io::stdout().flush()?;
                
                // Exit if too many consecutive errors
                if error_count >= MAX_CONSECUTIVE_ERRORS {
                    eprintln!("❌ Too many consecutive errors ({}), exiting", error_count);
                    return Err(format!("Too many consecutive errors: {}", e).into());
                }
            }
        }
        
        std::thread::sleep(Duration::from_millis(args.interval));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    if args.verbose {
        eprintln!("🚀 waysensor-rs-disk starting...");
    }
    
    // Handle special commands first
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
    
    if args.list_disks {
        return list_available_disks();
    }
    
    if args.info {
        let all_paths = if args.paths.is_empty() {
            vec![args.path.clone()]
        } else {
            let mut paths = vec![args.path.clone()];
            paths.extend(args.paths.clone());
            paths
        };
        return show_disk_info(&all_paths, args.verbose);
    }
    
    if args.test {
        return test_configuration(&args);
    }
    
    // Validate thresholds
    if args.warning >= args.critical {
        return Err(format!(
            "Warning threshold ({}) must be less than critical threshold ({})",
            args.warning, args.critical
        ).into());
    }
    
    // Create and run sensor
    let sensor = create_sensor(&args)?;
    run_monitoring_loop(sensor, &args)
}