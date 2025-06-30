//! GPU sensor implementation with advanced features.

use crate::{
    GpuMetrics, MetricsReader,
};
use waysensor_rs_core::{
    Sensor, SensorConfig, SensorError, Theme, WaybarOutput, format, IconStyle
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use thiserror::Error;

/// Errors specific to AMD GPU monitoring operations.
#[derive(Debug, Error)]
pub enum GpuError {
    /// GPU metrics file not found or inaccessible
    #[error("GPU metrics file error: {path} - {reason}")]
    MetricsFileError { path: String, reason: String },
    
    /// GPU metrics parsing error
    #[error("Metrics parsing failed: {reason}")]
    MetricsParsingError { reason: String },
    
    /// GPU discovery error
    #[error("GPU discovery failed: {reason}")]
    DiscoveryError { reason: String },
    
    /// Thermal monitoring error
    #[error("Thermal monitoring error: {reason}")]
    ThermalError { reason: String },
    
    /// Performance analytics error
    #[error("Performance analytics error: {reason}")]
    PerformanceError { reason: String },
    
    /// GPU hardware error or anomaly
    #[error("GPU hardware error: {reason}")]
    HardwareError { reason: String },
}

impl From<GpuError> for SensorError {
    fn from(err: GpuError) -> Self {
        match err {
            GpuError::MetricsFileError { reason, .. } => SensorError::unavailable(reason),
            GpuError::MetricsParsingError { reason } => SensorError::parse(reason),
            GpuError::DiscoveryError { reason } => SensorError::unavailable(reason),
            GpuError::ThermalError { reason } => SensorError::invalid_data(reason),
            GpuError::PerformanceError { reason } => SensorError::parse(reason),
            GpuError::HardwareError { reason } => SensorError::invalid_data(reason),
        }
    }
}

/// GPU output format variants for different use cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Compact format: temperature, power, activity
    Compact,
    /// Detailed format: includes frequency and fan speed
    Detailed,
    /// Minimal format: temperature only
    Minimal,
    /// Power-focused format: power consumption and efficiency
    Power,
    /// Activity-focused format: GPU utilization metrics
    Activity,
    /// Thermal-focused format: comprehensive temperature data
    Thermal,
    /// Performance format: clocks, power efficiency, throttling
    Performance,
    /// Custom format with user-defined fields
    Custom(Vec<String>),
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Compact
    }
}

/// Cache strategy for GPU metrics to optimize performance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStrategy {
    /// No caching - always read fresh data
    None,
    /// Basic caching with time-based invalidation
    Basic { max_age: Duration },
    /// Aggressive caching with smart invalidation
    Aggressive { max_age: Duration, change_threshold: f64 },
    /// Memory-mapped file caching for maximum performance
    MemoryMapped,
}

impl Default for CacheStrategy {
    fn default() -> Self {
        Self::Basic { max_age: Duration::from_millis(500) }
    }
}

/// GPU thermal zone information.
#[derive(Debug, Clone)]
pub struct ThermalZone {
    pub name: String,
    pub temperature: f64,
    pub critical_point: Option<f64>,
    pub alert_level: ThermalAlert,
}

/// Thermal alert levels for GPU monitoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThermalAlert {
    Normal,
    Elevated,
    Warning,
    Critical,
    Emergency,
}

/// GPU performance state information.
#[derive(Debug, Clone)]
pub struct PerformanceState {
    pub power_efficiency: f64,  // Performance per watt
    pub thermal_efficiency: f64, // Performance per degree
    pub utilization_efficiency: f64, // Actual vs theoretical performance
    pub bottleneck_analysis: Vec<String>,
    pub optimization_hints: Vec<String>,
}

