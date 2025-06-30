use waysensor_rs_core::{Sensor, SensorConfig, SensorError, WaybarOutput};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct BatterySensor {
    name: String,
    config: SensorConfig,
    battery_path: PathBuf,
    warning_threshold: u8,
    critical_threshold: u8,
    previous_capacity: Option<u8>,
    previous_time: Option<std::time::Instant>,
}

#[derive(Debug, Clone)]
struct BatteryInfo {
    capacity: u8,
    status: String,
    technology: Option<String>,
    cycle_count: Option<u32>,
    energy_now: Option<u64>,
    energy_full: Option<u64>,
    energy_full_design: Option<u64>,
    power_now: Option<u64>,
    voltage_now: Option<u64>,
    charge_now: Option<u64>,
    charge_full: Option<u64>,
    charge_full_design: Option<u64>,
    current_now: Option<i64>,
    manufacturer: Option<String>,
    model_name: Option<String>,
}

impl BatteryInfo {
    fn time_remaining_hours(&self) -> Option<f64> {
        match self.status.as_str() {
            "Discharging" => {
                if let (Some(energy_now), Some(power_now)) = (self.energy_now, self.power_now) {
                    if power_now > 0 {
                        return Some(energy_now as f64 / power_now as f64);
                    }
                }
                if let (Some(charge_now), Some(current_now)) = (self.charge_now, self.current_now) {
                    if current_now > 0 {
                        return Some(charge_now as f64 / current_now as f64);
                    }
                }
            }
            "Charging" => {
                if let (Some(energy_now), Some(energy_full), Some(power_now)) = 
                    (self.energy_now, self.energy_full, self.power_now) {
                    if power_now > 0 && energy_full > energy_now {
                        return Some((energy_full - energy_now) as f64 / power_now as f64);
                    }
                }
                if let (Some(charge_now), Some(charge_full), Some(current_now)) = 
                    (self.charge_now, self.charge_full, self.current_now) {
                    if current_now > 0 && charge_full > charge_now {
                        return Some((charge_full - charge_now) as f64 / current_now as f64);
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn format_time_remaining(&self) -> String {
        if let Some(hours) = self.time_remaining_hours() {
            let total_minutes = (hours * 60.0) as u32;
            let hours = total_minutes / 60;
            let minutes = total_minutes % 60;
            format!("{}:{:02}", hours, minutes)
        } else {
            "Unknown".to_string()
        }
    }

    fn health_percentage(&self) -> Option<u8> {
        if let (Some(full), Some(design)) = (self.energy_full, self.energy_full_design) {
            if design > 0 {
                return Some(((full as f64 / design as f64) * 100.0) as u8);
            }
        }
        if let (Some(full), Some(design)) = (self.charge_full, self.charge_full_design) {
            if design > 0 {
                return Some(((full as f64 / design as f64) * 100.0) as u8);
            }
        }
        None
    }
}

impl BatterySensor {
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
    
    /// Get a color indicator based on battery percentage and status.
    fn get_battery_indicator(percentage: u8, status: &str) -> &'static str {
        match status {
            "Charging" => "🔋",     // Charging
            "Full" => "✅",         // Full
            _ => match percentage { // Discharging
                p if p >= 80 => "🟢", // High
                p if p >= 50 => "🟡", // Medium
                p if p >= 20 => "🟠", // Low
                _ => "🔴",            // Critical
            }
        }
    }

    pub fn new(
        battery_name: Option<String>, 
        warning_threshold: u8, 
        critical_threshold: u8
    ) -> Result<Self, SensorError> {
        let battery_path = if let Some(name) = battery_name {
            PathBuf::from("/sys/class/power_supply").join(&name)
        } else {
            Self::find_battery()?
        };

        // Verify the battery exists and is actually a battery
        if !battery_path.exists() {
            return Err(SensorError::Unavailable {
                reason: format!("Battery path does not exist: {}", battery_path.display()),
                is_temporary: false,
            });
        }

        // Check if it's actually a battery device
        let type_path = battery_path.join("type");
        if type_path.exists() {
            let device_type = fs::read_to_string(&type_path)
                .map_err(|e| SensorError::Io(e))?
                .trim()
                .to_string();
            
            if device_type != "Battery" {
                return Err(SensorError::Unavailable {
                    reason: format!("Device is not a battery: {} (type: {})", battery_path.display(), device_type),
                    is_temporary: false,
                });
            }
        }

        let name = battery_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("battery")
            .to_string();

        Ok(Self {
            name,
            config: SensorConfig::default(),
            battery_path,
            warning_threshold,
            critical_threshold,
            previous_capacity: None,
            previous_time: None,
        })
    }

    fn find_battery() -> Result<PathBuf, SensorError> {
        let power_supply_dir = Path::new("/sys/class/power_supply");
        
        if !power_supply_dir.exists() {
            return Err(SensorError::Unavailable {
                reason: "Power supply directory not found".to_string(),
                is_temporary: false,
            });
        }

        let entries = fs::read_dir(power_supply_dir)
            .map_err(|e| SensorError::Io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| SensorError::Io(e))?;
            let path = entry.path();
            
            // Check if this is a battery device
            let type_path = path.join("type");
            if type_path.exists() {
                if let Ok(device_type) = fs::read_to_string(&type_path) {
                    if device_type.trim() == "Battery" {
                        return Ok(path);
                    }
                }
            }
        }

        Err(SensorError::Unavailable {
            reason: "No battery found".to_string(),
            is_temporary: true,
        })
    }

    fn read_battery_info(&self) -> Result<BatteryInfo, SensorError> {
        let mut info = BatteryInfo {
            capacity: 0,
            status: "Unknown".to_string(),
            technology: None,
            cycle_count: None,
            energy_now: None,
            energy_full: None,
            energy_full_design: None,
            power_now: None,
            voltage_now: None,
            charge_now: None,
            charge_full: None,
            charge_full_design: None,
            current_now: None,
            manufacturer: None,
            model_name: None,
        };

        // Helper function to read a file and parse as a specific type
        let read_file = |filename: &str| -> Result<String, SensorError> {
            let path = self.battery_path.join(filename);
            fs::read_to_string(&path).map_err(|e| SensorError::Io(e))
        };

        let read_u64 = |filename: &str| -> Option<u64> {
            read_file(filename).ok()?.trim().parse().ok()
        };

        let read_i64 = |filename: &str| -> Option<i64> {
            read_file(filename).ok()?.trim().parse().ok()
        };

        let read_u32 = |filename: &str| -> Option<u32> {
            read_file(filename).ok()?.trim().parse().ok()
        };

        let read_string = |filename: &str| -> Option<String> {
            read_file(filename).ok().map(|s| s.trim().to_string())
        };

        // Read capacity (required)
        info.capacity = read_file("capacity")?
            .trim()
            .parse()
            .map_err(|e| SensorError::Parse {
                message: format!("Failed to parse capacity: {}", e),
                source: None,
            })?;

        // Read status (required)
        info.status = read_file("status")?
            .trim()
            .to_string();

        // Read optional fields
        info.technology = read_string("technology");
        info.cycle_count = read_u32("cycle_count");
        info.energy_now = read_u64("energy_now");
        info.energy_full = read_u64("energy_full");
        info.energy_full_design = read_u64("energy_full_design");
        info.power_now = read_u64("power_now");
        info.voltage_now = read_u64("voltage_now");
        info.charge_now = read_u64("charge_now");
        info.charge_full = read_u64("charge_full");
        info.charge_full_design = read_u64("charge_full_design");
        info.current_now = read_i64("current_now");
        info.manufacturer = read_string("manufacturer");
        info.model_name = read_string("model_name");

        Ok(info)
    }

    fn format_battery_output(&self, info: &BatteryInfo) -> (String, String) {
        use waysensor_rs_core::format;
        
        let is_charging = info.status == "Charging";
        // Select appropriate battery icon based on charge percentage and charging state
        let icon = if is_charging {
            &self.config.icons.battery_charging
        } else {
            match info.capacity {
                90..=100 => &self.config.icons.battery_full,
                65..=89 => &self.config.icons.battery_three_quarters,
                35..=64 => &self.config.icons.battery_half,
                10..=34 => &self.config.icons.battery_quarter,
                _ => &self.config.icons.battery_empty,
            }
        };
        let text = format::with_icon_and_colors(&format!("{:3.0}%", info.capacity), icon, &self.config);

        // Build detailed tooltip with gauges
        let capacity_gauge = Self::create_gauge(info.capacity as f64, 12);
        let capacity_indicator = Self::get_battery_indicator(info.capacity, &info.status);
        
        let capacity_line = format::key_value("Battery", &format!("{} {}% {}", 
            capacity_gauge, info.capacity, capacity_indicator), &self.config);
        let status_line = format::key_value("Status", &info.status, &self.config);
        
        let mut tooltip_lines = vec![capacity_line, status_line];

        // Time remaining
        match info.status.as_str() {
            "Charging" | "Discharging" => {
                let time_str = info.format_time_remaining();
                let action = if info.status == "Charging" { "until full" } else { "remaining" };
                let time_line = format::key_value(&format!("Time {}", action), &time_str, &self.config);
                tooltip_lines.push(time_line);
            }
            _ => {}
        }

        // Device information
        if let Some(ref manufacturer) = info.manufacturer {
            if let Some(ref model) = info.model_name {
                let device_line = format::key_value("Device", &format!("{} {}", manufacturer, model), &self.config);
                tooltip_lines.push(device_line);
            } else {
                let manufacturer_line = format::key_value("Manufacturer", manufacturer, &self.config);
                tooltip_lines.push(manufacturer_line);
            }
        } else if let Some(ref model) = info.model_name {
            let model_line = format::key_value("Model", model, &self.config);
            tooltip_lines.push(model_line);
        }

        // Technology and health
        if let Some(ref tech) = info.technology {
            let tech_line = format::key_value("Technology", tech, &self.config);
            tooltip_lines.push(tech_line);
        }

        if let Some(health) = info.health_percentage() {
            let health_gauge = Self::create_gauge(health as f64, 12);
            let health_indicator = Self::get_battery_indicator(health, "Health");
            let health_line = format::key_value("Health", &format!("{} {}% {}", 
                health_gauge, health, health_indicator), &self.config);
            tooltip_lines.push(health_line);
        }

        if let Some(cycles) = info.cycle_count {
            let cycles_line = format::key_value("Cycles", &cycles.to_string(), &self.config);
            tooltip_lines.push(cycles_line);
        }

        // Power information
        if let Some(power) = info.power_now {
            let power_w = power as f64 / 1_000_000.0; // Convert µW to W
            let power_line = format::key_value("Power", &format!("{:.1}W", power_w), &self.config);
            tooltip_lines.push(power_line);
        }

        if let Some(voltage) = info.voltage_now {
            let voltage_v = voltage as f64 / 1_000_000.0; // Convert µV to V
            let voltage_line = format::key_value("Voltage", &format!("{:.2}V", voltage_v), &self.config);
            tooltip_lines.push(voltage_line);
        }

        // Energy/Charge information
        if let (Some(now), Some(full)) = (info.energy_now, info.energy_full) {
            let now_wh = now as f64 / 1_000_000.0; // Convert µWh to Wh
            let full_wh = full as f64 / 1_000_000.0;
            let energy_percent = (now_wh / full_wh) * 100.0;
            let energy_gauge = if self.config.visuals.tooltip_gauges {
                format::create_gauge(energy_percent, self.config.visuals.gauge_width, self.config.visuals.gauge_style)
            } else {
                String::new()
            };
            let energy_line = format::key_value("Energy", &format!("{} {:.1}Wh / {:.1}Wh", 
                energy_gauge, now_wh, full_wh), &self.config);
            tooltip_lines.push(energy_line);
        } else if let (Some(now), Some(full)) = (info.charge_now, info.charge_full) {
            let now_ah = now as f64 / 1_000_000.0; // Convert µAh to Ah
            let full_ah = full as f64 / 1_000_000.0;
            let charge_percent = (now_ah / full_ah) * 100.0;
            let charge_gauge = Self::create_gauge(charge_percent, 12);
            let charge_line = format::key_value("Charge", &format!("{} {:.2}Ah / {:.2}Ah", 
                charge_gauge, now_ah, full_ah), &self.config);
            tooltip_lines.push(charge_line);
        }

        let tooltip = tooltip_lines.join("\n");

        (text, tooltip)
    }

    fn get_battery_class(&self, info: &BatteryInfo) -> String {
        match info.status.as_str() {
            "Charging" => self.config.theme.good.clone(),
            "Full" => self.config.theme.good.clone(),
            _ => {
                if info.capacity <= self.critical_threshold {
                    self.config.theme.critical.clone()
                } else if info.capacity <= self.warning_threshold {
                    self.config.theme.warning.clone()
                } else {
                    self.config.theme.normal.clone()
                }
            }
        }
    }

    pub fn list_available_batteries() -> Result<Vec<String>, SensorError> {
        let power_supply_dir = Path::new("/sys/class/power_supply");
        
        if !power_supply_dir.exists() {
            return Ok(Vec::new());
        }

        let mut batteries = Vec::new();
        let entries = fs::read_dir(power_supply_dir)
            .map_err(|e| SensorError::Io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| SensorError::Io(e))?;
            let path = entry.path();
            
            // Check if this is a battery device
            let type_path = path.join("type");
            if type_path.exists() {
                if let Ok(device_type) = fs::read_to_string(&type_path) {
                    if device_type.trim() == "Battery" {
                        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                            batteries.push(name.to_string());
                        }
                    }
                }
            }
        }

        Ok(batteries)
    }
}

impl Sensor for BatterySensor {
    type Error = SensorError;

    fn read(&mut self) -> Result<WaybarOutput, Self::Error> {
        let info = self.read_battery_info()?;
        let (text, tooltip) = self.format_battery_output(&info);
        let class = self.get_battery_class(&info);

        // Update tracking for rate calculation
        self.previous_capacity = Some(info.capacity);
        self.previous_time = Some(std::time::Instant::now());

        Ok(WaybarOutput {
            text,
            tooltip: Some(tooltip),
            class: Some(class),
            percentage: Some(info.capacity),
        })
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