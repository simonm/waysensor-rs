//! Memory usage monitoring sensor for waysensor-rs.
//!
//! This module provides memory usage monitoring by reading from `/proc/meminfo`
//! and calculating memory usage percentages including RAM and optionally swap.

use waysensor_rs_core::{
    format, Sensor, SensorConfig, SensorError, WaybarOutput,
};
use std::fs;
use std::path::Path;

/// Memory usage sensor that monitors system memory utilization.
///
/// Reads memory statistics from `/proc/meminfo` and can monitor both RAM
/// and swap usage. Supports showing either used memory percentage or
/// available memory percentage.
///
/// # Examples
///
/// ```rust
/// use waysensor_memory::MemorySensor;
/// use waysensor_rs_core::Sensor;
///
/// // Monitor RAM only, show used percentage, 70% warning, 90% critical
/// let mut sensor = MemorySensor::new(70, 90, false, false)?;
/// let output = sensor.read()?;
/// println!("Memory usage: {}", output.text);
/// # Ok::<(), waysensor_rs_core::SensorError>(())
/// ```
#[derive(Debug)]
pub struct MemorySensor {
    name: String,
    config: SensorConfig,
    warning_threshold: f64,
    critical_threshold: f64,
    include_swap: bool,
    show_available: bool,
    usage_history: Vec<f64>,
}

/// Memory statistics from `/proc/meminfo`.
///
/// All values are in bytes for consistency and easier calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryInfo {
    /// Total physical memory
    pub mem_total: u64,
    /// Free physical memory
    pub mem_free: u64,
    /// Available physical memory (free + reclaimable)
    pub mem_available: u64,
    /// Memory used for buffers
    pub mem_buffers: u64,
    /// Memory used for page cache
    pub mem_cached: u64,
    /// Total swap space
    pub swap_total: u64,
    /// Free swap space
    pub swap_free: u64,
}

impl MemoryInfo {
    /// Calculate physical memory currently in use.
    #[must_use]
    pub const fn mem_used(&self) -> u64 {
        self.mem_total.saturating_sub(self.mem_available)
    }
    
    /// Calculate percentage of physical memory in use.
    #[must_use]
    pub fn mem_used_percentage(&self) -> f64 {
        if self.mem_total == 0 {
            0.0
        } else {
            (self.mem_used() as f64 / self.mem_total as f64) * 100.0
        }
    }
    
    /// Calculate percentage of physical memory available.
    #[must_use]
    pub fn mem_available_percentage(&self) -> f64 {
        if self.mem_total == 0 {
            0.0
        } else {
            (self.mem_available as f64 / self.mem_total as f64) * 100.0
        }
    }
    
    /// Calculate swap memory currently in use.
    #[must_use]
    pub const fn swap_used(&self) -> u64 {
        self.swap_total.saturating_sub(self.swap_free)
    }
    
    /// Calculate percentage of swap memory in use.
    #[must_use]
    pub fn swap_used_percentage(&self) -> f64 {
        if self.swap_total == 0 {
            0.0
        } else {
            (self.swap_used() as f64 / self.swap_total as f64) * 100.0
        }
    }
    
    /// Calculate total memory (RAM + swap) currently in use.
    #[must_use]
    pub const fn total_used_with_swap(&self) -> u64 {
        self.mem_used() + self.swap_used()
    }
    
    /// Calculate total memory capacity (RAM + swap).
    #[must_use]
    pub const fn total_capacity_with_swap(&self) -> u64 {
        self.mem_total + self.swap_total
    }
    
    /// Calculate percentage of total memory (RAM + swap) in use.
    #[must_use]
    pub fn total_used_percentage_with_swap(&self) -> f64 {
        let total_capacity = self.total_capacity_with_swap();
        if total_capacity == 0 {
            0.0
        } else {
            (self.total_used_with_swap() as f64 / total_capacity as f64) * 100.0
        }
    }
    
    /// Parse memory information from `/proc/meminfo`.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::Parse`] if the meminfo format is invalid.
    pub fn from_proc_meminfo() -> Result<Self, SensorError> {
        Self::from_proc_meminfo_path(Path::new("/proc/meminfo"))
    }
    
