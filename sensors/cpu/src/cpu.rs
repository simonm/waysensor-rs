//! CPU usage monitoring sensor for waysensor-rs.
//!
//! This module provides CPU usage monitoring by reading from `/proc/stat`
//! and calculating the percentage of CPU time spent in active (non-idle) states.

use waysensor_rs_core::{
    format, Sensor, SensorConfig, SensorError, WaybarOutput,
};
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

/// CPU usage sensor that monitors system CPU utilization.
///
/// Reads CPU statistics from `/proc/stat` and calculates usage percentages
/// by comparing consecutive readings. Provides configurable warning and
/// critical thresholds for theming.
///
/// # Examples
///
/// ```rust
/// use waysensor_cpu::CpuSensor;
/// use waysensor_rs_core::Sensor;
///
/// let mut sensor = CpuSensor::new(70, 90)?;
/// let output = sensor.read()?;
/// println!("CPU usage: {}", output.text);
/// # Ok::<(), waysensor_rs_core::SensorError>(())
/// ```
#[derive(Debug)]
pub struct CpuSensor {
    name: String,
    config: SensorConfig,
    warning_threshold: f64,
    critical_threshold: f64,
    prev_stats: Option<(CpuStats, Instant)>,
    prev_core_stats: Option<Vec<PerCoreCpuStats>>,
    min_sample_interval: Duration,
    usage_history: Vec<f64>,
}

/// CPU statistics from `/proc/stat`.
///
/// Represents the different types of CPU time measurements available
/// in the Linux `/proc/stat` file. All values are in "jiffies" (clock ticks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuStats {
    /// Time spent in user mode (normal processes)
    pub user: u64,
    /// Time spent in user mode with low priority (nice)
    pub nice: u64,
    /// Time spent in system mode (kernel)
    pub system: u64,
    /// Time spent idle
    pub idle: u64,
    /// Time waiting for I/O to complete
    pub iowait: u64,
    /// Time servicing hardware interrupts
    pub irq: u64,
    /// Time servicing software interrupts
    pub softirq: u64,
    /// Time stolen by virtualization
    pub steal: u64,
}

/// Per-core CPU statistics.
///
/// Holds statistics for an individual CPU core, including its core number
/// and usage statistics.
#[derive(Debug, Clone)]
pub struct PerCoreCpuStats {
    /// Core number (0-based)
    pub core_id: usize,
    /// CPU statistics for this core
    pub stats: CpuStats,
}

impl PerCoreCpuStats {
    /// Parse per-core CPU statistics from a `/proc/stat` line.
    ///
    /// # Errors
    ///
    /// Returns a [`SensorError::Parse`] if the line format is invalid or
    /// doesn't represent a CPU core.
    pub fn parse_from_proc_stat_line(line: &str) -> Result<Self, SensorError> {
        if !line.starts_with("cpu") || line.starts_with("cpu ") {
            return Err(SensorError::parse("Line is not a CPU core line"));
        }
        
        // Extract core number
        let cpu_prefix = "cpu";
        let core_id_str = line.split_whitespace()
            .next()
            .ok_or_else(|| SensorError::parse("Empty line"))?
            .strip_prefix(cpu_prefix)
            .ok_or_else(|| SensorError::parse("Invalid CPU line format"))?;
            
        let core_id = core_id_str.parse::<usize>()
            .map_err(|e| SensorError::parse_with_source("Failed to parse core ID", e))?;
        
        let stats = CpuStats::parse_from_proc_stat_line(line)?;
        
        Ok(Self { core_id, stats })
    }
}

