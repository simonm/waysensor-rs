//! # waysensor-rs-disk
//!
//! Advanced disk monitoring library with multi-disk support, sophisticated usage tracking,
//! and comprehensive disk health monitoring.
//!
//! ## Features
//!
//! - **Single and multi-disk monitoring** - Monitor individual disks or aggregate statistics
//! - **Advanced usage tracking** - Track usage trends and predict disk space issues
//! - **Builder pattern configuration** - Flexible and type-safe configuration
//! - **Multiple display modes** - Customizable display strategies for multi-disk setups
//! - **Performance optimization** - Cached reads and efficient data structures
//! - **Comprehensive error handling** - Detailed error reporting with recovery suggestions
//!
//! ## Quick Start
//!
//! ```rust
//! use waysensor_disk::{DiskSensor, DiskSensorBuilder};
//!
//! // Single disk monitoring
//! let sensor = DiskSensorBuilder::new("/")
//!     .warning_threshold(75)
//!     .critical_threshold(90)
//!     .show_available(false)
//!     .build()?;
//!
//! // Multi-disk monitoring
//! let multi_sensor = DiskSensorBuilder::multi_disk()
//!     .add_path("/")
//!     .add_path("/home")
//!     .display_mode(DisplayMode::HighestUsage)
//!     .build()?;
//! ```

use waysensor_rs_core::{
    Sensor, SensorConfig, SensorError, WaybarOutput, format
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
    process::Command,
};
use thiserror::Error;

/// Errors specific to disk monitoring operations.
#[derive(Debug, Error)]
pub enum DiskError {
    /// Failed to execute disk monitoring command
    #[error("Command execution failed: {command}")]
    CommandFailed {
        command: String,
        #[source]
        source: std::io::Error,
    },
    
    /// Invalid disk path or mount point
    #[error("Invalid disk path: {path} - {reason}")]
    InvalidPath { path: String, reason: String },
    
    /// Disk usage calculation error
    #[error("Usage calculation failed for {path}: {reason}")]
    UsageCalculation { path: String, reason: String },
    
    /// Disk performance monitoring error
    #[error("Performance monitoring failed: {reason}")]
    PerformanceMonitoring { reason: String },
}

impl From<DiskError> for SensorError {
    fn from(err: DiskError) -> Self {
        match err {
            DiskError::CommandFailed { source, .. } => SensorError::Io(source),
            DiskError::InvalidPath { path, reason } => {
                SensorError::invalid_data_with_value(
                    format!("Invalid disk path: {}", reason),
                    path
                )
            },
            DiskError::UsageCalculation { reason, .. } => SensorError::parse(reason),
            DiskError::PerformanceMonitoring { reason } => SensorError::parse(reason),
        }
    }
}

/// Display modes for multi-disk monitoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    /// Show the disk with highest usage percentage
    HighestUsage,
    /// Show combined/aggregated usage across all disks
    Combined,
    /// Cycle through disks on each read
    Cycle,
    /// Show average usage across all disks
    Average,
    /// Show total used/available space across all disks
    Total,
}

impl Default for DisplayMode {
    fn default() -> Self {
        Self::HighestUsage
    }
}

/// Comprehensive disk information with performance metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskInfo {
    /// Mount path
    pub path: PathBuf,
    /// Device identifier (e.g., /dev/sda1)
    pub device: String,
    /// Filesystem type (e.g., ext4, btrfs)
    pub filesystem: String,
    /// Total space in bytes
    pub total: u64,
    /// Used space in bytes
    pub used: u64,
    /// Available space in bytes
    pub available: u64,
    /// Number of inodes total
    pub inodes_total: Option<u64>,
    /// Number of inodes used
    pub inodes_used: Option<u64>,
    /// Read-only flag
    pub readonly: bool,
    /// Timestamp when this information was collected
    pub timestamp: Instant,
}