    /// Parse memory information from a meminfo file path (useful for testing).
    pub fn from_proc_meminfo_path(path: &Path) -> Result<Self, SensorError> {
        let content = fs::read_to_string(path)?;
        Self::parse_meminfo_content(&content)
    }
    
    /// Parse memory information from meminfo content.
    fn parse_meminfo_content(content: &str) -> Result<Self, SensorError> {
        let mut mem_total = 0;
        let mut mem_free = 0;
        let mut mem_available = 0;
        let mut mem_buffers = 0;
        let mut mem_cached = 0;
        let mut swap_total = 0;
        let mut swap_free = 0;
        
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            
            let key = parts[0].trim_end_matches(':');
            let value = parts[1].parse::<u64>()
                .map_err(|e| SensorError::parse_with_source(
                    format!("Failed to parse {} value", key), e
                ))?;
            
            // Convert from kB to bytes (meminfo values are in kB)
            let value_bytes = value * 1024;
            
            match key {
                "MemTotal" => mem_total = value_bytes,
                "MemFree" => mem_free = value_bytes,
                "MemAvailable" => mem_available = value_bytes,
                "Buffers" => mem_buffers = value_bytes,
                "Cached" => mem_cached = value_bytes,
                "SwapTotal" => swap_total = value_bytes,
                "SwapFree" => swap_free = value_bytes,
                _ => {} // Ignore other fields
            }
        }
        
        // If MemAvailable is not available (older kernels < 3.14), estimate it
        if mem_available == 0 {
            mem_available = mem_free + mem_buffers + mem_cached;
        }
        
        Ok(Self {
            mem_total,
            mem_free,
            mem_available,
            mem_buffers,
            mem_cached,
            swap_total,
            swap_free,
        })
    }
}