/// Comprehensive AMD GPU sensor with advanced monitoring capabilities.
#[derive(Debug)]
pub struct AmdgpuSensor {
    /// Sensor name for identification
    name: String,
    /// Sensor configuration
    config: SensorConfig,
    /// Path to GPU metrics file
    metrics_path: PathBuf,
    /// GPU device information
    device_info: GpuDeviceInfo,
    /// Output format configuration
    output_format: OutputFormat,
    /// Temperature thresholds
    temp_warning: f64,
    temp_critical: f64,
    /// Power thresholds (watts)
    power_warning: f64,
    power_critical: f64,
    /// Display configuration
    show_temperature: bool,
    show_power: bool,
    show_utilization: bool,
    show_memory: bool,
    show_frequency: bool,
    /// Metrics reader with caching
    metrics_reader: MetricsReader,
    /// Cache strategy
    cache_strategy: CacheStrategy,
    /// Thermal monitoring
    thermal_monitor: Option<ThermalMonitor>,
    /// Performance analytics
    performance_analytics: Option<PerformanceAnalytics>,
    /// Last known good metrics for comparison
    last_metrics: Option<(Box<dyn GpuMetrics>, Instant)>,
    /// Error recovery state
    consecutive_errors: usize,
    last_error_time: Option<Instant>,
}

/// GPU device information for enhanced monitoring.
#[derive(Debug, Clone)]
pub struct GpuDeviceInfo {
    pub card_name: String,
    pub device_id: String,
    pub vendor_id: String,
    pub driver_version: Option<String>,
    pub memory_size: Option<u64>,
    pub pci_slot: Option<String>,
    pub supported_features: Vec<String>,
}

/// Builder for configuring AmdgpuSensor instances.
#[derive(Debug)]
pub struct AmdgpuSensorBuilder {
    metrics_path: Option<PathBuf>,
    auto_detect: bool,
    output_format: OutputFormat,
    temp_warning: f64,
    temp_critical: f64,
    power_warning: f64,
    power_critical: f64,
    cache_strategy: CacheStrategy,
    enable_thermal_monitoring: bool,
    enable_performance_analytics: bool,
    thermal_zones: Vec<String>,
    custom_fields: Vec<String>,
    error_recovery_enabled: bool,
    max_consecutive_errors: usize,
}

impl AmdgpuSensorBuilder {
    /// Create a new builder with a specific GPU metrics file path.
    pub fn new<P: AsRef<Path>>(metrics_path: P) -> Self {
        Self {
            metrics_path: Some(metrics_path.as_ref().to_path_buf()),
            auto_detect: false,
            output_format: OutputFormat::default(),
            temp_warning: 75.0,
            temp_critical: 90.0,
            power_warning: 200.0,
            power_critical: 250.0,
            cache_strategy: CacheStrategy::default(),
            enable_thermal_monitoring: false,
            enable_performance_analytics: false,
            thermal_zones: Vec::new(),
            custom_fields: Vec::new(),
            error_recovery_enabled: true,
            max_consecutive_errors: 5,
        }
    }
    
    /// Create a builder with automatic GPU detection.
    pub fn auto_detect() -> Self {
        Self {
            metrics_path: None,
            auto_detect: true,
            output_format: OutputFormat::default(),
            temp_warning: 75.0,
            temp_critical: 90.0,
            power_warning: 200.0,
            power_critical: 250.0,
            cache_strategy: CacheStrategy::default(),
            enable_thermal_monitoring: false,
            enable_performance_analytics: false,
            thermal_zones: Vec::new(),
            custom_fields: Vec::new(),
            error_recovery_enabled: true,
            max_consecutive_errors: 5,
        }
    }
    