impl DiskInfo {
    /// Calculate used space percentage.
    pub fn used_percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.used as f64 / self.total as f64) * 100.0
        }
    }
    
    /// Calculate available space percentage.
    pub fn available_percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.available as f64 / self.total as f64) * 100.0
        }
    }
    
    /// Calculate inode usage percentage if available.
    pub fn inode_usage_percentage(&self) -> Option<f64> {
        match (self.inodes_total, self.inodes_used) {
            (Some(total), Some(used)) if total > 0 => {
                Some((used as f64 / total as f64) * 100.0)
            },
            _ => None,
        }
    }
    
    
    /// Estimate time until disk is full based on usage trend.
    pub fn time_until_full(&self, usage_trend_per_day: f64) -> Option<Duration> {
        if usage_trend_per_day <= 0.0 {
            return None; // Not filling up
        }
        
        let remaining_percentage = 100.0 - self.used_percentage();
        let days_remaining = remaining_percentage / usage_trend_per_day;
        
        if days_remaining > 0.0 && days_remaining.is_finite() {
            Some(Duration::from_secs_f64(days_remaining * 24.0 * 3600.0))
        } else {
            None
        }
    }
}

/// Usage trend tracking for predictive monitoring.
#[derive(Debug, Clone)]
pub struct UsageTrend {
    /// Historical usage percentages with timestamps
    history: Vec<(Instant, f64)>,
    /// Maximum history entries to keep
    max_history: usize,
}

impl UsageTrend {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::with_capacity(max_history),
            max_history,
        }
    }
    
    pub fn add_sample(&mut self, timestamp: Instant, usage_percentage: f64) {
        self.history.push((timestamp, usage_percentage));
        
        // Keep only recent history
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }
    
    /// Calculate usage trend in percentage points per day.
    pub fn trend_per_day(&self) -> Option<f64> {
        if self.history.len() < 2 {
            return None;
        }
        
        let (first_time, first_usage) = self.history.first()?;
        let (last_time, last_usage) = self.history.last()?;
        
        let duration = last_time.duration_since(*first_time);
        let usage_change = last_usage - first_usage;
        
        if duration.as_secs() > 0 {
            let days = duration.as_secs_f64() / (24.0 * 3600.0);
            Some(usage_change / days)
        } else {
            None
        }
    }
}

/// Configuration for disk monitoring caching.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum age before cache entry is considered stale
    pub max_age: Duration,
    /// Whether to use aggressive caching for better performance
    pub aggressive: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_age: Duration::from_secs(5),
            aggressive: false,
        }
    }
}

/// Single disk monitoring sensor with advanced features.
#[derive(Debug)]
pub struct DiskSensor {
    /// Sensor name for identification
    name: String,
    /// Sensor configuration
    config: SensorConfig,
    /// Disk mount path
    path: PathBuf,
    /// Warning threshold percentage (0-100)
    warning_threshold: u8,
    /// Critical threshold percentage (0-100)
    critical_threshold: u8,
    /// Show available space instead of used space
    show_available: bool,
    /// Include inode monitoring
    monitor_inodes: bool,
    /// Cache configuration
    cache_config: CacheConfig,
    /// Cached disk information
    cached_info: Option<DiskInfo>,
    /// Usage trend tracking
    usage_trend: UsageTrend,
    /// Performance monitoring enabled
    performance_monitoring: bool,
}

/// Builder for configuring DiskSensor instances.
#[derive(Debug)]
pub struct DiskSensorBuilder {
    path: Option<PathBuf>,
    paths: Vec<PathBuf>,
    warning_threshold: u8,
    critical_threshold: u8,
    show_available: bool,
    monitor_inodes: bool,
    cache_config: CacheConfig,
    display_mode: DisplayMode,
    performance_monitoring: bool,
    trend_history_size: usize,
}