impl MemorySensor {
    /// Path to the proc meminfo file.
    const PROC_MEMINFO_PATH: &'static str = "/proc/meminfo";
    
    
    /// Get a color indicator based on memory usage percentage.
    fn get_usage_indicator(percentage: f64) -> &'static str {
        match percentage {
            p if p >= 90.0 => "🔴",  // Critical
            p if p >= 70.0 => "🟠",  // Warning
            p if p >= 50.0 => "🟡",  // Medium
            p if p >= 25.0 => "🟢",  // Normal
            _ => "⚪",               // Low usage
        }
    }
    
    /// Create a new memory sensor with the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `warning_threshold` - Memory usage percentage that triggers warning state
    /// * `critical_threshold` - Memory usage percentage that triggers critical state
    /// * `include_swap` - Whether to include swap usage in calculations
    /// * `show_available` - Whether to show available memory instead of used
    ///
    /// # Errors
    ///
    /// Returns an error if the thresholds are invalid.
    pub fn new(
        warning_threshold: u8,
        critical_threshold: u8,
        include_swap: bool,
        show_available: bool,
    ) -> Result<Self, SensorError> {
        if critical_threshold <= warning_threshold {
            return Err(SensorError::config(format!(
                "Critical threshold ({}) must be greater than warning threshold ({})",
                critical_threshold, warning_threshold
            )));
        }
        
        Ok(Self {
            name: "memory".to_owned(),
            config: SensorConfig::default(),
            warning_threshold: f64::from(warning_threshold),
            critical_threshold: f64::from(critical_threshold),
            include_swap,
            show_available,
            usage_history: Vec::new(),
        })
    }
    
    /// Create a new memory sensor with default settings.
    ///
    /// Defaults: 70% warning, 90% critical, no swap, show used percentage.
    pub fn with_defaults() -> Result<Self, SensorError> {
        Self::new(70, 90, false, false)
    }
    
    /// Create a new memory sensor that includes swap in calculations.
    pub fn with_swap(
        warning_threshold: u8,
        critical_threshold: u8,
    ) -> Result<Self, SensorError> {
        Self::new(warning_threshold, critical_threshold, true, false)
    }
    
    /// Create a new memory sensor that shows available memory percentage.
    pub fn show_available(
        warning_threshold: u8,
        critical_threshold: u8,
    ) -> Result<Self, SensorError> {
        Self::new(warning_threshold, critical_threshold, false, true)
    }
    
    /// Build a detailed tooltip with memory information.
    fn build_tooltip(&self, info: &MemoryInfo) -> String {
        use waysensor_rs_core::format;
        
        let mem_used = info.mem_used();
        let mem_used_percent = info.mem_used_percentage();
        let mem_available_percent = info.mem_available_percentage();
        
        // Create gauges for memory usage if enabled
        let gauge_enabled = self.config.visuals.tooltip_gauges;
        let gauge_width = self.config.visuals.gauge_width;
        let gauge_style = self.config.visuals.gauge_style;
        
        let mem_gauge = if gauge_enabled {
            format::create_gauge(mem_used_percent, gauge_width, gauge_style)
        } else {
            String::new()
        };
        let mem_indicator = if self.config.visuals.status_indicators {
            Self::get_usage_indicator(mem_used_percent)
        } else {
            ""
        };
        
        let header = format::key_only("Memory Usage", &self.config);
        let used_value = if gauge_enabled {
            format!("{} {} ({:.1}%) {}", mem_gauge, format::bytes_to_human(mem_used), mem_used_percent, mem_indicator)
        } else {
            format!("{} ({:.1}%) {}", format::bytes_to_human(mem_used), mem_used_percent, mem_indicator)
        };
        let used_line = format::key_value("Used", &used_value.trim(), &self.config);
        let available_line = format::key_value("Available", &format!("{} ({:.1}%)", 
            format::bytes_to_human(info.mem_available), mem_available_percent), &self.config);
        let total_line = format::key_value("Total", &format::bytes_to_human(info.mem_total), &self.config);
        
        let mut tooltip = format!("{}\n{}\n{}\n{}", header, used_line, available_line, total_line);
        
        // Add swap information if swap is available
        if info.swap_total > 0 {
            let swap_used = info.swap_used();
            let swap_used_percent = info.swap_used_percentage();
            let swap_free_percent = 100.0 - swap_used_percent;
            
            // Create gauges for swap usage
            let swap_gauge = if gauge_enabled {
                format::create_gauge(swap_used_percent, gauge_width, gauge_style)
            } else {
                String::new()
            };
            let swap_indicator = if self.config.visuals.status_indicators {
                Self::get_usage_indicator(swap_used_percent)
            } else {
                ""
            };
            
            let swap_header = format::key_only("Swap Usage", &self.config);
            let swap_used_value = if gauge_enabled {
                format!("{} {} ({:.1}%) {}", swap_gauge, format::bytes_to_human(swap_used), swap_used_percent, swap_indicator)
            } else {
                format!("{} ({:.1}%) {}", format::bytes_to_human(swap_used), swap_used_percent, swap_indicator)
            };
            let swap_used_line = format::key_value("Used", &swap_used_value.trim(), &self.config);
            let swap_free_line = format::key_value("Free", &format!("{} ({:.1}%)", 
                format::bytes_to_human(info.swap_free), swap_free_percent), &self.config);
            let swap_total_line = format::key_value("Total", &format::bytes_to_human(info.swap_total), &self.config);
            
            tooltip.push_str(&format!("\n\n{}\n{}\n{}\n{}", swap_header, swap_used_line, swap_free_line, swap_total_line));
            
            // Add combined stats if including swap in calculations
            if self.include_swap {
                let total_used = info.total_used_with_swap();
                let total_capacity = info.total_capacity_with_swap();
                let total_used_percent = info.total_used_percentage_with_swap();
                
                // Create gauge for combined usage
                let combined_gauge = if gauge_enabled {
                    format::create_gauge(total_used_percent, gauge_width, gauge_style)
                } else {
                    String::new()
                };
                let combined_indicator = if self.config.visuals.status_indicators {
                    Self::get_usage_indicator(total_used_percent)
                } else {
                    ""
                };
                
                let combined_header = format::key_only("Total (RAM + Swap)", &self.config);
                let combined_used_value = if gauge_enabled {
                    format!("{} {} ({:.1}%) {}", combined_gauge, format::bytes_to_human(total_used), total_used_percent, combined_indicator)
                } else {
                    format!("{} ({:.1}%) {}", format::bytes_to_human(total_used), total_used_percent, combined_indicator)
                };
                let combined_used_line = format::key_value("Used", &combined_used_value.trim(), &self.config);
                let combined_total_line = format::key_value("Total", &format::bytes_to_human(total_capacity), &self.config);
                
                tooltip.push_str(&format!("\n\n{}\n{}\n{}", combined_header, combined_used_line, combined_total_line));
            }
        }
        
        // Add sparkline to tooltip if enabled and we have history
        if self.config.visuals.sparklines && self.usage_history.len() > 1 {
            let sparkline = format::create_sparkline(&self.usage_history, self.config.visuals.sparkline_style);
            if !sparkline.is_empty() {
                let colored_sparkline = format::colored_sparkline(&sparkline, self.config.sparkline_color.as_deref());
                let sparkline_line = format::key_value("Usage History", &colored_sparkline, &self.config);
                tooltip.push_str(&format!("\n{}", sparkline_line));
            }
        }
        
        // Add top processes by memory if enabled
        if self.config.visuals.show_top_processes {
            let top_processes = format::get_top_processes_by_memory(
                self.config.visuals.top_processes_count as usize,
                self.config.visuals.process_name_max_length as usize
            );
            let processes_section = format::format_top_processes(
                &top_processes,
                "Top Processes by Memory",
                self.config.tooltip_label_color.as_deref(),
                self.config.tooltip_value_color.as_deref()
            );
            tooltip.push_str(&processes_section);
        }
        
        tooltip
    }
}

