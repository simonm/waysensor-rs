use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "waysensor-rs-discover")]
#[command(about = "Hardware discovery tool for waysensor sensors")]
#[command(version)]
struct Args {
    /// Output format: json, ron, waybar-config
    #[arg(short, long, default_value = "json")]
    format: String,

    /// Generate waybar configuration
    #[arg(long)]
    waybar_config: bool,

    /// Smart detection with capability testing
    #[arg(long)]
    smart: bool,

    /// Interactive setup wizard
    #[arg(long)]
    setup: bool,

    /// Generate complete waybar config with styling
    #[arg(long)]
    complete_config: bool,

    /// Test sensor performance and find optimal intervals
    #[arg(long)]
    benchmark: bool,

    /// Output directory for generated files
    #[arg(short, long, default_value = ".")]
    output: String,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HardwareInfo {
    cpu: CpuInfo,
    memory: MemoryInfo,
    disks: Vec<DiskInfo>,
    gpus: Vec<GpuInfo>,
    thermal: Vec<ThermalZone>,
    network: Vec<NetworkInterface>,
    battery: Vec<BatteryInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CpuInfo {
    model: String,
    cores: u32,
    threads: u32,
    max_frequency: Option<u64>,
    available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryInfo {
    total_ram: u64,
    total_swap: u64,
    available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiskInfo {
    path: String,
    filesystem: String,
    total: u64,
    device: String,
    available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GpuInfo {
    vendor: String,
    model: String,
    driver: String,
    metrics_path: Option<String>,
    available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThermalZone {
    name: String,
    r#type: String,
    path: String,
    current_temp: Option<f64>,
    available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkInterface {
    name: String,
    r#type: String,
    speed: Option<u64>,
    available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatteryInfo {
    name: String,
    path: String,
    capacity: Option<u64>,
    status: Option<String>,
    available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WaybarConfig {
    modules: HashMap<String, serde_json::Value>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("🔍 waysensor-rs Hardware Discovery & Configuration");
    println!("=============================================");
    
    // Handle special modes first
    if args.setup {
        return run_setup_wizard(&args);
    }
    
    if args.benchmark {
        return run_benchmark(&args);
    }
    
    let hardware = if args.smart {
        discover_hardware_smart(args.verbose)?
    } else {
        discover_hardware(args.verbose)?
    };
    
    if args.complete_config {
        return generate_complete_waybar_setup(&hardware, &args);
    }
    
    match args.format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&hardware)?);
        }
        "ron" => {
            println!("{}", ron::ser::to_string_pretty(&hardware, ron::ser::PrettyConfig::default())?);
        }
        "waybar-config" => {
            let config = generate_waybar_config(&hardware)?;
            println!("{}", serde_json::to_string_pretty(&config)?);
        }
        _ => {
            eprintln!("Unsupported format: {}", args.format);
            std::process::exit(1);
        }
    }
    
    if args.waybar_config {
        println!("\n📋 Suggested waybar configuration:");
        println!("   1. Copy the JSON above to your waybar config");
        println!("   2. Add the module names to your waybar 'modules-left/center/right'");
        println!("   3. Customize intervals and styling as needed");
        println!("\n💡 Tip: Use --complete-config for a full waybar setup with styling!");
    }
    
    Ok(())
}

fn discover_hardware(verbose: bool) -> Result<HardwareInfo, Box<dyn std::error::Error>> {
    if verbose {
        println!("🔍 Scanning CPU...");
    }
    let cpu = discover_cpu()?;
    
    if verbose {
        println!("🔍 Scanning Memory...");
    }
    let memory = discover_memory()?;
    
    if verbose {
        println!("🔍 Scanning Disks...");
    }
    let disks = discover_disks()?;
    
    if verbose {
        println!("🔍 Scanning GPUs...");
    }
    let gpus = discover_gpus()?;
    
    if verbose {
        println!("🔍 Scanning Thermal Zones...");
    }
    let thermal = discover_thermal_zones()?;
    
    if verbose {
        println!("🔍 Scanning Network Interfaces...");
    }
    let network = discover_network_interfaces()?;
    
    if verbose {
        println!("🔍 Scanning Batteries...");
    }
    let battery = discover_batteries()?;
    
    Ok(HardwareInfo {
        cpu,
        memory,
        disks,
        gpus,
        thermal,
        network,
        battery,
    })
}

fn discover_cpu() -> Result<CpuInfo, Box<dyn std::error::Error>> {
    let content = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    
    let mut model = "Unknown CPU".to_string();
    let mut cores = 0u32;
    let mut max_frequency = None;
    
    for line in content.lines() {
        if line.starts_with("model name") {
            if let Some(name) = line.split(':').nth(1) {
                model = name.trim().to_string();
            }
        } else if line.starts_with("processor") {
            cores += 1;
        } else if line.starts_with("cpu MHz") {
            if let Some(freq_str) = line.split(':').nth(1) {
                if let Ok(freq) = freq_str.trim().parse::<f64>() {
                    max_frequency = Some((freq * 1_000_000.0) as u64); // Convert to Hz
                }
            }
        }
    }
    
    // Get thread count from /proc/stat if available
    let threads = cores; // Simplified for now
    
    Ok(CpuInfo {
        model,
        cores,
        threads,
        max_frequency,
        available: Path::new("/proc/stat").exists(),
    })
}

fn discover_memory() -> Result<MemoryInfo, Box<dyn std::error::Error>> {
    let content = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    
    let mut total_ram = 0;
    let mut total_swap = 0;
    
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        
        let key = parts[0].trim_end_matches(':');
        if let Ok(value) = parts[1].parse::<u64>() {
            let value_bytes = value * 1024; // Convert from kB
            
            match key {
                "MemTotal" => total_ram = value_bytes,
                "SwapTotal" => total_swap = value_bytes,
                _ => {}
            }
        }
    }
    
    Ok(MemoryInfo {
        total_ram,
        total_swap,
        available: Path::new("/proc/meminfo").exists(),
    })
}

fn discover_disks() -> Result<Vec<DiskInfo>, Box<dyn std::error::Error>> {
    let mut disks = Vec::new();
    
    // Common mount points to check
    let mount_points = ["/", "/home", "/boot", "/var", "/tmp"];
    
    for &mount_point in &mount_points {
        if let Ok(metadata) = fs::metadata(mount_point) {
            if metadata.is_dir() {
                // Use statvfs-like functionality (simplified)
                if let Ok(output) = std::process::Command::new("df")
                    .arg("-T")
                    .arg(mount_point)
                    .output()
                {
                    if let Ok(stdout) = String::from_utf8(output.stdout) {
                        for line in stdout.lines().skip(1) {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 7 {
                                let device = parts[0].to_string();
                                let filesystem = parts[1].to_string();
                                let total = parts[2].parse::<u64>().unwrap_or(0) * 1024; // Convert from KB
                                
                                disks.push(DiskInfo {
                                    path: mount_point.to_string(),
                                    filesystem,
                                    total,
                                    device,
                                    available: true,
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(disks)
}

fn discover_gpus() -> Result<Vec<GpuInfo>, Box<dyn std::error::Error>> {
    let mut gpus = Vec::new();
    
    // Check for AMD GPUs
    if let Ok(entries) = fs::read_dir("/sys/class/drm") {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("card") && !name.contains("-") {
                        let device_path = path.join("device");
                        let gpu_metrics_path = device_path.join("gpu_metrics");
                        
                        if gpu_metrics_path.exists() {
                            // Try to read vendor information
                            let vendor_path = device_path.join("vendor");
                            let device_id_path = device_path.join("device");
                            
                            let vendor = fs::read_to_string(&vendor_path)
                                .unwrap_or_default()
                                .trim()
                                .to_string();
                            
                            let device_id = fs::read_to_string(&device_id_path)
                                .unwrap_or_default()
                                .trim()
                                .to_string();
                            
                            let vendor_name = match vendor.as_str() {
                                "0x1002" => "AMD",
                                "0x10de" => "NVIDIA",
                                "0x8086" => "Intel",
                                _ => "Unknown",
                            };
                            
                            gpus.push(GpuInfo {
                                vendor: vendor_name.to_string(),
                                model: format!("GPU {} ({})", name, device_id),
                                driver: "amdgpu".to_string(), // Detected from gpu_metrics presence
                                metrics_path: Some(gpu_metrics_path.to_string_lossy().to_string()),
                                available: true,
                            });
                        }
                    }
                }
            }
        }
    }
    
    Ok(gpus)
}

fn discover_thermal_zones() -> Result<Vec<ThermalZone>, Box<dyn std::error::Error>> {
    let mut zones = Vec::new();
    
    if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("thermal_zone") {
                        let type_path = path.join("type");
                        let temp_path = path.join("temp");
                        
                        let zone_type = fs::read_to_string(&type_path)
                            .unwrap_or_default()
                            .trim()
                            .to_string();
                        
                        let current_temp = fs::read_to_string(&temp_path)
                            .ok()
                            .and_then(|s| s.trim().parse::<i32>().ok())
                            .map(|t| t as f64 / 1000.0); // Convert from millidegrees
                        
                        zones.push(ThermalZone {
                            name: name.to_string(),
                            r#type: zone_type,
                            path: path.to_string_lossy().to_string(),
                            current_temp,
                            available: temp_path.exists(),
                        });
                    }
                }
            }
        }
    }
    
    Ok(zones)
}

fn discover_network_interfaces() -> Result<Vec<NetworkInterface>, Box<dyn std::error::Error>> {
    let mut interfaces = Vec::new();
    
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name != "lo" { // Skip loopback
                        let type_path = path.join("type");
                        let speed_path = path.join("speed");
                        
                        let interface_type = fs::read_to_string(&type_path)
                            .unwrap_or_default()
                            .trim()
                            .to_string();
                        
                        let speed = fs::read_to_string(&speed_path)
                            .ok()
                            .and_then(|s| s.trim().parse::<u64>().ok());
                        
                        let type_name = match interface_type.as_str() {
                            "1" => "ethernet",
                            "24" => "ethernet", // Also ethernet
                            "803" => "wireless",
                            _ => "unknown",
                        };
                        
                        interfaces.push(NetworkInterface {
                            name: name.to_string(),
                            r#type: type_name.to_string(),
                            speed,
                            available: path.join("statistics").exists(),
                        });
                    }
                }
            }
        }
    }
    
    Ok(interfaces)
}

fn discover_batteries() -> Result<Vec<BatteryInfo>, Box<dyn std::error::Error>> {
    let mut batteries = Vec::new();
    
    if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let type_path = path.join("type");
                    
                    if let Ok(supply_type) = fs::read_to_string(&type_path) {
                        if supply_type.trim() == "Battery" {
                            let capacity_path = path.join("capacity");
                            let status_path = path.join("status");
                            
                            let capacity = fs::read_to_string(&capacity_path)
                                .ok()
                                .and_then(|s| s.trim().parse::<u64>().ok());
                            
                            let status = fs::read_to_string(&status_path)
                                .ok()
                                .map(|s| s.trim().to_string());
                            
                            batteries.push(BatteryInfo {
                                name: name.to_string(),
                                path: path.to_string_lossy().to_string(),
                                capacity,
                                status,
                                available: capacity_path.exists(),
                            });
                        }
                    }
                }
            }
        }
    }
    
    Ok(batteries)
}

