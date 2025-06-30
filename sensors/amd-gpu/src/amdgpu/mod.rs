mod types;
mod reader;
mod formats;

pub use types::*;
pub use reader::*;
// pub use formats::*;

use waysensor_rs_core::{Sensor, SensorConfig, SensorError, WaybarOutput, format};
use std::path::PathBuf;

#[derive(Debug)]
pub struct AmdgpuSensor {
    name: String,
    drm_path: PathBuf,
    temp_warning: u16,
    temp_critical: u16,
    format: OutputFormat,
    config: SensorConfig,
}

fn find_amd_gpu_drm_path() -> Result<PathBuf, SensorError> {
    // Look for AMD GPU in DRM class
    let drm_path = std::path::Path::new("/sys/class/drm");
    if !drm_path.exists() {
        return Err(SensorError::unavailable("DRM subsystem not available"));
    }
    
    // Check each card
    for entry in std::fs::read_dir(drm_path)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            if name.starts_with("card") && !name.contains("-") { // Skip card0-eDP-1 type entries
                let device_path = entry.path().join("device");
                let vendor_path = device_path.join("vendor");
                
                // Check if it's an AMD GPU (vendor ID 0x1002)
                if let Ok(vendor) = std::fs::read_to_string(&vendor_path) {
                    if vendor.trim() == "0x1002" {
                        // Check if gpu_busy_percent exists (confirms AMD GPU support)
                        if device_path.join("gpu_busy_percent").exists() {
                            return Ok(device_path);
                        }
                    }
                }
            }
        }
    }
    
    Err(SensorError::unavailable("No AMD GPU found with sysfs support"))
}

impl AmdgpuSensor {
    pub fn new(
        _file: Option<String>, // Ignore file parameter, auto-detect instead
        temp_warning: u16,
        temp_critical: u16,
        format_str: String,
        _verbose: bool,
    ) -> Result<Self, SensorError> {
        let drm_path = find_amd_gpu_drm_path()?;

        let format = match format_str.as_str() {
            "compact" => OutputFormat::Compact,
            "detailed" => OutputFormat::Detailed,
            "minimal" => OutputFormat::Minimal,
            "power" => OutputFormat::Power,
            "activity" => OutputFormat::Activity,
            _ => OutputFormat::Compact,
        };

        Ok(Self {
            name: "amd-gpu".to_string(),
            drm_path,
            temp_warning,
            temp_critical,
            format,
            config: SensorConfig::default(),
        })
    }
    
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
    