    /// Set the output format.
    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }
    
    /// Set temperature warning threshold in Celsius.
    pub fn warning_temperature(mut self, temp: f64) -> Self {
        self.temp_warning = temp;
        self
    }
    
    /// Set temperature critical threshold in Celsius.
    pub fn critical_temperature(mut self, temp: f64) -> Self {
        self.temp_critical = temp;
        self
    }
    
    /// Set power warning threshold in watts.
    pub fn warning_power(mut self, power: f64) -> Self {
        self.power_warning = power;
        self
    }
    
    /// Set power critical threshold in watts.
    pub fn critical_power(mut self, power: f64) -> Self {
        self.power_critical = power;
        self
    }
    
    /// Configure caching strategy.
    pub fn cache_strategy(mut self, strategy: CacheStrategy) -> Self {
        self.cache_strategy = strategy;
        self
    }
    
    /// Enable advanced thermal monitoring.
    pub fn thermal_monitoring(mut self, enable: bool) -> Self {
        self.enable_thermal_monitoring = enable;
        self
    }
    
    /// Enable performance analytics.
    pub fn performance_analytics(mut self, enable: bool) -> Self {
        self.enable_performance_analytics = enable;
        self
    }
    
    /// Add custom thermal zones to monitor.
    pub fn thermal_zones<I, S>(mut self, zones: I) -> Self 
    where 
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.thermal_zones = zones.into_iter().map(|s| s.into()).collect();
        self
    }
    
    /// Set custom fields for custom output format.
    pub fn custom_fields<I, S>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.custom_fields = fields.into_iter().map(|s| s.into()).collect();
        self
    }
    
    /// Configure error recovery behavior.
    pub fn error_recovery(mut self, enabled: bool, max_errors: usize) -> Self {
        self.error_recovery_enabled = enabled;
        self.max_consecutive_errors = max_errors;
        self
    }
    
    /// Build the AMDGPU sensor.
    pub fn build(self) -> Result<AmdgpuSensor, SensorError> {
        // Determine GPU metrics path
        let metrics_path = if self.auto_detect {
            GpuDiscovery::find_primary_gpu()?
                .ok_or_else(|| SensorError::unavailable("No compatible AMD GPU found"))?
        } else {
            self.metrics_path
                .ok_or_else(|| SensorError::config("No GPU metrics path specified"))?
        };
        
        // Validate metrics file exists and is readable
        if !metrics_path.exists() {
            return Err(GpuError::MetricsFileError {
                path: metrics_path.display().to_string(),
                reason: "File does not exist".to_string(),
            }.into());
        }
        
        // Validate temperature thresholds
        if self.temp_warning >= self.temp_critical {
            return Err(SensorError::config_with_value(
                "Temperature warning threshold must be less than critical threshold",
                format!("warning: {}, critical: {}", self.temp_warning, self.temp_critical),
            ));
        }
        
        // Validate power thresholds
        if self.power_warning >= self.power_critical {
            return Err(SensorError::config_with_value(
                "Power warning threshold must be less than critical threshold",
                format!("warning: {}, critical: {}", self.power_warning, self.power_critical),
            ));
        }
        
        // Get device information
        let device_info = GpuDiscovery::get_device_info(&metrics_path)?;
        
        // Create metrics reader with specified cache strategy
        let metrics_reader = MetricsReader::with_cache_strategy(self.cache_strategy);
        
        // Initialize thermal monitoring if enabled
        let thermal_monitor = if self.enable_thermal_monitoring {
            Some(ThermalMonitor::new(
                self.temp_warning,
                self.temp_critical,
                self.thermal_zones,
            )?)
        } else {
            None
        };
        
        // Initialize performance analytics if enabled
        let performance_analytics = if self.enable_performance_analytics {
            Some(PerformanceAnalytics::new())
        } else {
            None
        };
        
        // Generate sensor name
        let name = format!("amd-gpu-{}", 
            device_info.card_name.replace(' ', "-").to_lowercase());
        
        Ok(AmdgpuSensor {
            name,
            config: SensorConfig::default(),
            metrics_path,
            device_info,
            output_format: self.output_format,
            temp_warning: self.temp_warning,
            temp_critical: self.temp_critical,
            power_warning: self.power_warning,
            power_critical: self.power_critical,
            show_temperature: true,  // Default to showing all
            show_power: true,
            show_utilization: true,
            show_memory: false,
            show_frequency: false,
            metrics_reader,
            cache_strategy: self.cache_strategy,
            thermal_monitor,
            performance_analytics,
            last_metrics: None,
            consecutive_errors: 0,
            last_error_time: None,
        })
    }
}