impl Sensor for MemorySensor {
    type Error = SensorError;
    
    fn read(&mut self) -> Result<WaybarOutput, Self::Error> {
        let info = MemoryInfo::from_proc_meminfo()?;
        
        let icon = &self.config.icons.memory;
        
        // Determine what to display and how to theme it
        let (text, percentage, value_for_theming) = if self.show_available {
            // Show available memory percentage
            let available_percent = info.mem_available_percentage();
            let display_text = format!("{:.0}% free", available_percent);
            let text_with_icon = format::with_icon_and_colors(&display_text, icon, &self.config);
            
            // For theming, we want high *usage* to trigger warnings, so invert available
            let usage_for_theming = 100.0 - available_percent;
            let percentage_for_display = usage_for_theming.round().clamp(0.0, 100.0) as u8;
            
            (text_with_icon, Some(percentage_for_display), usage_for_theming)
        } else if self.include_swap {
            // Show combined RAM + swap usage
            let used_percent = info.total_used_percentage_with_swap();
            let display_text = format!("{:3.0}%", used_percent);
            let text_with_icon = format::with_icon_and_colors(&display_text, icon, &self.config);
            let percentage_value = used_percent.round().clamp(0.0, 100.0) as u8;
            
            (text_with_icon, Some(percentage_value), used_percent)
        } else {
            // Show RAM usage only
            let used_percent = info.mem_used_percentage();
            let display_text = format!("{:3.0}%", used_percent);
            let text_with_icon = format::with_icon_and_colors(&display_text, icon, &self.config);
            let percentage_value = used_percent.round().clamp(0.0, 100.0) as u8;
            
            (text_with_icon, Some(percentage_value), used_percent)
        };
        
        // Track usage history for sparklines
        self.usage_history.push(value_for_theming);
        if self.usage_history.len() > self.config.visuals.sparkline_length {
            self.usage_history.remove(0);
        }
        
        let tooltip = self.build_tooltip(&info);
        
        Ok(format::themed_output(
            text,
            Some(tooltip),
            percentage,
            value_for_theming,
            self.warning_threshold,
            self.critical_threshold,
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
    
    fn check_availability(&self) -> Result<(), Self::Error> {
        // Check if /proc/meminfo exists and is readable
        if !Path::new(Self::PROC_MEMINFO_PATH).exists() {
            return Err(SensorError::unavailable(format!(
                "{} does not exist (not a Linux system?)", 
                Self::PROC_MEMINFO_PATH
            )));
        }
        
        // Try to read it to make sure we have permission and it's valid
        MemoryInfo::from_proc_meminfo().map_err(|e| match e {
            SensorError::Io(io_err) if io_err.kind() == std::io::ErrorKind::PermissionDenied => {
                SensorError::permission_denied(Self::PROC_MEMINFO_PATH)
            }
            other => other,
        })?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_info_parsing() {
        let content = r#"
MemTotal:       16384000 kB
MemFree:         4096000 kB
MemAvailable:   12288000 kB
Buffers:         1024000 kB
Cached:          2048000 kB
SwapTotal:       8192000 kB
SwapFree:        6144000 kB
"#;
        
        let info = MemoryInfo::parse_meminfo_content(content).unwrap();
        
        // Values should be converted from kB to bytes
        assert_eq!(info.mem_total, 16_384_000 * 1024);
        assert_eq!(info.mem_free, 4_096_000 * 1024);
        assert_eq!(info.mem_available, 12_288_000 * 1024);
        assert_eq!(info.mem_buffers, 1_024_000 * 1024);
        assert_eq!(info.mem_cached, 2_048_000 * 1024);
        assert_eq!(info.swap_total, 8_192_000 * 1024);
        assert_eq!(info.swap_free, 6_144_000 * 1024);
    }

    #[test]
    fn test_memory_calculations() {
        let info = MemoryInfo {
            mem_total: 16 * 1024 * 1024 * 1024, // 16 GB
            mem_free: 4 * 1024 * 1024 * 1024,   // 4 GB
            mem_available: 12 * 1024 * 1024 * 1024, // 12 GB
            mem_buffers: 1024 * 1024 * 1024,    // 1 GB
            mem_cached: 2 * 1024 * 1024 * 1024, // 2 GB
            swap_total: 8 * 1024 * 1024 * 1024, // 8 GB
            swap_free: 6 * 1024 * 1024 * 1024,  // 6 GB
        };

        // Memory calculations
        assert_eq!(info.mem_used(), 4 * 1024 * 1024 * 1024); // 16 - 12 = 4 GB used
        assert!((info.mem_used_percentage() - 25.0).abs() < 0.1); // 4/16 = 25%
        assert!((info.mem_available_percentage() - 75.0).abs() < 0.1); // 12/16 = 75%

        // Swap calculations
        assert_eq!(info.swap_used(), 2 * 1024 * 1024 * 1024); // 8 - 6 = 2 GB used
        assert!((info.swap_used_percentage() - 25.0).abs() < 0.1); // 2/8 = 25%

        // Combined calculations
        assert_eq!(info.total_used_with_swap(), 6 * 1024 * 1024 * 1024); // 4 + 2 = 6 GB
        assert_eq!(info.total_capacity_with_swap(), 24 * 1024 * 1024 * 1024); // 16 + 8 = 24 GB
        assert!((info.total_used_percentage_with_swap() - 25.0).abs() < 0.1); // 6/24 = 25%
    }

    #[test]
    fn test_memory_info_fallback() {
        // Test content without MemAvailable (older kernels)
        let content = r#"
MemTotal:       16384000 kB
MemFree:         4096000 kB
Buffers:         1024000 kB
Cached:          2048000 kB
SwapTotal:       8192000 kB
SwapFree:        6144000 kB
"#;
        
        let info = MemoryInfo::parse_meminfo_content(content).unwrap();
        
        // MemAvailable should be calculated as MemFree + Buffers + Cached
        let expected_available = (4_096_000 + 1_024_000 + 2_048_000) * 1024;
        assert_eq!(info.mem_available, expected_available);
    }

    #[test]
    fn test_memory_sensor_creation() {
        let sensor = MemorySensor::new(70, 90, false, false).unwrap();
        assert_eq!(sensor.warning_threshold, 70.0);
        assert_eq!(sensor.critical_threshold, 90.0);
        assert!(!sensor.include_swap);
        assert!(!sensor.show_available);
        
        // Test invalid thresholds
        assert!(MemorySensor::new(90, 70, false, false).is_err());
        assert!(MemorySensor::new(80, 80, false, false).is_err());
    }

    #[test]
    fn test_memory_sensor_constructors() {
        let sensor = MemorySensor::with_defaults().unwrap();
        assert_eq!(sensor.warning_threshold, 70.0);
        assert_eq!(sensor.critical_threshold, 90.0);
        
        let sensor = MemorySensor::with_swap(80, 95).unwrap();
        assert!(sensor.include_swap);
        
        let sensor = MemorySensor::show_available(60, 80).unwrap();
        assert!(sensor.show_available);
    }
}