impl CpuStats {
    /// Calculate the total CPU time across all states.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.user + self.nice + self.system + self.idle + 
        self.iowait + self.irq + self.softirq + self.steal
    }
    
    /// Calculate CPU time spent in active (non-idle) states.
    #[must_use]
    pub const fn active(&self) -> u64 {
        self.total() - self.idle - self.iowait
    }
    
    /// Calculate CPU usage percentage compared to a previous reading.
    ///
    /// Returns the percentage of CPU time spent in active states between
    /// the previous reading and this reading.
    ///
    /// # Returns
    ///
    /// A value between 0.0 and 100.0 representing CPU usage percentage.
    /// Returns 0.0 if no time has elapsed between readings.
    #[must_use]
    pub fn usage_percent(&self, prev: &CpuStats) -> f64 {
        // Calculate differences, handling potential wraparound
        let total_diff = self.total().saturating_sub(prev.total());
        let active_diff = self.active().saturating_sub(prev.active());
        
        if total_diff == 0 {
            0.0
        } else {
            ((active_diff as f64) / (total_diff as f64) * 100.0).clamp(0.0, 100.0)
        }
    }
    
    /// Parse CPU statistics from a `/proc/stat` line.
    ///
    /// # Errors
    ///
    /// Returns a [`SensorError::Parse`] if the line format is invalid or
    /// contains non-numeric values.
    pub fn parse_from_proc_stat_line(line: &str) -> Result<Self, SensorError> {
        if !line.starts_with("cpu") {
            return Err(SensorError::parse("Line does not start with 'cpu'"));
        }
        
        let values: Result<Vec<u64>, _> = line
            .split_whitespace()
            .skip(1) // Skip "cpu" or "cpuN"
            .take(8) // Take up to 8 values
            .map(str::parse)
            .collect();
            
        let values = values.map_err(|e| {
            SensorError::parse_with_source("Failed to parse CPU statistics", e)
        })?;
            
        if values.len() < 4 {
            return Err(SensorError::parse(format!(
                "Insufficient CPU statistics: expected at least 4, got {}", 
                values.len()
            )));
        }
        
        Ok(Self {
            user: values[0],
            nice: values[1], 
            system: values[2],
            idle: values[3],
            iowait: values.get(4).copied().unwrap_or(0),
            irq: values.get(5).copied().unwrap_or(0),
            softirq: values.get(6).copied().unwrap_or(0),
            steal: values.get(7).copied().unwrap_or(0),
        })
    }
}

/// CPU information extracted from `/proc/cpuinfo`.
#[derive(Debug, Clone)]
pub struct CpuInfo {
    /// CPU model name
    pub model_name: String,
    /// Number of logical processors/cores
    pub core_count: usize,
    /// Current frequency in MHz (if available)
    pub frequency_mhz: Option<f64>,
}

impl CpuInfo {
    /// Parse CPU information from `/proc/cpuinfo`.
    pub fn from_proc_cpuinfo() -> Result<Self, SensorError> {
        Self::from_proc_cpuinfo_path(Path::new("/proc/cpuinfo"))
    }
    
    /// Parse CPU information from a cpuinfo file path (useful for testing).
    pub fn from_proc_cpuinfo_path(path: &Path) -> Result<Self, SensorError> {
        let content = fs::read_to_string(path)?;
        Self::parse_cpuinfo_content(&content)
    }
    
    /// Parse CPU information from cpuinfo content.
    fn parse_cpuinfo_content(content: &str) -> Result<Self, SensorError> {
        let mut model_name = None;
        let mut core_count = 0;
        let mut frequency = None;
        
        for line in content.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                
                match key {
                    "model name" if model_name.is_none() => {
                        model_name = Some(value.to_owned());
                    }
                    "processor" => {
                        core_count += 1;
                    }
                    "cpu MHz" if frequency.is_none() => {
                        frequency = value.parse().ok();
                    }
                    _ => {}
                }
            }
        }
        
        Ok(Self {
            model_name: model_name.unwrap_or_else(|| "Unknown CPU".to_owned()),
            core_count,
            frequency_mhz: frequency,
        })
    }
    
    /// Format CPU information as a human-readable string.
    #[must_use]
    pub fn format_info(&self) -> String {
        let mut info = format!("CPU: {}\nCores: {}", self.model_name, self.core_count);
        
        if let Some(freq) = self.frequency_mhz {
            use waysensor_rs_core::format;
            let freq_hz = (freq * 1_000_000.0) as u64;
            info.push_str(&format!("\nFrequency: {}", format::frequency_to_human(freq_hz)));
        }
        
        info
    }
    
    /// Format CPU information with optional coloring for tooltips.
    #[must_use]
    pub fn format_info_colored(&self, config: &SensorConfig) -> String {
        use waysensor_rs_core::format;
        
        let mut lines = Vec::new();
        lines.push(format::key_value("CPU", &self.model_name, config));
        lines.push(format::key_value("Cores", &self.core_count.to_string(), config));
        
        if let Some(freq) = self.frequency_mhz {
            let freq_hz = (freq * 1_000_000.0) as u64;
            let freq_str = format::frequency_to_human(freq_hz);
            lines.push(format::key_value("Frequency", &freq_str, config));
        }
        
        lines.join("\n")
    }
}