impl AmdgpuSensor {
    /// Update display configuration from sensor config
    pub fn update_display_config(&mut self, config: &SensorConfig) {
        // Check for GPU-specific display settings in custom config
        if let Some(show_temp) = config.custom.get("show_temperature")
            .and_then(|v| v.as_bool()) {
            self.show_temperature = show_temp;
        }
        if let Some(show_power) = config.custom.get("show_power")
            .and_then(|v| v.as_bool()) {
            self.show_power = show_power;
        }
        if let Some(show_util) = config.custom.get("show_utilization")
            .and_then(|v| v.as_bool()) {
            self.show_utilization = show_util;
        }
        if let Some(show_mem) = config.custom.get("show_memory")
            .and_then(|v| v.as_bool()) {
            self.show_memory = show_mem;
        }
        if let Some(show_freq) = config.custom.get("show_frequency")
            .and_then(|v| v.as_bool()) {
            self.show_frequency = show_freq;
        }
        
        // Handle display_order - if specified, turn off all flags and only enable the ones in the order
        if let Some(display_order) = config.custom.get("display_order")
            .and_then(|v| v.as_array()) {
            // Turn off all displays first
            self.show_temperature = false;
            self.show_power = false;
            self.show_utilization = false;
            self.show_memory = false;
            self.show_frequency = false;
            
            // Enable only the ones specified in display_order
            for item in display_order {
                if let Some(field) = item.as_str() {
                    match field {
                        "temperature" => self.show_temperature = true,
                        "power" => self.show_power = true,
                        "utilization" => self.show_utilization = true,
                        "memory" => self.show_memory = true,
                        "frequency" => self.show_frequency = true,
                        _ => {} // Ignore unknown fields
                    }
                }
            }
        }
    }
    /// Create a new AMDGPU sensor with auto-detection.
    pub fn new() -> Result<Self, SensorError> {
        AmdgpuSensorBuilder::auto_detect().build()
    }
    
    /// Create a new AMDGPU sensor with a specific metrics file.
    pub fn with_file<P: AsRef<Path>>(path: P) -> Result<Self, SensorError> {
        AmdgpuSensorBuilder::new(path).build()
    }
    
    /// Read current GPU metrics with error recovery.
    fn read_metrics_with_recovery(&mut self) -> Result<Box<dyn GpuMetrics>, SensorError> {
        match self.metrics_reader.read_file(&self.metrics_path) {
            Ok(metrics) => {
                // Reset error state on success
                self.consecutive_errors = 0;
                self.last_error_time = None;
                
                // Cache metrics for comparison
                self.last_metrics = Some((metrics.clone_box(), Instant::now()));
                
                Ok(metrics)
            },
            Err(e) => {
                self.consecutive_errors += 1;
                self.last_error_time = Some(Instant::now());
                
                // Attempt error recovery
                if self.consecutive_errors <= 3 && self.last_metrics.is_some() {
                    // Use cached metrics for temporary recovery
                    if let Some((cached_metrics, cached_time)) = &self.last_metrics {
                        let age = Instant::now().duration_since(*cached_time);
                        if age < Duration::from_secs(10) {
                            // Return cached metrics with warning
                            return Ok(cached_metrics.clone_box());
                        }
                    }
                }
                
                Err(e)
            }
        }
    }
    