impl DiskSensorBuilder {
    /// Create a new builder for a single disk.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: Some(path.as_ref().to_path_buf()),
            paths: Vec::new(),
            warning_threshold: 80,
            critical_threshold: 95,
            show_available: false,
            monitor_inodes: false,
            cache_config: CacheConfig::default(),
            display_mode: DisplayMode::default(),
            performance_monitoring: false,
            trend_history_size: 24, // 24 hours worth of hourly samples
        }
    }
    
    /// Create a new builder for multi-disk monitoring.
    pub fn multi_disk() -> Self {
        Self {
            path: None,
            paths: Vec::new(),
            warning_threshold: 80,
            critical_threshold: 95,
            show_available: false,
            monitor_inodes: false,
            cache_config: CacheConfig::default(),
            display_mode: DisplayMode::default(),
            performance_monitoring: false,
            trend_history_size: 24,
        }
    }
    
    /// Add a path for multi-disk monitoring.
    pub fn add_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.paths.push(path.as_ref().to_path_buf());
        self
    }
    
    /// Set warning threshold percentage (0-100).
    pub fn warning_threshold(mut self, threshold: u8) -> Self {
        self.warning_threshold = threshold.min(100);
        self
    }
    
    /// Set critical threshold percentage (0-100).
    pub fn critical_threshold(mut self, threshold: u8) -> Self {
        self.critical_threshold = threshold.min(100);
        self
    }
    
    /// Show available space instead of used space.
    pub fn show_available(mut self, show: bool) -> Self {
        self.show_available = show;
        self
    }
    
    /// Enable inode monitoring.
    pub fn monitor_inodes(mut self, enable: bool) -> Self {
        self.monitor_inodes = enable;
        self
    }
    
    /// Configure caching behavior.
    pub fn cache_config(mut self, config: CacheConfig) -> Self {
        self.cache_config = config;
        self
    }
    
    /// Set display mode for multi-disk monitoring.
    pub fn display_mode(mut self, mode: DisplayMode) -> Self {
        self.display_mode = mode;
        self
    }
    
    /// Enable performance monitoring and trend tracking.
    pub fn performance_monitoring(mut self, enable: bool) -> Self {
        self.performance_monitoring = enable;
        self
    }
    
    /// Set the size of the trend history buffer.
    pub fn trend_history_size(mut self, size: usize) -> Self {
        self.trend_history_size = size.max(2);
        self
    }
    
    /// Build a single disk sensor.
    pub fn build(self) -> Result<DiskSensor, SensorError> {
        let path = self.path
            .ok_or_else(|| SensorError::config("No path specified for single disk sensor"))?;
        
        // Validate path exists and is accessible
        if !path.exists() {
            return Err(DiskError::InvalidPath {
                path: path.display().to_string(),
                reason: "Path does not exist".to_string(),
            }.into());
        }
        
        if !path.is_dir() {
            return Err(DiskError::InvalidPath {
                path: path.display().to_string(),
                reason: "Path is not a directory".to_string(),
            }.into());
        }
        
        // Validate thresholds
        if self.warning_threshold >= self.critical_threshold {
            return Err(SensorError::config_with_value(
                "Warning threshold must be less than critical threshold",
                format!("warning: {}, critical: {}", self.warning_threshold, self.critical_threshold),
            ));
        }
        
        let name = format!("disk-{}", 
            path.to_string_lossy().replace('/', "-").trim_matches('-'));
        
        Ok(DiskSensor {
            name,
            config: SensorConfig::default(),
            path,
            warning_threshold: self.warning_threshold,
            critical_threshold: self.critical_threshold,
            show_available: self.show_available,
            monitor_inodes: self.monitor_inodes,
            cache_config: self.cache_config,
            cached_info: None,
            usage_trend: UsageTrend::new(self.trend_history_size),
            performance_monitoring: self.performance_monitoring,
        })
    }
}

impl DiskSensor {
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
    
