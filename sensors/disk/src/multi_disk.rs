use waysensor_rs_core::{Sensor, SensorConfig, SensorError, Theme, WaybarOutput, format};
use std::path::Path;

#[derive(Debug)]
pub struct MultiDiskSensor {
    name: String,
    paths: Vec<String>,
    warning_threshold: u8,
    critical_threshold: u8,
    show_available: bool,
    display_mode: DisplayMode,
    theme: Theme,
}

#[derive(Debug, Clone)]
pub enum DisplayMode {
    /// Show the path with highest usage
    HighestUsage,
    /// Show combined usage of all paths
    Combined,
    /// Cycle through paths
    Cycle { current: usize },
    /// Show specific path by index
    Specific(usize),
}

#[derive(Debug, Clone)]
struct DiskInfo {
    path: String,
    total: u64,
    used: u64,
    available: u64,
    filesystem: String,
    device: String,
}

impl DiskInfo {
    fn used_percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.used as f64 / self.total as f64) * 100.0
        }
    }
    
    fn available_percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.available as f64 / self.total as f64) * 100.0
        }
    }
}

impl MultiDiskSensor {
    pub fn new(
        paths: Vec<String>,
        warning_threshold: u8,
        critical_threshold: u8,
        show_available: bool,
        display_mode: DisplayMode,
    ) -> Result<Self, SensorError> {
        if paths.is_empty() {
            return Err(SensorError::Unavailable {
                reason: "No paths specified".to_string(),
                is_temporary: false,
            });
        }
        
        // Validate all paths exist
        for path in &paths {
            if !Path::new(path).exists() {
                return Err(SensorError::Unavailable {
                    reason: format!("Path does not exist: {}", path),
                    is_temporary: false,
                });
            }
        }
        
        let name = if paths.len() == 1 {
            format!("disk-{}", paths[0].replace('/', "-"))
        } else {
            "disk-multi".to_string()
        };
        
        Ok(Self {
            name,
            paths,
            warning_threshold,
            critical_threshold,
            show_available,
            display_mode,
            theme: Theme::default(),
        })
    }
    
    fn get_disk_usage(&self, path: &str) -> Result<DiskInfo, SensorError> {
        let output = std::process::Command::new("df")
            .arg("-B1") // Get output in bytes
            .arg("-T")  // Include filesystem type
            .arg(path)
            .output()
            .map_err(|e| SensorError::Io(e))?;
        
        if !output.status.success() {
            return Err(SensorError::Unavailable {
                reason: format!("Failed to get disk usage for {}", path),
                is_temporary: true,
            });
        }
        
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| SensorError::Parse {
                message: format!("Invalid UTF-8: {}", e),
                source: None,
            })?;
        
        // Parse df output (skip header line)
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 7 {
                let device = parts[0].to_string();
                let filesystem = parts[1].to_string();
                let total = parts[2].parse::<u64>()
                    .map_err(|e| SensorError::Parse {
                        message: format!("Failed to parse total: {}", e),
                        source: None,
                    })?;
                let used = parts[3].parse::<u64>()
                    .map_err(|e| SensorError::Parse {
                        message: format!("Failed to parse used: {}", e),
                        source: None,
                    })?;
                let available = parts[4].parse::<u64>()
                    .map_err(|e| SensorError::Parse {
                        message: format!("Failed to parse available: {}", e),
                        source: None,
                    })?;
                
                return Ok(DiskInfo {
                    path: path.to_string(),
                    total,
                    used,
                    available,
                    filesystem,
                    device,
                });
            }
        }
        
        Err(SensorError::Parse {
            message: "Could not parse df output".to_string(),
            source: None,
        })
    }
    
    fn get_all_disk_info(&self) -> Result<Vec<DiskInfo>, SensorError> {
        let mut all_info = Vec::new();
        
        for path in &self.paths {
            match self.get_disk_usage(path) {
                Ok(info) => all_info.push(info),
                Err(e) => eprintln!("Warning: Failed to get disk info for {}: {}", path, e),
            }
        }
        
        if all_info.is_empty() {
            return Err(SensorError::Unavailable {
                reason: "No disk information available".to_string(),
                is_temporary: true,
            });
        }
        
        Ok(all_info)
    }
}

