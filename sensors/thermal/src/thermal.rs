use waysensor_rs_core::{Sensor, SensorConfig, SensorError, Theme, WaybarOutput, format};
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct ThermalSensor {
    name: String,
    zone: String,
    warning_threshold: f64,  // Celsius
    critical_threshold: f64, // Celsius
    theme: Theme,
    config: SensorConfig,
}

impl ThermalSensor {
    /// Create a visual bar gauge for a percentage value.
    /// Returns a string with filled and empty blocks to represent the percentage.
    fn create_gauge(percentage: f64, width: usize) -> String {
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
    
    /// Get a color indicator based on temperature.
    fn get_temperature_indicator(temperature: f64, warning: f64, critical: f64) -> &'static str {
        match temperature {
            t if t >= critical => "🔴",      // Critical - very hot
            t if t >= warning => "🟠",      // Warning - hot
            t if t >= (warning * 0.7) => "🟡", // Warm
            t if t >= 30.0 => "🟢",         // Normal
            _ => "🔵",                      // Cool
        }
    }

    pub fn new(
        zone: Option<String>,
        warning_threshold: f64,
        critical_threshold: f64,
    ) -> Result<Self, SensorError> {
        let zone = if let Some(z) = zone {
            z
        } else {
            Self::find_best_thermal_zone()?
        };
        
        // Validate zone exists
        let zone_path = if zone.starts_with("/") {
            // Already a full path (hwmon sensor)
            zone.clone()
        } else {
            // thermal_zone format
            format!("/sys/class/thermal/{}/temp", zone)
        };
        
        if !Path::new(&zone_path).exists() {
            return Err(SensorError::Unavailable {
                reason: format!("Thermal sensor not found: {}", zone_path),
                is_temporary: false,
            });
        }
        
        // Generate a more descriptive name
        let name = if zone.starts_with("/") {
            // Extract a meaningful name from hwmon path
            let path_parts: Vec<&str> = zone.split('/').collect();
            if let Some(filename) = path_parts.last() {
                format!("thermal-{}", filename.replace("_input", ""))
            } else {
                "thermal-hwmon".to_string()
            }
        } else {
            format!("thermal-{}", zone)
        };
        
        Ok(Self {
            name,
            zone,
            warning_threshold,
            critical_threshold,
            theme: Theme::default(),
            config: SensorConfig::default(),
        })
    }
    
    fn find_best_thermal_zone() -> Result<String, SensorError> {
        // First try thermal_zone interface
        if let Ok(zone) = Self::find_thermal_zone() {
            return Ok(zone);
        }
        
        // Fall back to hwmon interface
        if let Ok(hwmon) = Self::find_hwmon_sensor() {
            return Ok(hwmon);
        }
        
        Err(SensorError::Unavailable {
            reason: "No thermal sensors found (checked both thermal_zone and hwmon interfaces)".to_string(),
            is_temporary: false,
        })
    }
    
