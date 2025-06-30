use std::fs;
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub interface_type: InterfaceType,
    pub is_up: bool,
    pub has_ip: bool,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub activity_score: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterfaceType {
    Ethernet,
    Wireless,
    Virtual,
    Loopback,
    Unknown,
}

/// Detect and rank network interfaces by activity
pub fn detect_active_interfaces() -> Result<Vec<InterfaceInfo>, Box<dyn std::error::Error>> {
    let mut interfaces = Vec::new();
    
    // First pass: collect all interfaces
    for entry in fs::read_dir("/sys/class/net")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        
        // Get interface type
        let interface_type = get_interface_type(&name)?;
        
        // Skip certain virtual interfaces
        if should_skip_interface(&name, &interface_type) {
            continue;
        }
        
        // Check if interface is up
        let operstate_path = format!("/sys/class/net/{}/operstate", name);
        let is_up = fs::read_to_string(&operstate_path)
            .map(|s| s.trim() == "up")
            .unwrap_or(false);
        
        // Check if it has an IP address
        let has_ip = has_ip_address(&name);
        
        // Get current packet counts
        let rx_packets = read_stat(&name, "rx_packets")?;
        let tx_packets = read_stat(&name, "tx_packets")?;
        
        interfaces.push(InterfaceInfo {
            name,
            interface_type,
            is_up,
            has_ip,
            rx_packets,
            tx_packets,
            activity_score: 0.0,
        });
    }
    
    // Test activity by monitoring packet changes
    test_interface_activity(&mut interfaces)?;
    
    // Sort by activity score (highest first)
    interfaces.sort_by(|a, b| b.activity_score.partial_cmp(&a.activity_score).unwrap());
    
    Ok(interfaces)
}

/// Find the best interface for monitoring
pub fn find_best_interface() -> Result<String, Box<dyn std::error::Error>> {
    let interfaces = detect_active_interfaces()?;
    
    // Find the best interface based on criteria
    for iface in &interfaces {
        // Prefer active ethernet interfaces
        if iface.is_up && iface.has_ip && iface.activity_score > 0.0 {
            if iface.interface_type == InterfaceType::Ethernet {
                return Ok(iface.name.clone());
            }
        }
    }
    
    // Fall back to any active interface
    for iface in &interfaces {
        if iface.is_up && iface.activity_score > 0.0 {
            return Ok(iface.name.clone());
        }
    }
    
    Err("No active network interface found".into())
}

fn get_interface_type(name: &str) -> Result<InterfaceType, Box<dyn std::error::Error>> {
    // Check by name patterns first
    if name == "lo" {
        return Ok(InterfaceType::Loopback);
    }
    
    if name.starts_with("veth") || name.starts_with("br-") || 
       name.starts_with("docker") || name.starts_with("virbr") {
        return Ok(InterfaceType::Virtual);
    }
    
    // Check type file
    let type_path = format!("/sys/class/net/{}/type", name);
    if let Ok(type_str) = fs::read_to_string(type_path) {
        match type_str.trim() {
            "1" | "24" => Ok(InterfaceType::Ethernet),
            "801" | "803" => Ok(InterfaceType::Wireless),
            "772" => Ok(InterfaceType::Loopback),
            _ => Ok(InterfaceType::Unknown),
        }
    } else {
        Ok(InterfaceType::Unknown)
    }
}

fn should_skip_interface(name: &str, iface_type: &InterfaceType) -> bool {
    // Skip loopback
    if *iface_type == InterfaceType::Loopback {
        return true;
    }
    
    // Skip most virtual interfaces
    if *iface_type == InterfaceType::Virtual {
        // But keep bridge interfaces that might be primary
        if !name.starts_with("br") {
            return true;
        }
    }
    
    false
}

fn has_ip_address(interface: &str) -> bool {
    use std::process::Command;
    
    let output = Command::new("ip")
        .args(&["-4", "addr", "show", interface])
        .output();
    
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.contains("inet ")
    } else {
        false
    }
}

fn read_stat(interface: &str, stat: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let path = format!("/sys/class/net/{}/statistics/{}", interface, stat);
    let content = fs::read_to_string(path)?;
    Ok(content.trim().parse()?)
}

fn test_interface_activity(interfaces: &mut Vec<InterfaceInfo>) -> Result<(), Box<dyn std::error::Error>> {
    // Record initial packet counts
    let mut initial_counts = HashMap::new();
    for iface in interfaces.iter() {
        initial_counts.insert(
            iface.name.clone(),
            (iface.rx_packets, iface.tx_packets)
        );
    }
    
    // Wait a bit to measure activity
    sleep(Duration::from_millis(500));
    
    // Check packet count changes
    for iface in interfaces.iter_mut() {
        let new_rx = read_stat(&iface.name, "rx_packets").unwrap_or(iface.rx_packets);
        let new_tx = read_stat(&iface.name, "tx_packets").unwrap_or(iface.tx_packets);
        
        let (old_rx, old_tx) = initial_counts[&iface.name];
        let rx_delta = new_rx.saturating_sub(old_rx);
        let tx_delta = new_tx.saturating_sub(old_tx);
        
        // Calculate activity score
        let mut score = 0.0;
        
        // Base score from packet activity
        score += (rx_delta + tx_delta) as f64;
        
        // Bonus for being up
        if iface.is_up {
            score += 100.0;
        }
        
        // Bonus for having IP
        if iface.has_ip {
            score += 50.0;
        }
        
        // Bonus for ethernet
        if iface.interface_type == InterfaceType::Ethernet {
            score += 25.0;
        }
        
        // Penalty for virtual interfaces
        if iface.interface_type == InterfaceType::Virtual {
            score *= 0.5;
        }
        
        iface.activity_score = score;
        iface.rx_packets = new_rx;
        iface.tx_packets = new_tx;
    }
    
    Ok(())
}