impl Sensor for MultiDiskSensor {
    type Error = SensorError;
    
    fn read(&mut self) -> Result<WaybarOutput, Self::Error> {
        let all_info = self.get_all_disk_info()?;
        
        let (display_info, text_prefix) = match &mut self.display_mode {
            DisplayMode::HighestUsage => {
                let info = all_info.iter()
                    .max_by(|a, b| a.used_percentage().partial_cmp(&b.used_percentage()).unwrap())
                    .unwrap();
                (info.clone(), Some(format!("{}: ", basename(&info.path))))
            },
            DisplayMode::Combined => {
                // Calculate combined usage
                let total: u64 = all_info.iter().map(|i| i.total).sum();
                let used: u64 = all_info.iter().map(|i| i.used).sum();
                let available: u64 = all_info.iter().map(|i| i.available).sum();
                
                let combined = DiskInfo {
                    path: "All disks".to_string(),
                    total,
                    used,
                    available,
                    filesystem: "combined".to_string(),
                    device: format!("{} disks", all_info.len()),
                };
                (combined, None)
            },
            DisplayMode::Cycle { current } => {
                let idx = *current % all_info.len();
                *current = (*current + 1) % all_info.len();
                let info = &all_info[idx];
                (info.clone(), Some(format!("{}: ", basename(&info.path))))
            },
            DisplayMode::Specific(idx) => {
                let info = all_info.get(*idx)
                    .ok_or_else(|| SensorError::Unavailable {
                        reason: format!("No disk at index {}", idx),
                        is_temporary: false,
                    })?;
                (info.clone(), Some(format!("{}: ", basename(&info.path))))
            },
        };
        
        let icon = &self.config().icons.disk;
        let (mut text, percentage, value_for_theming) = if self.show_available {
            let available_percent = display_info.available_percentage();
            (
                format!("{}% free", available_percent.round() as u8),
                Some((100.0_f64 - available_percent).round() as u8),
                100.0 - available_percent,
            )
        } else {
            let used_percent = display_info.used_percentage();
            (
                format!("{}%", used_percent.round() as u8),
                Some(used_percent.round() as u8),
                used_percent,
            )
        };
        
        // Add prefix if needed
        if let Some(prefix) = text_prefix {
            text = format!("{}{}", prefix, text);
        }
        
        // Add icon
        text = format::with_icon_and_colors(&text, icon, &self.config());
        
        let tooltip = self.build_tooltip(&all_info, &display_info);
        
        Ok(format::themed_output(
            text,
            Some(tooltip),
            percentage,
            value_for_theming,
            self.warning_threshold as f64,
            self.critical_threshold as f64,
            &self.theme,
        ))
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn configure(&mut self, config: SensorConfig) -> Result<(), Self::Error> {
        self.theme = config.theme;
        Ok(())
    }
}

impl MultiDiskSensor {
    fn build_tooltip(&self, all_info: &[DiskInfo], display_info: &DiskInfo) -> String {
        let mut tooltip = String::new();
        
        // Show current disk info first
        tooltip.push_str(&format!(
            "Current: {}\nDevice: {} ({})\nUsed: {} ({:.1}%)\nAvailable: {} ({:.1}%)\nTotal: {}",
            display_info.path,
            display_info.device,
            display_info.filesystem,
            format::bytes_to_human(display_info.used),
            display_info.used_percentage(),
            format::bytes_to_human(display_info.available),
            display_info.available_percentage(),
            format::bytes_to_human(display_info.total)
        ));
        
        // If monitoring multiple disks, show all
        if all_info.len() > 1 {
            tooltip.push_str("\n\nAll monitored disks:");
            for info in all_info {
                tooltip.push_str(&format!(
                    "\n• {}: {} / {} ({:.0}%)",
                    basename(&info.path),
                    format::bytes_to_human(info.used),
                    format::bytes_to_human(info.total),
                    info.used_percentage()
                ));
            }
        }
        
        tooltip
    }
}

fn basename(path: &str) -> &str {
    if path == "/" {
        "root"
    } else {
        path.rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(path)
    }
}