impl CpuSensor {
    /// Path to the proc stat file (customizable for testing).
    const PROC_STAT_PATH: &'static str = "/proc/stat";
    
    /// Minimum interval between CPU samples to get meaningful data.
    const MIN_SAMPLE_INTERVAL: Duration = Duration::from_millis(100);
    
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
    
    /// Get a color indicator based on CPU usage percentage.
    fn get_usage_indicator(percentage: f64) -> &'static str {
        match percentage {
            p if p >= 90.0 => "🔴",  // Critical
            p if p >= 70.0 => "🟠",  // Warning
            p if p >= 50.0 => "🟡",  // Medium
            p if p >= 25.0 => "🟢",  // Normal
            _ => "⚪",               // Idle
        }
    }
    
    /// Create a new CPU sensor with the specified thresholds.
    ///
    /// # Arguments
    ///
    /// * `warning_threshold` - CPU usage percentage that triggers warning state
    /// * `critical_threshold` - CPU usage percentage that triggers critical state
    ///
    /// # Errors
    ///
    /// Returns an error if the thresholds are invalid (critical <= warning).
    pub fn new(warning_threshold: u8, critical_threshold: u8) -> Result<Self, SensorError> {
        if critical_threshold <= warning_threshold {
            return Err(SensorError::config(format!(
                "Critical threshold ({}) must be greater than warning threshold ({})",
                critical_threshold, warning_threshold
            )));
        }
        
        Ok(Self {
            name: "cpu".to_owned(),
            config: SensorConfig::default(),
            warning_threshold: f64::from(warning_threshold),
            critical_threshold: f64::from(critical_threshold),
            prev_stats: None,
            prev_core_stats: None,
            min_sample_interval: Self::MIN_SAMPLE_INTERVAL,
            usage_history: Vec::new(),
        })
    }
    
    /// Create a new CPU sensor with default thresholds (70% warning, 90% critical).
    pub fn with_defaults() -> Result<Self, SensorError> {
        Self::new(70, 90)
    }
    
    /// Read CPU statistics from `/proc/stat`.
    fn read_proc_stat() -> Result<CpuStats, SensorError> {
        Self::read_proc_stat_from_path(Path::new(Self::PROC_STAT_PATH))
    }
    
    /// Read CPU statistics from a specific path (useful for testing).
    fn read_proc_stat_from_path(path: &Path) -> Result<CpuStats, SensorError> {
        let content = fs::read_to_string(path)?;
        
        let first_line = content.lines().next().ok_or_else(|| {
            SensorError::invalid_data("Empty /proc/stat file")
        })?;
        
        CpuStats::parse_from_proc_stat_line(first_line)
    }
    
    /// Read all CPU statistics including per-core stats from `/proc/stat`.
    fn read_all_cpu_stats() -> Result<(CpuStats, Vec<PerCoreCpuStats>), SensorError> {
        Self::read_all_cpu_stats_from_path(Path::new(Self::PROC_STAT_PATH))
    }
    
    /// Read all CPU statistics from a specific path (useful for testing).
    fn read_all_cpu_stats_from_path(path: &Path) -> Result<(CpuStats, Vec<PerCoreCpuStats>), SensorError> {
        let content = fs::read_to_string(path)?;
        let mut lines = content.lines();
        
        // First line should be the overall CPU stats
        let first_line = lines.next().ok_or_else(|| {
            SensorError::invalid_data("Empty /proc/stat file")
        })?;
        
        let total_stats = CpuStats::parse_from_proc_stat_line(first_line)?;
        
        // Parse per-core stats
        let mut core_stats = Vec::new();
        for line in lines {
            if line.starts_with("cpu") && !line.starts_with("cpu ") {
                match PerCoreCpuStats::parse_from_proc_stat_line(line) {
                    Ok(stats) => core_stats.push(stats),
                    Err(_) => break, // Stop when we hit non-CPU lines
                }
            }
        }
        
        Ok((total_stats, core_stats))
    }
    
    /// Get CPU information from `/proc/cpuinfo`.
    fn get_cpu_info() -> Result<CpuInfo, SensorError> {
        CpuInfo::from_proc_cpuinfo()
    }
    
    /// Calculate CPU usage, handling the case where we need initial sampling.
    fn calculate_usage(&mut self) -> Result<(f64, Vec<(usize, f64)>), SensorError> {
        let now = Instant::now();
        let (current_stats, current_core_stats) = Self::read_all_cpu_stats()?;
        
        let (usage, core_usages) = match (&self.prev_stats, &self.prev_core_stats) {
            (Some((prev_stats, prev_time)), Some(prev_cores)) => {
                // Check if enough time has passed for a meaningful measurement
                let elapsed = now.duration_since(*prev_time);
                if elapsed < self.min_sample_interval {
                    // Sleep for the remaining time to get a good sample
                    let sleep_time = self.min_sample_interval - elapsed;
                    thread::sleep(sleep_time);
                    
                    // Read again after sleeping
                    let (current_stats, current_core_stats) = Self::read_all_cpu_stats()?;
                    let overall_usage = current_stats.usage_percent(prev_stats);
                    
                    // Calculate per-core usage
                    let mut core_usages = Vec::new();
                    for current_core in &current_core_stats {
                        if let Some(prev_core) = prev_cores.iter()
                            .find(|c| c.core_id == current_core.core_id) {
                            let usage = current_core.stats.usage_percent(&prev_core.stats);
                            core_usages.push((current_core.core_id, usage));
                        }
                    }
                    
                    (overall_usage, core_usages)
                } else {
                    let overall_usage = current_stats.usage_percent(prev_stats);
                    
                    // Calculate per-core usage
                    let mut core_usages = Vec::new();
                    for current_core in &current_core_stats {
                        if let Some(prev_core) = prev_cores.iter()
                            .find(|c| c.core_id == current_core.core_id) {
                            let usage = current_core.stats.usage_percent(&prev_core.stats);
                            core_usages.push((current_core.core_id, usage));
                        }
                    }
                    
                    (overall_usage, core_usages)
                }
            }
            _ => {
                // First read - sleep and read again to get a delta
                thread::sleep(self.min_sample_interval);
                let (second_stats, second_core_stats) = Self::read_all_cpu_stats()?;
                let overall_usage = second_stats.usage_percent(&current_stats);
                
                // Calculate per-core usage
                let mut core_usages = Vec::new();
                for second_core in &second_core_stats {
                    if let Some(first_core) = current_core_stats.iter()
                        .find(|c| c.core_id == second_core.core_id) {
                        let usage = second_core.stats.usage_percent(&first_core.stats);
                        core_usages.push((second_core.core_id, usage));
                    }
                }
                
                (overall_usage, core_usages)
            }
        };
        
        // Update previous stats
        self.prev_stats = Some((current_stats, now));
        self.prev_core_stats = Some(current_core_stats);
        
        Ok((usage, core_usages))
    }
}