    fn find_thermal_zone() -> Result<String, SensorError> {
        let thermal_dir = "/sys/class/thermal";
        let entries = fs::read_dir(thermal_dir)
            .map_err(|e| SensorError::Io(e))?;
        
        // Look for CPU thermal zone
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("thermal_zone") {
                        let type_path = format!("{}/{}/type", thermal_dir, name);
                        if let Ok(zone_type) = fs::read_to_string(&type_path) {
                            let zone_type = zone_type.trim();
                            // Prefer CPU zones
                            if zone_type.contains("x86_pkg_temp") || 
                               zone_type.contains("cpu") || 
                               zone_type.contains("coretemp") {
                                return Ok(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        // If no CPU zone found, use the first available zone
        let entries = fs::read_dir(thermal_dir)
            .map_err(|e| SensorError::Io(e))?;
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("thermal_zone") {
                        return Ok(name.to_string());
                    }
                }
            }
        }
        
        Err(SensorError::Unavailable {
            reason: "No thermal_zone found".to_string(),
            is_temporary: false,
        })
    }
    
    fn find_hwmon_sensor() -> Result<String, SensorError> {
        // Find hwmon temperature sensors and prefer CPU sensors
        let mut candidates = Vec::new();
        
        // Search for hwmon temperature sensors
        if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let hwmon_path = entry.path();
                if let Some(_hwmon_name) = hwmon_path.file_name().and_then(|n| n.to_str()) {
                    // Look for temperature inputs in this hwmon device
                    if let Ok(hwmon_entries) = std::fs::read_dir(&hwmon_path) {
                        for hwmon_entry in hwmon_entries.flatten() {
                            let file_name = hwmon_entry.file_name();
                            if let Some(name) = file_name.to_str() {
                                if name.starts_with("temp") && name.ends_with("_input") {
                                    let full_path = hwmon_entry.path();
                                    
                                    // Check if this has a label to identify CPU temperature
                                    let label_path = full_path.with_file_name(
                                        name.replace("_input", "_label")
                                    );
                                    
                                    let priority = if let Ok(label) = std::fs::read_to_string(&label_path) {
                                        let label = label.trim().to_lowercase();
                                        // Check device name for zenpower (most accurate AMD CPU temp)
                                        let name_path = hwmon_path.join("name");
                                        let device_name = if let Ok(name) = std::fs::read_to_string(&name_path) {
                                            name.trim().to_lowercase()
                                        } else {
                                            String::new()
                                        };
                                        
                                        if device_name.contains("zenpower") && (label.contains("tdie") || label.contains("tctl")) {
                                            110 // Highest priority for zenpower Tdie/Tctl (most accurate AMD)
                                        } else if device_name.contains("k10temp") && (label.contains("tdie") || label.contains("tctl")) {
                                            105 // Very high priority for k10temp Tdie/Tctl
                                        } else if label.contains("cpu") || label.contains("core") || 
                                           label.contains("package") || label.contains("tctl") ||
                                           label.contains("tdie") || label.contains("cputin") {
                                            100 // High priority for CPU sensors
                                        } else {
                                            50 // Medium priority for labeled sensors
                                        }
                                    } else {
                                        // Check hwmon name for CPU indicators
                                        let name_path = hwmon_path.join("name");
                                        if let Ok(hwmon_device_name) = std::fs::read_to_string(&name_path) {
                                            let device_name = hwmon_device_name.trim().to_lowercase();
                                            if device_name.contains("zenpower") {
                                                90 // High priority for zenpower (unlabeled)
                                            } else if device_name.contains("k10temp") {
                                                85 // High priority for k10temp
                                            } else if device_name.contains("cpu") || device_name.contains("core") ||
                                               device_name.contains("coretemp") {
                                                80 // High priority for CPU hwmon devices
                                            } else {
                                                10 // Low priority for unlabeled, non-CPU sensors
                                            }
                                        } else {
                                            10
                                        }
                                    };
                                    
                                    // Test if the sensor reads a valid temperature
                                    if let Ok(temp_content) = std::fs::read_to_string(&full_path) {
                                        if let Ok(millidegrees) = temp_content.trim().parse::<i32>() {
                                            let temp_celsius = millidegrees as f64 / 1000.0;
                                            // Only consider sensors that read reasonable temperatures (5°C to 150°C)
                                            if temp_celsius >= 5.0 && temp_celsius <= 150.0 {
                                                candidates.push((priority, full_path.to_string_lossy().to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by priority (highest first) and return the best candidate
        candidates.sort_by(|a, b| b.0.cmp(&a.0));
        
        if let Some((_, path)) = candidates.first() {
            Ok(path.clone())
        } else {
            Err(SensorError::Unavailable {
                reason: "No hwmon temperature sensors found".to_string(),
                is_temporary: false,
            })
        }
    }
    
    fn read_temperature(&self) -> Result<f64, SensorError> {
        let temp_path = if self.zone.starts_with("/") {
            // Already a full path (hwmon sensor)
            self.zone.clone()
        } else {
            // thermal_zone format
            format!("/sys/class/thermal/{}/temp", self.zone)
        };
        
        let content = fs::read_to_string(&temp_path)
            .map_err(|e| SensorError::Io(e))?;
        
        let millidegrees = content.trim().parse::<i32>()
            .map_err(|e| SensorError::Parse {
                message: format!("Failed to parse temperature: {}", e),
                source: None,
            })?;
        
        // Convert from millidegrees to degrees Celsius
        Ok(millidegrees as f64 / 1000.0)
    }
}

impl Sensor for ThermalSensor {
    type Error = SensorError;
    
    fn read(&mut self) -> Result<WaybarOutput, Self::Error> {
        let temperature = self.read_temperature()?;
        
        // Get appropriate thermal icon based on temperature
        let icon = if temperature < 50.0 {
            &self.config.icons.thermal_low
        } else if temperature < 70.0 {
            &self.config.icons.thermal_medium
        } else {
            &self.config.icons.thermal_high
        };
        let text = format::with_icon_and_colors(
            &format!("{:3.0}°C", temperature),
            icon,
            &self.config,
        );
        
        // Build enhanced tooltip with gauge
        let temp_percentage = ((temperature / self.critical_threshold) * 100.0).min(100.0);
        let temp_gauge = Self::create_gauge(temp_percentage, 12);
        let temp_indicator = Self::get_temperature_indicator(temperature, self.warning_threshold, self.critical_threshold);
        
        let zone_line = format::key_value("Thermal Zone", &self.zone, &self.config);
        let temp_line = format::key_value("Temperature", &format!("{} {:.1}°C {}", 
            temp_gauge, temperature, temp_indicator), &self.config);
        let thresholds_line = format::key_value("Thresholds", &format!("⚠️ {:.0}°C / 🔴 {:.0}°C", 
            self.warning_threshold, self.critical_threshold), &self.config);
        
        let tooltip = format!("{}\n{}\n{}", zone_line, temp_line, thresholds_line);
        
        // Calculate percentage (0°C = 0%, critical = 100%)
        let percentage = ((temperature / self.critical_threshold) * 100.0).min(100.0) as u8;
        
        Ok(format::themed_output(
            text,
            Some(tooltip),
            Some(percentage),
            temperature,
            self.warning_threshold,
            self.critical_threshold,
            &self.theme,
        ))
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn configure(&mut self, config: SensorConfig) -> Result<(), Self::Error> {
        self.theme = config.theme.clone();
        self.config = config;
        Ok(())
    }
}