use waysensor_rs_core::{Sensor, SensorConfig, SensorError, WaybarOutput, format};
use std::fs;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct NetworkSensor {
    name: String,
    config: SensorConfig,
    interface: String,
    warning_threshold: u64,  // MB/s
    critical_threshold: u64, // MB/s
    show_total: bool,
    upload_only: bool,
    download_only: bool,
    last_stats: Option<NetworkStats>,
    last_time: Option<Instant>,
}

#[derive(Debug, Clone)]
struct NetworkStats {
    rx_bytes: u64,
    tx_bytes: u64,
    rx_packets: u64,
    tx_packets: u64,
}

#[derive(Debug, Clone)]
struct NetworkSpeed {
    download_mbps: f64,
    upload_mbps: f64,
    total_mbps: f64,
}

impl NetworkSensor {
    /// Create a visual bar gauge for a speed value relative to maximum.
    /// Returns a string with filled and empty blocks to represent the speed.
    fn create_speed_gauge(speed_mbps: f64, max_mbps: f64, width: usize) -> String {
        let percentage = if max_mbps > 0.0 {
            ((speed_mbps / max_mbps) * 100.0).min(100.0)
        } else {
            0.0
        };
        
        let filled = ((percentage / 100.0) * width as f64).round() as usize;
        let empty = width.saturating_sub(filled);
        
        // Use Unicode block characters for smooth visualization
        let filled_char = '█';
        let empty_char = '░';
        
        format!("{}{}", 
            filled_char.to_string().repeat(filled),
            empty_char.to_string().repeat(empty)
        )
    }
    