fn generate_waybar_config(hardware: &HardwareInfo) -> Result<WaybarConfig, Box<dyn std::error::Error>> {
    let mut modules = HashMap::new();
    
    // CPU module
    if hardware.cpu.available {
        modules.insert("custom/waysensor-rs-cpu".to_string(), serde_json::json!({
            "exec": "waysensor-rs-cpu --once",
            "return-type": "json",
            "interval": 1,
            "tooltip": true
        }));
    }
    
    // Memory module
    if hardware.memory.available {
        modules.insert("custom/waysensor-rs-memory".to_string(), serde_json::json!({
            "exec": "waysensor-rs-memory --once",
            "return-type": "json",
            "interval": 2,
            "tooltip": true
        }));
    }
    
    // GPU modules
    for (i, gpu) in hardware.gpus.iter().enumerate() {
        if gpu.available {
            let module_name = if i == 0 {
                "custom/waysensor-rs-gpu".to_string()
            } else {
                format!("custom/waysensor-rs-gpu-{}", i)
            };
            
            let mut exec_args = vec!["waysensor-rs-amd-gpu", "--once"];
            if let Some(path) = &gpu.metrics_path {
                exec_args.push("--file");
                exec_args.push(path);
            }
            
            modules.insert(module_name, serde_json::json!({
                "exec": exec_args.join(" "),
                "return-type": "json",
                "interval": 2,
                "tooltip": true
            }));
        }
    }
    
    // Disk modules
    for disk in &hardware.disks {
        if disk.available && disk.path != "/boot" && disk.path != "/tmp" {
            let module_name = if disk.path == "/" {
                "custom/waysensor-rs-disk".to_string()
            } else {
                format!("custom/waysensor-rs-disk-{}", disk.path.replace('/', "-"))
            };
            
            modules.insert(module_name, serde_json::json!({
                "exec": format!("waysensor-rs-disk --once --path {}", disk.path),
                "return-type": "json",
                "interval": 30,
                "tooltip": true
            }));
        }
    }
    
    // Battery modules
    for (i, battery) in hardware.battery.iter().enumerate() {
        if battery.available {
            let module_name = if i == 0 {
                "custom/waysensor-rs-battery".to_string()
            } else {
                format!("custom/waysensor-rs-battery-{}", i)
            };
            
            modules.insert(module_name, serde_json::json!({
                "exec": format!("waysensor-rs-battery --once --battery {}", battery.name),
                "return-type": "json",
                "interval": 10,
                "tooltip": true
            }));
        }
    }
    
    Ok(WaybarConfig { modules })
}