    /// Get a color indicator based on usage percentage and type.
    fn get_usage_indicator(percentage: f64, metric_type: &str) -> &'static str {
        match metric_type {
            "temperature" => match percentage {
                p if p >= 90.0 => "🔴",  // Critical temp
                p if p >= 70.0 => "🟠",  // Warning temp
                p if p >= 50.0 => "🟡",  // Warm
                _ => "🟢",               // Cool
            },
            "power" => match percentage {
                p if p >= 90.0 => "⚡",  // Very high power
                p if p >= 70.0 => "🟠",  // High power
                p if p >= 40.0 => "🟡",  // Medium power
                _ => "🟢",               // Low power
            },
            "activity" => match percentage {
                p if p >= 90.0 => "🔥",  // Very busy
                p if p >= 70.0 => "🟠",  // Busy
                p if p >= 40.0 => "🟡",  // Active
                p if p >= 10.0 => "🟢",  // Light load
                _ => "⚪",               // Idle
            },
            _ => match percentage {
                p if p >= 80.0 => "🔴",
                p if p >= 60.0 => "🟠",
                p if p >= 40.0 => "🟡",
                p if p >= 20.0 => "🟢",
                _ => "⚪",
            }
        }
    }
    
    fn read_sysfs_metrics(&self) -> Result<SimplifiedGpuMetrics, SensorError> {
        // Read temperature from hwmon
        let temp = self.read_temperature()?;
        
        // Read GPU activity percentage
        let activity = self.read_file_u16(&self.drm_path.join("gpu_busy_percent"))?;
        
        // Read power from hwmon (convert from microwatts to watts)
        let power_microwatts = self.read_hwmon_power()?;
        let power_watts = (power_microwatts / 1_000_000) as u16;
        
        // Read frequency (current GPU clock)
        let frequency = self.read_current_frequency()?;
        
        // Read fan speed
        let fan_speed = self.read_fan_speed()?;
        
        Ok(SimplifiedGpuMetrics {
            temperature_edge: temp,
            gpu_activity: activity,
            socket_power: power_watts,
            frequency,
            fan_speed,
        })
    }
    
    fn read_file_u16(&self, path: &std::path::Path) -> Result<u16, SensorError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SensorError::Io(e))?;
        content.trim().parse::<u16>()
            .map_err(|e| SensorError::parse(format!("Failed to parse {}: {}", path.display(), e)))
    }
    
    fn read_file_u32(&self, path: &std::path::Path) -> Result<u32, SensorError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SensorError::Io(e))?;
        content.trim().parse::<u32>()
            .map_err(|e| SensorError::parse(format!("Failed to parse {}: {}", path.display(), e)))
    }
    
    fn read_temperature(&self) -> Result<u16, SensorError> {
        // Look for AMD GPU hwmon temperature
        let hwmon_path = self.drm_path.join("hwmon");
        if let Ok(entries) = std::fs::read_dir(&hwmon_path) {
            for entry in entries.flatten() {
                // Verify this is an AMD GPU hwmon device
                let name_path = entry.path().join("name");
                if let Ok(name) = std::fs::read_to_string(&name_path) {
                    if name.trim() == "amdgpu" {
                        let temp_path = entry.path().join("temp1_input");
                        if temp_path.exists() {
                            let temp_millicelsius = self.read_file_u32(&temp_path)?;
                            return Ok((temp_millicelsius / 1000) as u16);
                        }
                    }
                }
            }
        }
        Ok(50) // Default fallback
    }
    
    fn read_hwmon_power(&self) -> Result<u32, SensorError> {
        // Look for AMD GPU hwmon power
        let hwmon_path = self.drm_path.join("hwmon");
        if let Ok(entries) = std::fs::read_dir(&hwmon_path) {
            for entry in entries.flatten() {
                // Verify this is an AMD GPU hwmon device
                let name_path = entry.path().join("name");
                if let Ok(name) = std::fs::read_to_string(&name_path) {
                    if name.trim() == "amdgpu" {
                        let power_path = entry.path().join("power1_average");
                        if power_path.exists() {
                            return self.read_file_u32(&power_path);
                        }
                    }
                }
            }
        }
        Ok(0) // Default if no power info
    }
    
    fn read_current_frequency(&self) -> Result<u16, SensorError> {
        // Try to read current GPU frequency from DPM
        let freq_path = self.drm_path.join("pp_dpm_sclk");
        if freq_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&freq_path) {
                // Parse current frequency from DPM state (look for line with *)
                for line in content.lines() {
                    if line.contains('*') {
                        if let Some(freq_str) = line.split_whitespace().nth(1) {
                            if let Ok(freq_mhz) = freq_str.replace("Mhz", "").parse::<u16>() {
                                return Ok(freq_mhz);
                            }
                        }
                    }
                }
            }
        }
        Ok(800) // Default fallback
    }
    
    fn read_fan_speed(&self) -> Result<u16, SensorError> {
        // Look for AMD GPU hwmon fan
        let hwmon_path = self.drm_path.join("hwmon");
        if let Ok(entries) = std::fs::read_dir(&hwmon_path) {
            for entry in entries.flatten() {
                // Verify this is an AMD GPU hwmon device
                let name_path = entry.path().join("name");
                if let Ok(name) = std::fs::read_to_string(&name_path) {
                    if name.trim() == "amdgpu" {
                        let fan_path = entry.path().join("pwm1");
                        if fan_path.exists() {
                            let pwm = self.read_file_u16(&fan_path)?;
                            // Convert PWM (0-255) to percentage
                            return Ok((pwm as u32 * 100 / 255) as u16);
                        }
                    }
                }
            }
        }
        Ok(0) // Default if no fan info
    }
}

#[derive(Debug)]
struct SimplifiedGpuMetrics {
    temperature_edge: u16,
    gpu_activity: u16,
    socket_power: u16, // in watts
    frequency: u16,
    fan_speed: u16,
}

impl Sensor for AmdgpuSensor {
    type Error = SensorError;