    /// Build comprehensive tooltip with GPU information.
    fn build_tooltip(&self, metrics: &dyn GpuMetrics) -> String {
        let mut tooltip = String::new();
        
        // Device information
        tooltip.push_str(&format!(
            "GPU: {}\\nDevice: {} ({})\\n",
            self.device_info.card_name,
            self.device_info.device_id,
            self.device_info.vendor_id
        ));
        
        // Current metrics
        let (temp, temp_label) = metrics.get_temperature();
        let power = metrics.get_power();
        let activity = metrics.get_activity();
        let frequency = metrics.get_frequency();
        
        tooltip.push_str(&format!(
            "Temperature ({}): {}°C\\nPower: {}W\\nActivity: {}%\\nFrequency: {}MHz\\n",
            temp_label, temp, power, activity, frequency
        ));
        
        // Fan information
        let (fan_speed, has_fan) = metrics.get_fan_speed();
        if has_fan && fan_speed > 0 {
            tooltip.push_str(&format!("Fan Speed: {}%\\n", fan_speed));
        }
        
        // Throttling information
        let throttle_status = metrics.get_throttle_status();
        if throttle_status != 0 {
            tooltip.push_str("\\n⚠️ Throttling Active\\n");
            // Add specific throttle reasons here
        }
        
        // Thermal monitoring data
        if let Some(ref thermal_monitor) = self.thermal_monitor {
            if let Some(thermal_state) = thermal_monitor.get_current_state() {
                tooltip.push_str(&format!(
                    "\\nThermal State: {:?}\\n",
                    thermal_state.alert_level
                ));
            }
        }
        
        // Performance analytics
        if let Some(ref analytics) = self.performance_analytics {
            if let Some(perf_state) = analytics.get_current_state() {
                tooltip.push_str(&format!(
                    "\\nPower Efficiency: {:.1} Perf/W\\n",
                    perf_state.power_efficiency
                ));
                
                if !perf_state.optimization_hints.is_empty() {
                    tooltip.push_str("\\nOptimization Hints:\\n");
                    for hint in &perf_state.optimization_hints {
                        tooltip.push_str(&format!("• {}\\n", hint));
                    }
                }
            }
        }
        
        // Error recovery information
        if self.consecutive_errors > 0 {
            tooltip.push_str(&format!(
                "\\n⚠️ Recent errors: {}\\n",
                self.consecutive_errors
            ));
        }
        
        tooltip.trim_end().to_string()
    }
    
    /// Format output based on the configured output format.
    fn format_output(&self, metrics: &dyn GpuMetrics) -> (String, f64, Option<u8>) {
        let (temp, _) = metrics.get_temperature();
        let power = metrics.get_power();
        let activity = metrics.get_activity();
        let frequency = metrics.get_frequency();
        
        match self.output_format {
            OutputFormat::Compact => {
                // Use display flags to determine what to show
                let mut parts = Vec::new();
                
                if self.show_temperature {
                    parts.push(format!("{:3.0}°C", temp));
                }
                if self.show_power {
                    parts.push(format!("{}W", power));
                }
                if self.show_utilization {
                    parts.push(format!("{:3.0}%", activity));
                }
                if self.show_memory {
                    let (mem_used, mem_total) = metrics.get_memory_usage();
                    if mem_total > 0 {
                        let mem_pct = (mem_used as f64 / mem_total as f64) * 100.0;
                        parts.push(format!("{:1.0}%M", mem_pct));
                    }
                }
                if self.show_frequency {
                    parts.push(format!("{}MHz", frequency));
                }
                
                let text = if parts.is_empty() {
                    format!("{:3.0}%", activity) // Fallback to activity
                } else {
                    parts.join(" ")
                };
                
                (text, temp as f64, Some(activity as u8))
            },
            OutputFormat::Detailed => {
                let (fan_speed, has_fan) = metrics.get_fan_speed();
                let mut parts = vec![
                    format!("{:3.0}°C", temp),
                    format!("{}W", power),
                    format!("{:3.0}%", activity),
                    format!("{}MHz", frequency),
                ];
                
                if has_fan && fan_speed > 0 {
                    parts.push(format!("{:3.0}%", fan_speed));
                }
                
                (parts.join(" "), temp as f64, Some(activity as u8))
            },
            OutputFormat::Minimal => {
                let text = format!("{:3.0}°C", temp);
                (text, temp as f64, None)
            },
            OutputFormat::Power => {
                let text = format!("{}W", power);
                (text, power as f64, Some(((power as f64 / self.power_critical) * 100.0) as u8))
            },
            OutputFormat::Activity => {
                let text = format!("{:3.0}%", activity);
                (text, activity as f64, Some(activity as u8))
            },
            OutputFormat::Thermal => {
                let text = format!("{:3.0}°C", temp);
                let thermal_pct = ((temp as f64 / self.temp_critical) * 100.0) as u8;
                (text, temp as f64, Some(thermal_pct))
            },
            OutputFormat::Performance => {
                let text = format!("{}MHz {}W", frequency, power);
                (text, temp as f64, Some(activity as u8))
            },
            OutputFormat::Custom(ref fields) => {
                let mut parts = Vec::new();
                for field in fields {
                    match field.as_str() {
                        "temp" => parts.push(format!("{:3.0}°C", temp)),
                        "power" => parts.push(format!("{}W", power)),
                        "activity" => parts.push(format!("{:3.0}%", activity)),
                        "frequency" => parts.push(format!("{}MHz", frequency)),
                        "fan" => {
                            let (fan_speed, has_fan) = metrics.get_fan_speed();
                            if has_fan {
                                parts.push(format!("{:3.0}%", fan_speed));
                            }
                        },
                        _ => {} // Ignore unknown fields
                    }
                }
                
                let text = if parts.is_empty() {
                    format!("{:3.0}°C", temp)
                } else {
                    parts.join(" ")
                };
                
                (text, temp as f64, Some(activity as u8))
            },
        }
    }
    