// Enhanced discovery with capability testing
fn discover_hardware_smart(verbose: bool) -> Result<HardwareInfo, Box<dyn std::error::Error>> {
    if verbose {
        println!("🧠 Running smart detection with capability testing...");
    }
    
    let mut hardware = discover_hardware(verbose)?;
    
    // Test each sensor to verify it actually works
    if verbose {
        println!("🧪 Testing sensor capabilities...");
    }
    
    // Test CPU sensor
    if let Ok(output) = std::process::Command::new("waysensor-rs-cpu").arg("--once").output() {
        if output.status.success() {
            if verbose {
                println!("  ✅ CPU sensor: Working");
            }
        } else {
            if verbose {
                println!("  ❌ CPU sensor: Failed");
            }
            hardware.cpu.available = false;
        }
    }
    
    // Test memory sensor
    if let Ok(output) = std::process::Command::new("waysensor-rs-memory").arg("--once").output() {
        if output.status.success() {
            if verbose {
                println!("  ✅ Memory sensor: Working");
            }
        } else {
            if verbose {
                println!("  ❌ Memory sensor: Failed");
            }
            hardware.memory.available = false;
        }
    }
    
    // Test GPU sensors
    for gpu in &mut hardware.gpus {
        if let Some(path) = &gpu.metrics_path {
            if let Ok(output) = std::process::Command::new("waysensor-rs-amd-gpu")
                .arg("--once")
                .arg("--file")
                .arg(path)
                .output() {
                if output.status.success() {
                    if verbose {
                        println!("  ✅ GPU sensor ({}): Working", gpu.model);
                    }
                } else {
                    if verbose {
                        println!("  ❌ GPU sensor ({}): Failed", gpu.model);
                    }
                    gpu.available = false;
                }
            }
        }
    }
    
    if verbose {
        println!("🎯 Smart detection complete!");
    }
    
    Ok(hardware)
}