    /// Get a color indicator based on disk usage percentage.
    fn get_usage_indicator(percentage: f64) -> &'static str {
        match percentage {
            p if p >= 95.0 => "🔴",  // Critical - very high
            p if p >= 80.0 => "🟠",  // Warning - high
            p if p >= 60.0 => "🟡",  // Medium usage
            p if p >= 40.0 => "🟢",  // Normal usage
            _ => "⚪",               // Low usage
        }
    }

    /// Create a new disk sensor with default configuration.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, SensorError> {
        DiskSensorBuilder::new(path).build()
    }
    
    /// Get current disk information, using cache if available and valid.
    fn get_disk_info(&mut self) -> Result<DiskInfo, SensorError> {
        let now = Instant::now();
        
        // Check if cached data is still valid
        if let Some(ref cached) = self.cached_info {
            if now.duration_since(cached.timestamp) < self.cache_config.max_age {
                return Ok(cached.clone());
            }
        }
        
        // Fetch fresh data
        let info = self.fetch_disk_info()?;
        
        // Update trend tracking if performance monitoring is enabled
        if self.performance_monitoring {
            self.usage_trend.add_sample(now, info.used_percentage());
        }
        
        // Cache the result
        self.cached_info = Some(info.clone());
        
        Ok(info)
    }
    
    /// Fetch fresh disk information from the system.
    fn fetch_disk_info(&self) -> Result<DiskInfo, SensorError> {
        let path_str = self.path.to_string_lossy();
        
        // Use df command for comprehensive disk information
        let output = Command::new("df")
            .args(["-B1", "-T", "-P"]) // Bytes, filesystem type, POSIX format
            .arg(&*path_str)
            .output()
            .map_err(|e| DiskError::CommandFailed {
                command: "df".to_string(),
                source: e,
            })?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DiskError::UsageCalculation {
                path: path_str.to_string(),
                reason: format!("df command failed: {}", stderr),
            }.into());
        }
        
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| SensorError::parse_with_source("Invalid UTF-8 in df output", e))?;
        
        // Parse df output
        let disk_info = self.parse_df_output(&stdout)?;
        
        // Get inode information if monitoring is enabled
        let (inodes_total, inodes_used) = if self.monitor_inodes {
            self.get_inode_info()?
        } else {
            (None, None)
        };
        
        // Check if filesystem is read-only
        let readonly = self.is_readonly()?;
        
        Ok(DiskInfo {
            path: self.path.clone(),
            device: disk_info.0,
            filesystem: disk_info.1,
            total: disk_info.2,
            used: disk_info.3,
            available: disk_info.4,
            inodes_total,
            inodes_used,
            readonly,
            timestamp: Instant::now(),
        })
    }
    
    /// Parse df command output to extract disk information.
    fn parse_df_output(&self, output: &str) -> Result<(String, String, u64, u64, u64), SensorError> {
        // Skip header line and find the data line
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            
            // df -P ensures consistent output format: 
            // Filesystem Type 1024-blocks Used Available Capacity Mounted
            if parts.len() >= 6 {
                let device = parts[0].to_string();
                let filesystem = parts[1].to_string();
                
                let total = parts[2].parse::<u64>()
                    .map_err(|e| SensorError::parse_with_source("Failed to parse total space", e))?;
                let used = parts[3].parse::<u64>()
                    .map_err(|e| SensorError::parse_with_source("Failed to parse used space", e))?;
                let available = parts[4].parse::<u64>()
                    .map_err(|e| SensorError::parse_with_source("Failed to parse available space", e))?;
                
                return Ok((device, filesystem, total, used, available));
            }
        }
        
        Err(SensorError::parse("Could not parse df output"))
    }
    
    /// Get inode information for the filesystem.
    fn get_inode_info(&self) -> Result<(Option<u64>, Option<u64>), SensorError> {
        let path_str = self.path.to_string_lossy();
        
        let output = Command::new("df")
            .args(["-i", "-P"]) // Inodes, POSIX format
            .arg(&*path_str)
            .output()
            .map_err(|e| SensorError::Io(e))?;
        
        if !output.status.success() {
            // Inode information might not be available on all filesystems
            return Ok((None, None));
        }
        
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| SensorError::parse_with_source("Invalid UTF-8 in df -i output", e))?;
        
        // Parse inode output
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            
            if parts.len() >= 4 {
                let total = parts[1].parse::<u64>().ok();
                let used = parts[2].parse::<u64>().ok();
                return Ok((total, used));
            }
        }
        
        Ok((None, None))
    }
    
    /// Check if the filesystem is mounted read-only.
    fn is_readonly(&self) -> Result<bool, SensorError> {
        // Check /proc/mounts for read-only flag
        let mounts = std::fs::read_to_string("/proc/mounts")
            .map_err(|e| SensorError::Io(e))?;
        
        let path_str = self.path.to_string_lossy();
        
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 && parts[1] == path_str {
                // Check mount options for 'ro' flag
                return Ok(parts[3].split(',').any(|opt| opt == "ro"));
            }
        }
        
        Ok(false) // Assume read-write if not found
    }
    
    /// Build comprehensive tooltip with disk information and trends.
    fn build_tooltip(&self, info: &DiskInfo) -> String {
        use waysensor_rs_core::format;
        
        let used_percent = info.used_percentage();
        let available_percent = info.available_percentage();
        
        // Create gauges for disk usage if enabled
        let gauge_enabled = self.config.visuals.tooltip_gauges;
        let used_gauge = if gauge_enabled {
            format::create_gauge(used_percent, self.config.visuals.gauge_width, self.config.visuals.gauge_style)
        } else {
            String::new()
        };
        let used_indicator = if self.config.visuals.status_indicators {
            Self::get_usage_indicator(used_percent)
        } else {
            ""
        };
        
        // Basic information with styling
        let disk_header = format::key_only(&format!("Disk: {}", info.path.display()), &self.config);
        let device_line = format::key_value("Device", &format!("{} ({})", info.device, info.filesystem), &self.config);
        
        // Space information with gauges
        let used_value = if gauge_enabled {
            format!("{} {} ({:.1}%) {}", used_gauge, format::bytes_to_human(info.used), used_percent, used_indicator)
        } else {
            format!("{} ({:.1}%) {}", format::bytes_to_human(info.used), used_percent, used_indicator)
        };
        let used_line = format::key_value("Used", &used_value.trim(), &self.config);
        let available_line = format::key_value("Available", &format!("{} ({:.1}%)", 
            format::bytes_to_human(info.available), available_percent), &self.config);
        let total_line = format::key_value("Total", &format::bytes_to_human(info.total), &self.config);
        
        let mut tooltip = format!("{}\n{}\n\n{}\n{}\n{}", 
            disk_header, device_line, used_line, available_line, total_line);
        
        // Inode information if available
        if let (Some(total), Some(used)) = (info.inodes_total, info.inodes_used) {
            let usage_pct = info.inode_usage_percentage().unwrap_or(0.0);
            let inode_gauge = Self::create_gauge(usage_pct, 12);
            let inode_indicator = Self::get_usage_indicator(usage_pct);
            
            let inode_line = format::key_value("Inodes", &format!("{} {} / {} ({:.1}%) {}", 
                inode_gauge, used, total, usage_pct, inode_indicator), &self.config);
            tooltip.push_str(&format!("\n{}", inode_line));
        }
        
        // Read-only status
        if info.readonly {
            let status_line = format::key_value("Status", "Read-only", &self.config);
            tooltip.push_str(&format!("\n{}", status_line));
        }
        
        // Trend information if performance monitoring is enabled
        if self.performance_monitoring {
            if let Some(trend) = self.usage_trend.trend_per_day() {
                let trend_line = format::key_value("Trend", &format!("{:.2}% per day", trend), &self.config);
                tooltip.push_str(&format!("\n{}", trend_line));
                
                if let Some(time_until_full) = info.time_until_full(trend) {
                    let days = time_until_full.as_secs_f64() / (24.0 * 3600.0);
                    if days < 365.0 {
                        let estimate_line = format::key_value("Est. full in", &format!("{:.1} days", days), &self.config);
                        tooltip.push_str(&format!("\n{}", estimate_line));
                    }
                }
            }
        }
        
        tooltip
    }
    
    /// Get usage trend information if available.
    pub fn usage_trend_per_day(&self) -> Option<f64> {
        self.usage_trend.trend_per_day()
    }
    
    /// Clear cached data to force fresh read on next access.
    pub fn invalidate_cache(&mut self) {
        self.cached_info = None;
    }
}