impl Sensor for CpuSensor {
    type Error = SensorError;
    
    fn read(&mut self) -> Result<WaybarOutput, Self::Error> {
        let (usage, core_usages) = self.calculate_usage()?;
        
        // Update usage history
        self.usage_history.push(usage);
        if self.usage_history.len() > self.config.visuals.sparkline_length {
            self.usage_history.remove(0);
        }
        
        // Build the main text - just the percentage like other sensors
        let icon = &self.config.icons.cpu;
        let display_text = format!("{:3.0}%", usage);
        let text = format::with_icon_and_colors(&display_text, icon, &self.config);
        
        let tooltip = match Self::get_cpu_info() {
            Ok(info) => {
                use waysensor_rs_core::format;
                
                let info_str = info.format_info_colored(&self.config);
                let overall_usage_line = format::key_value("Overall Usage", &format!("{:.1}%", usage), &self.config);
                let mut tooltip_text = format!("{}\n{}", info_str, overall_usage_line);
                
                // Add sparkline to tooltip if enabled and we have history
                if self.config.visuals.sparklines && self.usage_history.len() > 1 {
                    let sparkline = format::create_sparkline(&self.usage_history, self.config.visuals.sparkline_style);
                    if !sparkline.is_empty() {
                        let colored_sparkline = format::colored_sparkline(&sparkline, self.config.sparkline_color.as_deref());
                        let sparkline_line = format::key_value("Usage History", &colored_sparkline, &self.config);
                        tooltip_text.push_str(&format!("\n{}", sparkline_line));
                    }
                }
                
                // Add per-core usage information with gauges
                if !core_usages.is_empty() {
                    let section_header = format::key_only("Per-Core Usage", &self.config);
                    tooltip_text.push_str(&format!("\n\n{}", section_header));
                    
                    // Sort cores by ID for consistent display
                    let mut sorted_cores = core_usages;
                    sorted_cores.sort_by_key(|&(id, _)| id);
                    
                    // Display each core with a gauge
                    for &(core_id, core_usage) in &sorted_cores {
                        let gauge = Self::create_gauge(core_usage, 10);
                        let indicator = Self::get_usage_indicator(core_usage);
                        let core_label = format::key_only(&format!("Core {:2}", core_id), &self.config);
                        let core_value = format::value_only(&format!("{} {:5.1}% {}", gauge, core_usage, indicator), &self.config);
                        tooltip_text.push_str(&format!("\n  {} {}", core_label, core_value));
                    }
                }
                
                // Add top processes by CPU if enabled
                if self.config.visuals.show_top_processes {
                    let top_processes = format::get_top_processes_by_cpu(
                        self.config.visuals.top_processes_count as usize,
                        self.config.visuals.process_name_max_length as usize
                    );
                    let processes_section = format::format_top_processes(
                        &top_processes,
                        "Top Processes by CPU",
                        self.config.tooltip_label_color.as_deref(),
                        self.config.tooltip_value_color.as_deref()
                    );
                    tooltip_text.push_str(&processes_section);
                }
                
                Some(tooltip_text)
            }
            Err(_) => {
                use waysensor_rs_core::format;
                
                let usage_line = format::key_value("CPU Usage", &format!("{:.1}%", usage), &self.config);
                let mut tooltip_text = usage_line;
                
                // Add sparkline to tooltip if enabled and we have history
                if self.config.visuals.sparklines && self.usage_history.len() > 1 {
                    let sparkline = format::create_sparkline(&self.usage_history, self.config.visuals.sparkline_style);
                    if !sparkline.is_empty() {
                        let colored_sparkline = format::colored_sparkline(&sparkline, self.config.sparkline_color.as_deref());
                        let sparkline_line = format::key_value("Usage History", &colored_sparkline, &self.config);
                        tooltip_text.push_str(&format!("\n{}", sparkline_line));
                    }
                }
                
                // Still try to show per-core usage even if cpuinfo fails
                if !core_usages.is_empty() {
                    let section_header = format::key_only("Per-Core Usage", &self.config);
                    tooltip_text.push_str(&format!("\n\n{}", section_header));
                    
                    let mut sorted_cores = core_usages;
                    sorted_cores.sort_by_key(|&(id, _)| id);
                    
                    // Display each core with a gauge
                    for &(core_id, core_usage) in &sorted_cores {
                        let gauge = Self::create_gauge(core_usage, 10);
                        let indicator = Self::get_usage_indicator(core_usage);
                        let core_label = format::key_only(&format!("Core {:2}", core_id), &self.config);
                        let core_value = format::value_only(&format!("{} {:5.1}% {}", gauge, core_usage, indicator), &self.config);
                        tooltip_text.push_str(&format!("\n  {} {}", core_label, core_value));
                    }
                }
                
                // Add top processes by CPU if enabled
                if self.config.visuals.show_top_processes {
                    let top_processes = format::get_top_processes_by_cpu(
                        self.config.visuals.top_processes_count as usize,
                        self.config.visuals.process_name_max_length as usize
                    );
                    let processes_section = format::format_top_processes(
                        &top_processes,
                        "Top Processes by CPU",
                        self.config.tooltip_label_color.as_deref(),
                        self.config.tooltip_value_color.as_deref()
                    );
                    tooltip_text.push_str(&processes_section);
                }
                
                Some(tooltip_text)
            }
        };
        
        let percentage = usage.round().clamp(0.0, 100.0) as u8;
        
        Ok(format::themed_output(
            text,
            tooltip,
            Some(percentage),
            usage,
            self.warning_threshold,
            self.critical_threshold,
            &self.config.theme,
        ))
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn configure(&mut self, config: SensorConfig) -> Result<(), Self::Error> {
        // Validate the configuration
        if config.update_interval < SensorConfig::MIN_UPDATE_INTERVAL {
            return Err(SensorError::config(format!(
                "Update interval must be at least {}ms",
                SensorConfig::MIN_UPDATE_INTERVAL
            )));
        }
        
        self.config = config;
        Ok(())
    }
    
    fn config(&self) -> &SensorConfig {
        &self.config
    }
    
    fn check_availability(&self) -> Result<(), Self::Error> {
        // Check if /proc/stat exists and is readable
        if !Path::new(Self::PROC_STAT_PATH).exists() {
            return Err(SensorError::unavailable(format!(
                "{} does not exist (not a Linux system?)", 
                Self::PROC_STAT_PATH
            )));
        }
        
        // Try to read it to make sure we have permission
        Self::read_proc_stat().map_err(|e| match e {
            SensorError::Io(io_err) if io_err.kind() == std::io::ErrorKind::PermissionDenied => {
                SensorError::permission_denied(Self::PROC_STAT_PATH)
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
    fn test_cpu_stats_parsing() {
        let line = "cpu  1234 5678 9012 3456 7890 1234 5678 9012";
        let stats = CpuStats::parse_from_proc_stat_line(line).unwrap();
        
        assert_eq!(stats.user, 1234);
        assert_eq!(stats.nice, 5678);
        assert_eq!(stats.system, 9012);
        assert_eq!(stats.idle, 3456);
        assert_eq!(stats.iowait, 7890);
        assert_eq!(stats.irq, 1234);
        assert_eq!(stats.softirq, 5678);
        assert_eq!(stats.steal, 9012);
    }

    #[test]
    fn test_cpu_stats_minimal() {
        let line = "cpu  100 200 300 400";
        let stats = CpuStats::parse_from_proc_stat_line(line).unwrap();
        
        assert_eq!(stats.user, 100);
        assert_eq!(stats.nice, 200);
        assert_eq!(stats.system, 300);
        assert_eq!(stats.idle, 400);
        assert_eq!(stats.iowait, 0);
        assert_eq!(stats.irq, 0);
        assert_eq!(stats.softirq, 0);
        assert_eq!(stats.steal, 0);
    }

    #[test]
    fn test_cpu_stats_usage_calculation() {
        let prev = CpuStats {
            user: 100, nice: 0, system: 50, idle: 850,
            iowait: 0, irq: 0, softirq: 0, steal: 0,
        };
        
        let current = CpuStats {
            user: 200, nice: 0, system: 100, idle: 1700,
            iowait: 0, irq: 0, softirq: 0, steal: 0,
        };
        
        let usage = current.usage_percent(&prev);
        assert!((usage - 15.0).abs() < 0.1); // Should be ~15%
    }

    #[test]
    fn test_cpu_info_parsing() {
        let content = r#"
processor       : 0
model name      : Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz
cpu MHz         : 3700.000

processor       : 1
model name      : Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz
cpu MHz         : 3700.000
"#;
        
        let info = CpuInfo::parse_cpuinfo_content(content).unwrap();
        assert_eq!(info.model_name, "Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz");
        assert_eq!(info.core_count, 2);
        assert_eq!(info.frequency_mhz, Some(3700.0));
    }

    #[test]
    fn test_cpu_sensor_creation() {
        let sensor = CpuSensor::new(70, 90).unwrap();
        assert_eq!(sensor.warning_threshold, 70.0);
        assert_eq!(sensor.critical_threshold, 90.0);
        
        // Test invalid thresholds
        assert!(CpuSensor::new(90, 70).is_err());
        assert!(CpuSensor::new(80, 80).is_err());
    }

    #[test]
    fn test_cpu_sensor_with_defaults() {
        let sensor = CpuSensor::with_defaults().unwrap();
        assert_eq!(sensor.warning_threshold, 70.0);
        assert_eq!(sensor.critical_threshold, 90.0);
    }

    #[test]
    fn test_per_core_cpu_stats_parsing() {
        let line = "cpu0  1234 5678 9012 3456 7890 1234 5678 9012";
        let per_core = PerCoreCpuStats::parse_from_proc_stat_line(line).unwrap();
        
        assert_eq!(per_core.core_id, 0);
        assert_eq!(per_core.stats.user, 1234);
        assert_eq!(per_core.stats.nice, 5678);
        assert_eq!(per_core.stats.system, 9012);
        assert_eq!(per_core.stats.idle, 3456);
        
        // Test multi-digit core numbers
        let line2 = "cpu12  100 200 300 400";
        let per_core2 = PerCoreCpuStats::parse_from_proc_stat_line(line2).unwrap();
        assert_eq!(per_core2.core_id, 12);
        
        // Test invalid lines
        assert!(PerCoreCpuStats::parse_from_proc_stat_line("cpu  1 2 3 4").is_err());
        assert!(PerCoreCpuStats::parse_from_proc_stat_line("notcpu0 1 2 3 4").is_err());
    }
}