// Check which binaries are required and whether they are installed
fn check_required_binaries(hardware: &HardwareInfo) -> Vec<(String, bool)> {
    let mut binaries = Vec::new();
    
    // Function to check if a binary exists
    let check_binary = |name: &str| -> bool {
        // Check in ~/.local/bin
        let home_local = std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".local/bin").join(name))
            .ok()
            .and_then(|p| if p.exists() { Some(()) } else { None })
            .is_some();
        
        // Check in PATH
        let in_path = std::process::Command::new("which")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        home_local || in_path
    };
    
    // Check each required binary based on hardware
    if hardware.cpu.available {
        binaries.push(("waysensor-rs-cpu".to_string(), check_binary("waysensor-rs-cpu")));
    }
    
    if hardware.memory.available {
        binaries.push(("waysensor-rs-memory".to_string(), check_binary("waysensor-rs-memory")));
    }
    
    for gpu in &hardware.gpus {
        if gpu.available {
            binaries.push(("waysensor-rs-amd-gpu".to_string(), check_binary("waysensor-rs-amd-gpu")));
            break;
        }
    }
    
    for disk in &hardware.disks {
        if disk.available && (disk.path == "/" || disk.path == "/home") {
            binaries.push(("waysensor-rs-disk".to_string(), check_binary("waysensor-rs-disk")));
            break;
        }
    }
    
    for iface in &hardware.network {
        if iface.available {
            binaries.push(("waysensor-rs-network".to_string(), check_binary("waysensor-rs-network")));
            break;
        }
    }
    
    for battery in &hardware.battery {
        if battery.available {
            binaries.push(("waysensor-rs-battery".to_string(), check_binary("waysensor-rs-battery")));
            break;
        }
    }
    
    // Always check for discover
    binaries.push(("waysensor-rs-discover".to_string(), check_binary("waysensor-rs-discover")));
    
    binaries
}

// Interactive setup wizard
fn run_setup_wizard(_args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧙 waysensor-rs Setup Wizard");
    println!("========================");
    println!();
    println!("Welcome! This wizard will help you set up waysensor-rs sensors for your system.");
    println!("We'll detect your hardware and create a complete waybar configuration.");
    println!();
    
    // Hardware detection
    println!("🔍 Step 1: Hardware Detection");
    println!("------------------------------");
    let hardware = discover_hardware_smart(true)?;
    
    println!();
    println!("📊 Step 2: Sensor Selection");
    println!("----------------------------");
    println!("Found the following sensors:");
    
    let mut selected_sensors = Vec::new();
    
    if hardware.cpu.available {
        println!("  • CPU Monitor ({} cores)", hardware.cpu.cores);
        selected_sensors.push("cpu");
    }
    
    if hardware.memory.available {
        println!("  • Memory Monitor ({} total)", format::bytes_to_human(hardware.memory.total_ram));
        selected_sensors.push("memory");
    }
    
    for (i, gpu) in hardware.gpus.iter().enumerate() {
        if gpu.available {
            println!("  • GPU Monitor {} ({})", i + 1, gpu.model);
            selected_sensors.push("gpu");
        }
    }
    
    for disk in &hardware.disks {
        if disk.available && (disk.path == "/" || disk.path == "/home") {
            println!("  • Disk Monitor {} ({})", disk.path, format::bytes_to_human(disk.total));
            selected_sensors.push("disk");
        }
    }
    
    for battery in &hardware.battery {
        if battery.available {
            println!("  • Battery Monitor ({})", battery.name);
            selected_sensors.push("battery");
        }
    }
    
    println!();
    println!("🎨 Step 3: Configuration");
    println!("-------------------------");
    println!("Selected {} sensors for monitoring.", selected_sensors.len());
    println!("Generating optimal waybar configuration...");
    
    // Generate complete configuration
    let config = generate_complete_waybar_config(&hardware)?;
    
    println!();
    println!("✅ Setup Complete!");
    println!("==================");
    println!("Generated files:");
    println!("  • waybar-config.json - Waybar module configuration");
    println!("  • waybar-style.css - Recommended styling");
    println!("  • generated-install.sh - Auto-generated installation script");
    println!();
    
    // Check if binaries are already installed
    let binaries_needed = check_required_binaries(&hardware);
    let all_installed = binaries_needed.iter().all(|(_, installed)| *installed);
    
    if all_installed {
        println!("✅ All required binaries are already installed!");
        println!();
        println!("To use:");
        println!("  1. Copy modules from waybar-config.json to your waybar config");
        println!("  2. Add CSS from waybar-style.css to your waybar CSS file");
        println!("  3. Restart waybar");
        println!();
        println!("💡 No need to run the install script - binaries are already available!");
    } else {
        println!("⚠️  Missing binaries detected:");
        for (binary, installed) in &binaries_needed {
            if !installed {
                println!("  ❌ {}", binary);
            }
        }
        println!();
        println!("To use:");
        println!("  1. Run: ./generated-install.sh (to install missing binaries)");
        println!("  2. Add modules from waybar-config.json to your waybar config");
        println!("  3. Add CSS from waybar-style.css to your waybar CSS file");
        println!("  4. Restart waybar");
    }
    
    // Write files
    std::fs::write("waybar-config.json", serde_json::to_string_pretty(&config)?)?;
    std::fs::write("waybar-style.css", generate_css_styling())?;
    std::fs::write("generated-install.sh", generate_install_script(&hardware)?)?;
    
    // Make install script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata("generated-install.sh")?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions("generated-install.sh", perms)?;
    }
    
    Ok(())
}