    fn read(&mut self) -> Result<WaybarOutput, Self::Error> {
        let metrics = self.read_sysfs_metrics()?;
        
        match self.format {
            OutputFormat::Compact => self.format_compact(&metrics),
            OutputFormat::Detailed => self.format_detailed(&metrics),
            OutputFormat::Minimal => self.format_minimal(&metrics),
            OutputFormat::Power => self.format_power(&metrics),
            OutputFormat::Activity => self.format_activity(&metrics),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn configure(&mut self, config: SensorConfig) -> Result<(), Self::Error> {
        self.config = config;
        Ok(())
    }
}

impl AmdgpuSensor {
    fn format_compact(&self, metrics: &SimplifiedGpuMetrics) -> Result<WaybarOutput, SensorError> {
        let icon = &self.config.icons.gpu;
        
        // Build display text based on configuration
        let display_text = self.build_display_text(metrics);
        
        let text = format::with_icon_and_colors(&display_text, icon, &self.config);
        
        let tooltip = self.build_tooltip(metrics);
        
        let temp_percentage = ((metrics.temperature_edge as f64 / 100.0) * 100.0).min(100.0) as u8;
        
        Ok(format::themed_output(
            text,
            Some(tooltip),
            Some(temp_percentage),
            metrics.temperature_edge as f64,
            self.temp_warning as f64,
            self.temp_critical as f64,
            &self.config.theme,
        ))
    }
    
    fn build_display_text(&self, metrics: &SimplifiedGpuMetrics) -> String {
        let mut parts = Vec::new();
        
        // Check for display_order configuration first
        if let Some(display_order) = self.config.custom.get("display_order")
            .and_then(|v| v.as_array()) {
            
            for item in display_order {
                if let Some(field) = item.as_str() {
                    match field {
                        "temperature" => parts.push(format!("{}°C", metrics.temperature_edge)),
                        "power" => parts.push(format!("{}W", metrics.socket_power)),
                        "utilization" => parts.push(format!("{}%", metrics.gpu_activity)),
                        _ => {} // Ignore unknown fields
                    }
                }
            }
        } else {
            // Check individual show flags or default to all three
            let show_temperature = self.config.custom.get("show_temperature")
                .and_then(|v| v.as_bool()).unwrap_or(true);
            let show_power = self.config.custom.get("show_power")
                .and_then(|v| v.as_bool()).unwrap_or(true);
            let show_utilization = self.config.custom.get("show_utilization")
                .and_then(|v| v.as_bool()).unwrap_or(true);
            
            if show_temperature {
                parts.push(format!("{}°C", metrics.temperature_edge));
            }
            if show_power {
                parts.push(format!("{}W", metrics.socket_power));
            }
            if show_utilization {
                parts.push(format!("{}%", metrics.gpu_activity));
            }
        }
        
        // If no parts were configured, default to activity percentage
        if parts.is_empty() {
            parts.push(format!("{}%", metrics.gpu_activity));
        }
        
        parts.join(" ")
    }
    
    fn format_detailed(&self, metrics: &SimplifiedGpuMetrics) -> Result<WaybarOutput, SensorError> {
        let mut text_parts = vec![
            format!("{}°C", metrics.temperature_edge),
            format!("{}W", metrics.socket_power),
            format!("{}%", metrics.gpu_activity),
            format!("{}MHz", metrics.frequency),
        ];
        
        if metrics.fan_speed > 0 {
            text_parts.push(format!("{}%", metrics.fan_speed));
        }
        
        let icon = &self.config.icons.gpu;
        let text = format::with_icon_and_colors(&text_parts.join(" "), icon, &self.config);
        let tooltip = self.build_tooltip(metrics);
        
        let temp_percentage = ((metrics.temperature_edge as f64 / 100.0) * 100.0).min(100.0) as u8;
        
        Ok(format::themed_output(
            text,
            Some(tooltip),
            Some(temp_percentage),
            metrics.temperature_edge as f64,
            self.temp_warning as f64,
            self.temp_critical as f64,
            &self.config.theme,
        ))
    }
    
    fn format_minimal(&self, metrics: &SimplifiedGpuMetrics) -> Result<WaybarOutput, SensorError> {
        let temp = metrics.temperature_edge;
        let icon = &self.config.icons.gpu;
        let text = format::with_icon_and_colors(&format!("{}°C", temp), icon, &self.config);
        let tooltip = self.build_tooltip(metrics);
        
        let temp_percentage = ((temp as f64 / 100.0) * 100.0).min(100.0) as u8;
        
        Ok(format::themed_output(
            text,
            Some(tooltip),
            Some(temp_percentage),
            temp as f64,
            self.temp_warning as f64,
            self.temp_critical as f64,
            &self.config.theme,
        ))
    }
    
    fn format_power(&self, metrics: &SimplifiedGpuMetrics) -> Result<WaybarOutput, SensorError> {
        let power = metrics.socket_power;
        let icon = &self.config.icons.gpu;
        let text = format::with_icon_and_colors(&format!("{}W", power), icon, &self.config);
        let tooltip = self.build_tooltip(metrics);
        
        // Use power as percentage (assuming 300W max for percentage calculation)
        let power_percentage = ((power as f64 / 300.0) * 100.0).min(100.0) as u8;
        
        Ok(format::themed_output(
            text,
            Some(tooltip),
            Some(power_percentage),
            power as f64,
            200.0, // 200W warning
            250.0, // 250W critical
            &self.config.theme,
        ))
    }
    
    fn format_activity(&self, metrics: &SimplifiedGpuMetrics) -> Result<WaybarOutput, SensorError> {
        let activity = metrics.gpu_activity;
        let icon = &self.config.icons.gpu;
        let text = format::with_icon_and_colors(&format!("{}%", activity), icon, &self.config);
        let tooltip = self.build_tooltip(metrics);
        
        Ok(format::themed_output(
            text,
            Some(tooltip),
            Some(activity as u8),
            activity as f64,
            70.0, // 70% warning
            90.0, // 90% critical
            &self.config.theme,
        ))
    }
    
    fn build_tooltip(&self, metrics: &SimplifiedGpuMetrics) -> String {
        use waysensor_rs_core::format;
        
        // Calculate percentages for gauges
        let temp_percentage = ((metrics.temperature_edge as f64 / 100.0) * 100.0).min(100.0);
        let power_percentage = ((metrics.socket_power as f64 / 300.0) * 100.0).min(100.0); // Assume 300W max
        let activity_percentage = metrics.gpu_activity as f64;
        let freq_percentage = ((metrics.frequency as f64 / 3000.0) * 100.0).min(100.0); // Assume 3GHz max
        
        // Create gauges
        let temp_gauge = Self::create_gauge(temp_percentage, 12);
        let power_gauge = Self::create_gauge(power_percentage, 12);
        let activity_gauge = Self::create_gauge(activity_percentage, 12);
        let freq_gauge = Self::create_gauge(freq_percentage, 12);
        
        // Get indicators
        let temp_indicator = Self::get_usage_indicator(temp_percentage, "temperature");
        let power_indicator = Self::get_usage_indicator(power_percentage, "power");
        let activity_indicator = Self::get_usage_indicator(activity_percentage, "activity");
        let freq_indicator = Self::get_usage_indicator(freq_percentage, "frequency");
        
        // Build tooltip with styled lines
        let header = format::key_only("AMD GPU", &self.config);
        let temp_line = format::key_value("Temperature", &format!("{} {}°C {}", 
            temp_gauge, metrics.temperature_edge, temp_indicator), &self.config);
        let power_line = format::key_value("Power", &format!("{} {}W {}", 
            power_gauge, metrics.socket_power, power_indicator), &self.config);
        let activity_line = format::key_value("Activity", &format!("{} {}% {}", 
            activity_gauge, metrics.gpu_activity, activity_indicator), &self.config);
        let freq_line = format::key_value("Frequency", &format!("{} {}MHz {}", 
            freq_gauge, metrics.frequency, freq_indicator), &self.config);
        
        let mut tooltip = format!("{}\n{}\n{}\n{}\n{}", 
            header, temp_line, power_line, activity_line, freq_line);
        
        if metrics.fan_speed > 0 {
            let fan_percentage = ((metrics.fan_speed as f64 / 100.0) * 100.0).min(100.0);
            let fan_gauge = Self::create_gauge(fan_percentage, 12);
            let fan_indicator = Self::get_usage_indicator(fan_percentage, "fan");
            let fan_line = format::key_value("Fan Speed", &format!("{} {}% {}", 
                fan_gauge, metrics.fan_speed, fan_indicator), &self.config);
            tooltip.push_str(&format!("\n{}", fan_line));
        }
        
        tooltip
    }
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Compact,
    Detailed,
    Minimal,
    Power,
    Activity,
}

// ThrottleStatus and find_gpu_metrics_file are imported from types.rs