    /// Get a color indicator based on network speed.
    fn get_speed_indicator(speed_mbps: f64, warning: f64, critical: f64) -> &'static str {
        match speed_mbps {
            s if s >= critical => "🔴",     // Critical - very high traffic
            s if s >= warning => "🟠",     // Warning - high traffic
            s if s >= (warning * 0.5) => "🟡", // Medium traffic
            s if s >= 0.1 => "🟢",         // Normal traffic
            _ => "⚫",                      // No traffic
        }
    }

    pub fn new(
        interface: Option<String>,
        warning_threshold: u64,
        critical_threshold: u64,
        show_total: bool,
        upload_only: bool,
        download_only: bool,
    ) -> Result<Self, SensorError> {
        let interface = if let Some(iface) = interface {
            iface
        } else {
            Self::find_primary_interface()?
        };
        
        // Validate interface exists
        let stats_path = format!("/sys/class/net/{}/statistics", interface);
        if !std::path::Path::new(&stats_path).exists() {
            return Err(SensorError::Unavailable {
                reason: format!("Network interface not found: {}", interface),
                is_temporary: false,
            });
        }
        
        Ok(Self {
            name: format!("network-{}", interface),
            config: SensorConfig::default(),
            interface,
            warning_threshold,
            critical_threshold,
            show_total,
            upload_only,
            download_only,
            last_stats: None,
            last_time: None,
        })
    }
    
    fn find_primary_interface() -> Result<String, SensorError> {
        // Look for the primary interface (not loopback, virtual, or docker)
        let interfaces = fs::read_dir("/sys/class/net")
            .map_err(|e| SensorError::Io(e))?;
        
        let mut candidates = Vec::new();
        
        for entry in interfaces {
            if let Ok(entry) = entry {
                if let Some(name) = entry.file_name().to_str() {
                    // Skip virtual interfaces
                    if name.starts_with("lo") || 
                       name.starts_with("veth") || 
                       name.starts_with("br-") ||
                       name.starts_with("docker") ||
                       name.starts_with("virbr") {
                        continue;
                    }
                    
                    // Check if interface is up
                    let operstate_path = format!("/sys/class/net/{}/operstate", name);
                    if let Ok(state) = fs::read_to_string(&operstate_path) {
                        if state.trim() == "up" {
                            // Prefer ethernet over wireless
                            if name.starts_with("eth") || name.starts_with("enp") {
                                return Ok(name.to_string());
                            }
                            candidates.push(name.to_string());
                        }
                    }
                }
            }
        }
        
        if !candidates.is_empty() {
            Ok(candidates[0].clone())
        } else {
            Err(SensorError::Unavailable {
                reason: "No active network interface found".to_string(),
                is_temporary: true,
            })
        }
    }
    
    fn read_interface_stats(&self) -> Result<NetworkStats, SensorError> {
        let stats_dir = format!("/sys/class/net/{}/statistics", self.interface);
        
        let rx_bytes = self.read_stat_file(&format!("{}/rx_bytes", stats_dir))?;
        let tx_bytes = self.read_stat_file(&format!("{}/tx_bytes", stats_dir))?;
        let rx_packets = self.read_stat_file(&format!("{}/rx_packets", stats_dir))?;
        let tx_packets = self.read_stat_file(&format!("{}/tx_packets", stats_dir))?;
        
        Ok(NetworkStats {
            rx_bytes,
            tx_bytes,
            rx_packets,
            tx_packets,
        })
    }
    
    fn read_stat_file(&self, path: &str) -> Result<u64, SensorError> {
        let content = fs::read_to_string(path)
            .map_err(|e| SensorError::Io(e))?;
        
        content.trim().parse::<u64>()
            .map_err(|e| SensorError::Parse {
                message: format!("Failed to parse stat: {}", e),
                source: None,
            })
    }
    
    fn calculate_speed(&self, current: &NetworkStats, last: &NetworkStats, duration: Duration) -> NetworkSpeed {
        let duration_secs = duration.as_secs_f64();
        
        if duration_secs <= 0.0 {
            return NetworkSpeed {
                download_mbps: 0.0,
                upload_mbps: 0.0,
                total_mbps: 0.0,
            };
        }
        
        // Calculate bytes per second, then convert to Mbps
        let rx_bytes_per_sec = (current.rx_bytes.saturating_sub(last.rx_bytes)) as f64 / duration_secs;
        let tx_bytes_per_sec = (current.tx_bytes.saturating_sub(last.tx_bytes)) as f64 / duration_secs;
        
        // Convert bytes/sec to Mbps (1 MB = 1,000,000 bytes)
        let download_mbps = rx_bytes_per_sec / 1_000_000.0;
        let upload_mbps = tx_bytes_per_sec / 1_000_000.0;
        let total_mbps = download_mbps + upload_mbps;
        
        NetworkSpeed {
            download_mbps,
            upload_mbps,
            total_mbps,
        }
    }
    
    fn format_speed(mbps: f64) -> String {
        if mbps >= 1000.0 {
            format!("{:.1}GB/s", mbps / 1000.0)
        } else if mbps >= 1.0 {
            format!("{:.1}MB/s", mbps)
        } else if mbps >= 0.001 {
            format!("{:.0}KB/s", mbps * 1000.0)
        } else {
            "0B/s".to_string()
        }
    }
}

impl Sensor for NetworkSensor {
    type Error = SensorError;
    