// Performance benchmarking
fn run_benchmark(_args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("🏃 waysensor-rs Performance Benchmark");
    println!("=================================");
    println!("Testing sensor performance to find optimal intervals...");
    println!();
    
    let sensors = [
        ("CPU", "waysensor-rs-cpu"),
        ("Memory", "waysensor-rs-memory"),
        ("AMD GPU", "waysensor-rs-amd-gpu"),
        ("Disk", "waysensor-rs-disk"),
    ];
    
    for (name, binary) in &sensors {
        print!("Testing {} sensor... ", name);
        
        let _start = std::time::Instant::now();
        let mut total_time = std::time::Duration::new(0, 0);
        let mut successful_runs = 0;
        
        // Run 10 tests
        for _ in 0..10 {
            let run_start = std::time::Instant::now();
            if let Ok(output) = std::process::Command::new(binary).arg("--once").output() {
                if output.status.success() {
                    total_time += run_start.elapsed();
                    successful_runs += 1;
                }
            }
        }
        
        if successful_runs > 0 {
            let avg_time = total_time / successful_runs;
            let recommended_interval = (avg_time.as_millis() * 10).max(100); // 10x avg time, min 100ms
            println!("✅ Avg: {:.1}ms, Recommended interval: {}ms", 
                avg_time.as_millis(), recommended_interval);
        } else {
            println!("❌ Not available");
        }
    }
    
    println!();
    println!("💡 Recommendations:");
    println!("  • CPU: 1000ms (responsive)");
    println!("  • Memory: 2000ms (balanced)");
    println!("  • GPU: 1500ms (smooth)");
    println!("  • Disk: 5000ms (efficiency)");
    println!("  • Network: 1000ms (real-time)");
    println!("  • Battery: 10000ms (power saving)");
    
    Ok(())
}

// Generate complete waybar setup
fn generate_complete_waybar_setup(hardware: &HardwareInfo, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Generating Complete Waybar Setup");
    println!("====================================");
    
    let config = generate_complete_waybar_config(hardware)?;
    let css = generate_css_styling();
    let install_script = generate_install_script(hardware)?;
    
    // Write to output directory
    let output_dir = std::path::Path::new(&args.output);
    std::fs::create_dir_all(output_dir)?;
    
    let config_path = output_dir.join("waysensor-rs-waybar-config.json");
    let css_path = output_dir.join("waysensor-rs-style.css");
    let install_path = output_dir.join("install-waysensor-rs.sh");
    
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    std::fs::write(&css_path, css)?;
    std::fs::write(&install_path, install_script)?;
    
    // Make install script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&install_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&install_path, perms)?;
    }
    
    println!("✅ Generated files in '{}':", args.output);
    println!("  📄 {} - Waybar module configuration", config_path.display());
    println!("  🎨 {} - CSS styling", css_path.display());
    println!("  🚀 {} - Installation script", install_path.display());
    println!();
    println!("🔧 To install:");
    println!("  cd {}", args.output);
    println!("  ./install-waysensor-rs.sh");
    println!();
    println!("📋 Add to your waybar config:");
    println!("  \"modules-right\": [\"custom/waysensor-rs-cpu\", \"custom/waysensor-rs-memory\", ...]");
    
    Ok(())
}