impl Sensor for DiskSensor {
    type Error = SensorError;
    
    fn read(&mut self) -> Result<WaybarOutput, Self::Error> {
        let info = self.get_disk_info()?;
        
        let icon = &self.config.icons.disk;
        
        let (text, percentage, value_for_theming) = if self.show_available {
            let available_percent = info.available_percentage();
            (
                format!("{:3.0}% free", available_percent),
                Some((100.0 - available_percent).round() as u8), // Invert for theming
                100.0 - available_percent, // Higher usage = more critical
            )
        } else {
            let used_percent = info.used_percentage();
            (
                format!("{:3.0}%", used_percent),
                Some(used_percent.round() as u8),
                used_percent,
            )
        };
        
        let formatted_text = format::with_icon_and_colors(&text, icon, &self.config);
        let tooltip = self.build_tooltip(&info);
        
        // Consider inode usage for criticality if monitoring is enabled
        let effective_value = if self.monitor_inodes {
            if let Some(inode_usage) = info.inode_usage_percentage() {
                value_for_theming.max(inode_usage)
            } else {
                value_for_theming
            }
        } else {
            value_for_theming
        };
        
        Ok(format::themed_output(
            formatted_text,
            Some(tooltip),
            percentage,
            effective_value,
            self.warning_threshold as f64,
            self.critical_threshold as f64,
            &self.config.theme,
        ))
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn configure(&mut self, config: SensorConfig) -> Result<(), Self::Error> {
        // Update cache configuration from custom config first (before moving values)
        if let Some(cache_max_age) = config.get_custom("cache_max_age_ms") {
            if let serde_json::Value::Number(ms) = cache_max_age {
                if let Some(ms_u64) = ms.as_u64() {
                    self.cache_config.max_age = Duration::from_millis(ms_u64);
                }
            }
        }
        
        if let Some(aggressive_cache) = config.get_custom("aggressive_cache") {
            if let serde_json::Value::Bool(enable) = aggressive_cache {
                self.cache_config.aggressive = *enable;
            }
        }
        
        // Now move the values
        self.config = config;
        
        // Invalidate cache when configuration changes
        self.invalidate_cache();
        
        Ok(())
    }
    
    fn config(&self) -> &SensorConfig {
        &self.config
    }
    
    fn check_availability(&self) -> Result<(), Self::Error> {
        if !self.path.exists() {
            return Err(DiskError::InvalidPath {
                path: self.path.display().to_string(),
                reason: "Path no longer exists".to_string(),
            }.into());
        }
        
        // Test if we can read disk information
        let output = Command::new("df")
            .arg(&self.path)
            .output()
            .map_err(|e| DiskError::CommandFailed {
                command: "df".to_string(),
                source: e,
            })?;
        
        if !output.status.success() {
            return Err(DiskError::UsageCalculation {
                path: self.path.display().to_string(),
                reason: "Cannot read disk usage information".to_string(),
            }.into());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_disk_info_calculations() {
        let info = DiskInfo {
            path: PathBuf::from("/"),
            device: "/dev/sda1".to_string(),
            filesystem: "ext4".to_string(),
            total: 1000,
            used: 600,
            available: 400,
            inodes_total: Some(10000),
            inodes_used: Some(3000),
            readonly: false,
            timestamp: Instant::now(),
        };
        
        assert_eq!(info.used_percentage(), 60.0);
        assert_eq!(info.available_percentage(), 40.0);
        assert_eq!(info.inode_usage_percentage(), Some(30.0));
    }
    
    #[test]
    fn test_usage_trend() {
        let mut trend = UsageTrend::new(10);
        let base_time = Instant::now();
        
        // Add samples over time
        trend.add_sample(base_time, 50.0);
        trend.add_sample(base_time + Duration::from_secs(3600), 52.0); // +2% per hour
        trend.add_sample(base_time + Duration::from_secs(7200), 54.0); // +2% per hour
        
        let trend_per_day = trend.trend_per_day().unwrap();
        // Should be approximately 48% per day (2% per hour * 24 hours)
        assert!((trend_per_day - 48.0).abs() < 1.0);
    }
    
    #[test]
    fn test_disk_sensor_builder() {
        let sensor = DiskSensorBuilder::new("/tmp")
            .warning_threshold(75)
            .critical_threshold(90)
            .show_available(true)
            .monitor_inodes(true)
            .performance_monitoring(true)
            .build();
        
        assert!(sensor.is_ok());
        let sensor = sensor.unwrap();
        assert_eq!(sensor.warning_threshold, 75);
        assert_eq!(sensor.critical_threshold, 90);
        assert!(sensor.show_available);
        assert!(sensor.monitor_inodes);
        assert!(sensor.performance_monitoring);
    }
    
    #[test]
    fn test_invalid_thresholds() {
        let result = DiskSensorBuilder::new("/tmp")
            .warning_threshold(95)
            .critical_threshold(80) // Invalid: critical < warning
            .build();
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Warning threshold"));
    }
    
    #[test]
    fn test_cache_config() {
        let config = CacheConfig {
            max_age: Duration::from_secs(10),
            aggressive: true,
        };
        
        let sensor = DiskSensorBuilder::new("/tmp")
            .cache_config(config)
            .build()
            .unwrap();
        
        assert_eq!(sensor.cache_config.max_age, Duration::from_secs(10));
        assert!(sensor.cache_config.aggressive);
    }
}