    fn read(&mut self) -> Result<WaybarOutput, Self::Error> {
        let current_stats = self.read_interface_stats()?;
        let current_time = Instant::now();
        
        let speed = if let (Some(last_stats), Some(last_time)) = (&self.last_stats, &self.last_time) {
            let duration = current_time.duration_since(*last_time);
            self.calculate_speed(&current_stats, last_stats, duration)
        } else {
            // First read, no speed data available yet
            NetworkSpeed {
                download_mbps: 0.0,
                upload_mbps: 0.0,
                total_mbps: 0.0,
            }
        };
        
        // Update for next reading
        self.last_stats = Some(current_stats.clone());
        self.last_time = Some(current_time);
        
        // Determine which icon to use
        let icon = if self.interface.starts_with("wl") || self.interface.starts_with("wlan") {
            &self.config.icons.network_wifi
        } else {
            &self.config.icons.network_ethernet
        };
        
        let (text, value_for_theming) = if self.upload_only {
            let up_icon = &self.config.icons.network_upload;
            let text = format::with_icon_and_colors(&Self::format_speed(speed.upload_mbps), up_icon, &self.config);
            (text, speed.upload_mbps)
        } else if self.download_only {
            let down_icon = &self.config.icons.network_download;
            let text = format::with_icon_and_colors(&Self::format_speed(speed.download_mbps), down_icon, &self.config);
            (text, speed.download_mbps)
        } else if self.show_total {
            let text = format::with_icon_and_colors(&Self::format_speed(speed.total_mbps), icon, &self.config);
            (text, speed.total_mbps)
        } else {
            let down_icon = &self.config.icons.network_download;
            let up_icon = &self.config.icons.network_upload;
            // Use format::with_icon_and_colors for separate download and upload icons
            let down_text = format::with_icon_and_colors(&Self::format_speed(speed.download_mbps), down_icon, &self.config);
            let up_text = format::with_icon_and_colors(&Self::format_speed(speed.upload_mbps), up_icon, &self.config);
            let text = format!("{} {}", down_text, up_text);
            (text, speed.total_mbps)
        };
        
        let tooltip = self.build_tooltip(&current_stats, &speed);
        
        // Calculate percentage based on total throughput
        let percentage = ((value_for_theming / self.critical_threshold as f64) * 100.0).min(100.0) as u8;
        
        Ok(format::themed_output(
            text,
            Some(tooltip),
            Some(percentage),
            value_for_theming,
            self.warning_threshold as f64,
            self.critical_threshold as f64,
            &self.config.theme,
        ))
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn configure(&mut self, config: SensorConfig) -> Result<(), Self::Error> {
        self.config = config;
        Ok(())
    }
    
    fn config(&self) -> &SensorConfig {
        &self.config
    }
}

impl NetworkSensor {
    fn build_tooltip(&self, stats: &NetworkStats, speed: &NetworkSpeed) -> String {
        use waysensor_rs_core::format;
        
        let max_speed = self.critical_threshold as f64;
        
        // Create gauges for speeds
        let download_gauge = Self::create_speed_gauge(speed.download_mbps, max_speed, 12);
        let upload_gauge = Self::create_speed_gauge(speed.upload_mbps, max_speed, 12);
        let total_gauge = Self::create_speed_gauge(speed.total_mbps, max_speed, 12);
        
        // Get indicators
        let download_indicator = Self::get_speed_indicator(speed.download_mbps, self.warning_threshold as f64, self.critical_threshold as f64);
        let upload_indicator = Self::get_speed_indicator(speed.upload_mbps, self.warning_threshold as f64, self.critical_threshold as f64);
        let total_indicator = Self::get_speed_indicator(speed.total_mbps, self.warning_threshold as f64, self.critical_threshold as f64);
        
        // Build tooltip with styled lines
        let interface_line = format::key_value("Network", &self.interface, &self.config);
        let download_line = format::key_value("Download", &format!("{} {} {}", 
            download_gauge, Self::format_speed(speed.download_mbps), download_indicator), &self.config);
        let upload_line = format::key_value("Upload", &format!("{} {} {}", 
            upload_gauge, Self::format_speed(speed.upload_mbps), upload_indicator), &self.config);
        let total_line = format::key_value("Total", &format!("{} {} {}", 
            total_gauge, Self::format_speed(speed.total_mbps), total_indicator), &self.config);
        
        let transfer_header = format::key_only("Transferred", &self.config);
        let rx_line = format::key_value("RX", &format!("{} ({} packets)", 
            format::bytes_to_human(stats.rx_bytes), stats.rx_packets), &self.config);
        let tx_line = format::key_value("TX", &format!("{} ({} packets)", 
            format::bytes_to_human(stats.tx_bytes), stats.tx_packets), &self.config);
        
        format!("{}\n{}\n{}\n{}\n\n{}\n{}\n{}", 
            interface_line, download_line, upload_line, total_line, 
            transfer_header, rx_line, tx_line)
    }
}