fn generate_complete_waybar_config(hardware: &HardwareInfo) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut config = serde_json::Map::new();
    let mut modules = Vec::<String>::new();
    
    // Add available sensors with optimized intervals
    if hardware.cpu.available {
        modules.push("custom/waysensor-rs-cpu".to_string());
        config.insert("custom/waysensor-rs-cpu".to_string(), serde_json::json!({
            "exec": "waysensor-rs-cpu --once",
            "return-type": "json",
            "interval": 1,
            "tooltip": true,
            "format": "{icon} {text}",
            "format-icons": ["🖥️"]
        }));
    }
    
    if hardware.memory.available {
        modules.push("custom/waysensor-rs-memory".to_string());
        config.insert("custom/waysensor-rs-memory".to_string(), serde_json::json!({
            "exec": "waysensor-rs-memory --once",
            "return-type": "json",
            "interval": 2,
            "tooltip": true,
            "format": "{icon} {text}",
            "format-icons": ["🧠"]
        }));
    }
    
    // Add GPU modules
    for (i, gpu) in hardware.gpus.iter().enumerate() {
        if gpu.available {
            let module_name = if i == 0 {
                "custom/waysensor-rs-gpu".to_string()
            } else {
                format!("custom/waysensor-rs-gpu-{}", i)
            };
            
            modules.push(module_name.clone());
            
            let mut exec_args = vec!["waysensor-rs-amd-gpu", "--once"];
            if let Some(path) = &gpu.metrics_path {
                exec_args.push("--file");
                exec_args.push(path);
            }
            
            config.insert(module_name, serde_json::json!({
                "exec": exec_args.join(" "),
                "return-type": "json",
                "interval": 2,
                "tooltip": true,
                "format": "{icon} {text}",
                "format-icons": ["🎮"]
            }));
        }
    }
    
    // Add disk modules for important mounts
    for disk in &hardware.disks {
        if disk.available && (disk.path == "/" || disk.path == "/home") {
            let module_name = if disk.path == "/" {
                "custom/waysensor-rs-disk".to_string()
            } else {
                format!("custom/waysensor-rs-disk-{}", disk.path.replace('/', "-"))
            };
            
            modules.push(module_name.clone());
            
            config.insert(module_name, serde_json::json!({
                "exec": format!("waysensor-rs-disk --once --path {}", disk.path),
                "return-type": "json",
                "interval": 30,
                "tooltip": true,
                "format": "{icon} {text}",
                "format-icons": ["💾"]
            }));
        }
    }
    
    // Add battery modules
    for (i, battery) in hardware.battery.iter().enumerate() {
        if battery.available {
            let module_name = if i == 0 {
                "custom/waysensor-rs-battery".to_string()
            } else {
                format!("custom/waysensor-rs-battery-{}", i)
            };
            
            modules.push(module_name.clone());
            
            config.insert(module_name, serde_json::json!({
                "exec": format!("waysensor-rs-battery --once --battery {}", battery.name),
                "return-type": "json",
                "interval": 10,
                "tooltip": true,
                "format": "{text}",
            }));
        }
    }
    
    // Create complete waybar config structure
    let complete_config = serde_json::json!({
        "modules": config,
        "suggested_modules_right": modules,
        "layer": "top",
        "position": "top",
        "height": 30,
        "spacing": 4
    });
    
    Ok(complete_config)
}

fn generate_css_styling() -> String {
    r#"/* waysensor-rs CSS Styling for Waybar */

/* Base styling for all waysensor-rs modules */
[id^="custom/waysensor-rs"] {
    background-color: transparent;
    color: @text;
    border-radius: 6px;
    padding: 0 8px;
    margin: 0 2px;
    transition: all 0.3s ease;
}

/* CPU Sensor */
#custom-waysensor-rs-cpu {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
}

#custom-waysensor-rs-cpu.warning {
    background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}

#custom-waysensor-rs-cpu.critical {
    background: linear-gradient(135deg, #ff6b6b 0%, #ee5a24 100%);
    animation: pulse 2s ease-in-out infinite alternate;
}

/* Memory Sensor */
#custom-waysensor-rs-memory {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
}

#custom-waysensor-rs-memory.warning {
    background: linear-gradient(135deg, #ffecd2 0%, #fcb69f 100%);
    color: #333;
}

#custom-waysensor-rs-memory.critical {
    background: linear-gradient(135deg, #ff6b6b 0%, #ee5a24 100%);
    color: white;
}

/* GPU Sensor */
#custom-waysensor-rs-gpu,
[id^="custom/waysensor-rs-gpu-"] {
    background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);
    color: white;
}

#custom-waysensor-rs-gpu.warning,
[id^="custom/waysensor-rs-gpu-"].warning {
    background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}