    /// Get the current thermal state.
    pub fn thermal_state(&self) -> Option<ThermalAlert> {
        self.thermal_monitor.as_ref()
            .and_then(|tm| tm.get_current_state())
            .map(|state| state.alert_level)
    }
    
    /// Get the current performance state.
    pub fn performance_state(&self) -> Option<PerformanceState> {
        self.performance_analytics.as_ref()
            .and_then(|pa| pa.get_current_state())
    }
    
    /// Get device information.
    pub fn device_info(&self) -> &GpuDeviceInfo {
        &self.device_info
    }
    
    /// Check if GPU is currently throttling.
    pub fn is_throttling(&self) -> Option<bool> {
        self.last_metrics.as_ref()
            .map(|(metrics, _)| metrics.get_throttle_status() != 0)
    }
    
    /// Force cache invalidation for next read.
    pub fn invalidate_cache(&mut self) {
        self.metrics_reader.invalidate_cache();
        self.last_metrics = None;
    }
}

impl Sensor for AmdgpuSensor {
    type Error = SensorError;
    
    fn read(&mut self) -> Result<WaybarOutput, Self::Error> {
        // Read GPU metrics with error recovery
        let metrics = self.read_metrics_with_recovery()?;
        
        // Update thermal monitoring
        if let Some(ref mut thermal_monitor) = self.thermal_monitor {
            let (temp, _) = metrics.get_temperature();
            thermal_monitor.update(temp as f64)?;
        }
        
        // Update performance analytics
        if let Some(ref mut analytics) = self.performance_analytics {
            analytics.update(
                metrics.get_power() as f64,
                metrics.get_activity() as f64,
                metrics.get_frequency() as f64,
                metrics.get_temperature().0 as f64,
            )?;
        }
        
        // Format output
        let (text, primary_value, percentage) = self.format_output(metrics.as_ref());
        let icon = &self.config.icons.gpu;
        let formatted_text = format::with_icon_and_colors(&text, icon, &self.config);
        
        // Build tooltip
        let tooltip = self.build_tooltip(metrics.as_ref());
        
        // Determine appropriate thresholds based on output format
        let (warning_threshold, critical_threshold) = match self.output_format {
            OutputFormat::Power => (self.power_warning, self.power_critical),
            OutputFormat::Activity => (70.0, 90.0), // Activity percentages
            _ => (self.temp_warning, self.temp_critical), // Temperature by default
        };
        
        Ok(format::themed_output(
            formatted_text,
            Some(tooltip),
            percentage,
            primary_value,
            warning_threshold,
            critical_threshold,
            &self.config.theme,
        ))
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn configure(&mut self, config: SensorConfig) -> Result<(), Self::Error> {
        // Update configuration from custom settings before moving config
        if let Some(temp_warning) = config.get_custom("temp_warning") {
            if let serde_json::Value::Number(temp) = temp_warning {
                if let Some(temp_f64) = temp.as_f64() {
                    self.temp_warning = temp_f64;
                }
            }
        }
        
        if let Some(temp_critical) = config.get_custom("temp_critical") {
            if let serde_json::Value::Number(temp) = temp_critical {
                if let Some(temp_f64) = temp.as_f64() {
                    self.temp_critical = temp_f64;
                }
            }
        }
        
        // Update display configuration
        self.update_display_config(&config);
        
        self.config = config;
        
        // Invalidate cache when configuration changes
        self.invalidate_cache();
        
        Ok(())
    }
    
    fn config(&self) -> &SensorConfig {
        &self.config
    }
    
    fn check_availability(&self) -> Result<(), Self::Error> {
        if !self.metrics_path.exists() {
            return Err(GpuError::MetricsFileError {
                path: self.metrics_path.display().to_string(),
                reason: "GPU metrics file no longer exists".to_string(),
            }.into());
        }
        
        // Test if we can read metrics
        match self.metrics_reader.read_file(&self.metrics_path) {
            Ok(_) => Ok(()),
            Err(e) => Err(GpuError::MetricsFileError {
                path: self.metrics_path.display().to_string(),
                reason: format!("Cannot read GPU metrics: {}", e),
            }.into()),
        }
    }
}

// Implement Clone for GpuMetrics trait objects
trait GpuMetricsClone {
    fn clone_box(&self) -> Box<dyn GpuMetrics>;
}

impl<T> GpuMetricsClone for T
where
    T: 'static + GpuMetrics + Clone,
{
    fn clone_box(&self) -> Box<dyn GpuMetrics> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn GpuMetrics> {
    fn clone(&self) -> Box<dyn GpuMetrics> {
        self.clone_box()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_output_formats() {
        // Test various output format configurations
        let builder = AmdgpuSensorBuilder::auto_detect();
        
        // Test compact format
        let builder_compact = builder.output_format(OutputFormat::Compact);
        assert_eq!(builder_compact.output_format, OutputFormat::Compact);
        
        // Test custom format
        let custom_fields = vec!["temp".to_string(), "power".to_string()];
        let builder_custom = AmdgpuSensorBuilder::auto_detect()
            .output_format(OutputFormat::Custom(custom_fields.clone()));
        
        match builder_custom.output_format {
            OutputFormat::Custom(fields) => {
                assert_eq!(fields, custom_fields);
            },
            _ => panic!("Expected custom format"),
        }
    }
    
    #[test]
    fn test_threshold_validation() {
        // Test invalid temperature thresholds
        let result = AmdgpuSensorBuilder::auto_detect()
            .warning_temperature(90.0)
            .critical_temperature(80.0) // Invalid: critical < warning
            .build();
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("warning threshold"));
    }
    
    #[test]
    fn test_cache_strategies() {
        let strategies = [
            CacheStrategy::None,
            CacheStrategy::Basic { max_age: Duration::from_millis(500) },
            CacheStrategy::Aggressive { 
                max_age: Duration::from_secs(1), 
                change_threshold: 5.0 
            },
            CacheStrategy::MemoryMapped,
        ];
        
        for strategy in &strategies {
            let builder = AmdgpuSensorBuilder::auto_detect()
                .cache_strategy(*strategy);
            assert_eq!(builder.cache_strategy, *strategy);
        }
    }
    
    #[test]
    fn test_thermal_alert_ordering() {
        assert!(ThermalAlert::Normal < ThermalAlert::Warning);
        assert!(ThermalAlert::Warning < ThermalAlert::Critical);
        assert!(ThermalAlert::Critical < ThermalAlert::Emergency);
    }
}