#custom-waysensor-rs-gpu.critical,
[id^="custom/waysensor-rs-gpu-"].critical {
    background: linear-gradient(135deg, #ff6b6b 0%, #ee5a24 100%);
}

/* Disk Sensor */
#custom-waysensor-rs-disk,
[id^="custom/waysensor-rs-disk-"] {
    background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
    color: white;
}

#custom-waysensor-rs-disk.warning,
[id^="custom/waysensor-rs-disk-"].warning {
    background: linear-gradient(135deg, #fdbb2d 0%, #22c1c3 100%);
}

#custom-waysensor-rs-disk.critical,
[id^="custom/waysensor-rs-disk-"].critical {
    background: linear-gradient(135deg, #ff6b6b 0%, #ee5a24 100%);
}

/* Battery Sensor */
#custom-waysensor-rs-battery,
[id^="custom/waysensor-rs-battery-"] {
    background: linear-gradient(135deg, #a8edea 0%, #fed6e3 100%);
    color: #333;
}

#custom-waysensor-rs-battery.warning,
[id^="custom/waysensor-rs-battery-"].warning {
    background: linear-gradient(135deg, #ffecd2 0%, #fcb69f 100%);
}

#custom-waysensor-rs-battery.critical,
[id^="custom/waysensor-rs-battery-"].critical {
    background: linear-gradient(135deg, #ff6b6b 0%, #ee5a24 100%);
    color: white;
}

/* Animations */
@keyframes pulse {
    from {
        opacity: 1;
    }
    to {
        opacity: 0.7;
    }
}

/* Hover effects */
[id^="custom/waysensor-rs"]:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 8px rgba(0,0,0,0.2);
}

/* Tooltip styling */
tooltip {
    background: rgba(0, 0, 0, 0.8);
    border-radius: 8px;
    padding: 8px;
    color: white;
    font-family: monospace;
    font-size: 12px;
}
"#.to_string()
}

fn generate_install_script(hardware: &HardwareInfo) -> Result<String, Box<dyn std::error::Error>> {
    let mut script = String::from(r#"#!/bin/bash
# waysensor-rs Installation Script
# Generated automatically by waysensor-rs-discover

set -e  # Exit on any error

echo "🚀 waysensor-rs Installation Script"
echo "=============================="
echo ""

# Define install directory
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

# Function to check if a binary exists
check_binary() {
    local binary_name="$1"
    if [ -f "$INSTALL_DIR/$binary_name" ] || command -v "$binary_name" &> /dev/null; then
        return 0
    else
        return 1
    fi
}

# Track what needs to be built
NEED_BUILD=false
MISSING_BINARIES=()

echo "🔍 Checking for existing installations..."
echo ""

"#);

    // Check each required binary
    let mut binaries_to_check = Vec::new();
    
    if hardware.cpu.available {
        binaries_to_check.push("waysensor-rs-cpu");
    }
    if hardware.memory.available {
        binaries_to_check.push("waysensor-rs-memory");
    }
    for gpu in &hardware.gpus {
        if gpu.available {
            binaries_to_check.push("waysensor-rs-amd-gpu");
            break;
        }
    }
    for disk in &hardware.disks {
        if disk.available && (disk.path == "/" || disk.path == "/home") {
            binaries_to_check.push("waysensor-rs-disk");
            break;
        }
    }
    for iface in &hardware.network {
        if iface.available {
            binaries_to_check.push("waysensor-rs-network");
            break;
        }
    }
    for battery in &hardware.battery {
        if battery.available {
            binaries_to_check.push("waysensor-rs-battery");
            break;
        }
    }
    binaries_to_check.push("waysensor-rs-discover");

    // Generate check for each binary
    for binary in &binaries_to_check {
        script.push_str(&format!(r#"if check_binary "{}"; then
    echo "  ✅ {} is already installed"
else
    echo "  ❌ {} is missing"
    MISSING_BINARIES+=("{}")
    NEED_BUILD=true
fi
"#, binary, binary, binary, binary));
    }

    script.push_str(r#"
echo ""

# Decide whether to build or skip
if [ "$NEED_BUILD" = false ]; then
    echo "✅ All required binaries are already installed!"
    echo ""
    echo "📋 Generated Files:"
    echo "  • waysensor-rs-waybar-config.json (waybar configuration)"
    echo "  • waysensor-rs-style.css (CSS styling)"
    echo ""
    echo "🎯 Next Steps:"
    echo "============="
    echo "1. Copy modules from waysensor-rs-waybar-config.json to your waybar config"
    echo "2. Add CSS styling from waysensor-rs-style.css to your waybar CSS"
    echo "3. Restart waybar"
    echo ""
    echo "💡 No installation needed - binaries are already available!"
    exit 0
fi

# If we get here, we need to build
echo "🔧 Building missing waysensor-rs sensors..."
echo ""

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Cargo (Rust) is required but not installed."
    echo "   Please install Rust from https://rustup.rs/"
    echo "   Quick install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "✅ Rust/Cargo found: $(cargo --version)"
echo ""

echo "Missing binaries that will be built:"
for binary in "${MISSING_BINARIES[@]}"; do
    echo "  📦 $binary"
done
echo ""

# Build all sensors at once for efficiency
echo "Building all sensors (this may take a few minutes on first build)..."
cargo build --release --bins

echo ""
echo "✅ Build complete!"
echo ""

# Install to system
echo "📦 Installing binaries to ~/.local/bin..."

# Install available sensors
"#);

    // Install each detected sensor
    if hardware.cpu.available {
        script.push_str("if [ -f \"target/release/waysensor-rs-cpu\" ]; then\n");
        script.push_str("    cp target/release/waysensor-rs-cpu ~/.local/bin/\n");
        script.push_str("    echo \"  ✅ Installed waysensor-rs-cpu\"\n");
        script.push_str("fi\n");
    }
    
    if hardware.memory.available {
        script.push_str("if [ -f \"target/release/waysensor-rs-memory\" ]; then\n");
        script.push_str("    cp target/release/waysensor-rs-memory ~/.local/bin/\n");
        script.push_str("    echo \"  ✅ Installed waysensor-rs-memory\"\n");
        script.push_str("fi\n");
    }
    
    for gpu in &hardware.gpus {
        if gpu.available {
            script.push_str("if [ -f \"target/release/waysensor-rs-amd-gpu\" ]; then\n");
            script.push_str("    cp target/release/waysensor-rs-amd-gpu ~/.local/bin/\n");
            script.push_str("    echo \"  ✅ Installed waysensor-rs-amd-gpu\"\n");
            script.push_str("fi\n");
            break;
        }
    }
    
    for disk in &hardware.disks {
        if disk.available && (disk.path == "/" || disk.path == "/home") {
            script.push_str("if [ -f \"target/release/waysensor-rs-disk\" ]; then\n");
            script.push_str("    cp target/release/waysensor-rs-disk ~/.local/bin/\n");
            script.push_str("    echo \"  ✅ Installed waysensor-rs-disk\"\n");
            script.push_str("fi\n");
            break;
        }
    }
    
    for iface in &hardware.network {
        if iface.available {
            script.push_str("if [ -f \"target/release/waysensor-rs-network\" ]; then\n");
            script.push_str("    cp target/release/waysensor-rs-network ~/.local/bin/\n");
            script.push_str("    echo \"  ✅ Installed waysensor-rs-network\"\n");
            script.push_str("fi\n");
            break;
        }
    }
    
    for battery in &hardware.battery {
        if battery.available {
            script.push_str("if [ -f \"target/release/waysensor-rs-battery\" ]; then\n");
            script.push_str("    cp target/release/waysensor-rs-battery ~/.local/bin/\n");
            script.push_str("    echo \"  ✅ Installed waysensor-rs-battery\"\n");
            script.push_str("fi\n");
            break;
        }
    }

    // Always install discover tool
    script.push_str("if [ -f \"target/release/waysensor-rs-discover\" ]; then\n");
    script.push_str("    cp target/release/waysensor-rs-discover ~/.local/bin/\n");
    script.push_str("    echo \"  ✅ Installed waysensor-rs-discover\"\n");
    script.push_str("fi\n");

    script.push_str(r#"
echo ""

# Add to PATH if needed
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "📌 Adding ~/.local/bin to PATH..."
    
    # Add to multiple shell configs if they exist
    for shell_config in ~/.bashrc ~/.zshrc ~/.profile; do
        if [ -f "$shell_config" ]; then
            echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$shell_config"
            echo "  ✅ Updated $shell_config"
        fi
    done
    
    echo "   ⚠️  Please restart your shell or run: source ~/.bashrc"
    echo ""
fi

# Test installation
echo "🧪 Testing installation..."
if command -v waysensor-rs-cpu &> /dev/null; then
    echo "  ✅ waysensor-rs sensors are in PATH and working"
else
    echo "  ⚠️  Sensors not found in PATH - you may need to restart your shell"
fi
echo ""

echo "✅ Installation complete!"
echo ""
echo "📋 Generated Files:"
echo "  • waysensor-rs-waybar-config.json (waybar configuration)"
echo "  • waysensor-rs-style.css (CSS styling)"
echo ""
echo "🎯 Quick Start:"
echo "============="
echo ""
echo "1. Test a sensor:"
echo "   waysensor-rs-cpu --once --icon-style nerdfont"
echo ""
echo "2. Add to waybar config:"
echo "   Copy modules from waysensor-rs-waybar-config.json to your waybar config"
echo ""
echo "3. Add CSS styling:"
echo "   Copy waysensor-rs-style.css content to your waybar CSS"
echo ""
echo "4. Restart waybar"
echo ""
echo "🎉 Happy monitoring!"

"#);

    Ok(script)
}

// Add bytes_to_human function for use in wizard
mod format {
    pub fn bytes_to_human(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_idx = 0;
        
        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }
        
        if unit_idx == 0 {
            format!("{:.0}{}", size, UNITS[unit_idx])
        } else {
            format!("{:.1}{}", size, UNITS[unit_idx])
        }
    }
}