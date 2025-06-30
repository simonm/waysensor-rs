//! Intel GPU monitoring using sysfs and DRM interfaces.

use waysensor_rs_core::{
    format, Sensor, SensorConfig, SensorError, WaybarOutput,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Intel GPU sensor that monitors GPU frequency, power, and utilization.
#[derive(Debug)]
pub struct IntelGpuSensor {
    name: String,
    config: SensorConfig,
    warning_threshold: f64,
    critical_threshold: f64,
    card_path: PathBuf,
    gt_path: Option<PathBuf>,
    frequency_history: Vec<f64>,
    utilization_history: Vec<f64>,
}

/// Intel GPU metrics from sysfs.
#[derive(Debug, Clone, PartialEq)]
pub struct IntelGpuMetrics {
    /// Current GPU frequency in MHz
    pub current_freq_mhz: Option<u32>,
    /// Maximum GPU frequency in MHz
    pub max_freq_mhz: Option<u32>,
    /// Minimum GPU frequency in MHz
    pub min_freq_mhz: Option<u32>,
    /// GPU frequency as percentage of max
    pub frequency_percent: f64,
    /// Power consumption (if available)
    pub power_watts: Option<f64>,
    /// GPU name/model
    pub name: String,
    /// Driver name
    pub driver: String,
}

impl IntelGpuSensor {
    /// Create a visual bar gauge for a percentage value.
    fn create_gauge(percentage: f64, width: usize) -> String {
        let filled = ((percentage / 100.0) * width as f64).round() as usize;
        let empty = width.saturating_sub(filled);
        
        let filled_char = '█';
        let empty_char = '░';
        
        format!("{}{}", 
            filled_char.to_string().repeat(filled),
            empty_char.to_string().repeat(empty)
        )
    }
    
    /// Get a color indicator based on usage percentage.
    fn get_usage_indicator(percentage: f64) -> &'static str {
        match percentage {
            p if p >= 90.0 => "🔴",
            p if p >= 70.0 => "🟠",
            p if p >= 50.0 => "🟡",
            p if p >= 25.0 => "🟢",
            _ => "⚪",
        }
    }

    /// Create a new Intel GPU sensor.
    pub fn new(warning_threshold: u8, critical_threshold: u8) -> Result<Self, SensorError> {
        if critical_threshold <= warning_threshold {
            return Err(SensorError::config(format!(
                "Critical threshold ({}) must be greater than warning threshold ({})",
                critical_threshold, warning_threshold
            )));
        }

        // Find Intel GPU card
        let card_path = Self::find_intel_gpu_card()?;
        let gt_path = Self::find_gt_path(&card_path);

        Ok(Self {
            name: "intel-gpu".to_owned(),
            config: SensorConfig::default(),
            warning_threshold: f64::from(warning_threshold),
            critical_threshold: f64::from(critical_threshold),
            card_path,
            gt_path,
            frequency_history: Vec::new(),
            utilization_history: Vec::new(),
        })
    }

    /// Create a new Intel GPU sensor with default thresholds (80% warning, 95% critical).
    pub fn with_defaults() -> Result<Self, SensorError> {
        Self::new(80, 95)
    }

    /// Find Intel GPU card in /sys/class/drm/.
    fn find_intel_gpu_card() -> Result<PathBuf, SensorError> {
        let drm_path = Path::new("/sys/class/drm");
        
        if !drm_path.exists() {
            return Err(SensorError::unavailable(
                "DRM subsystem not available (/sys/class/drm not found)"
            ));
        }

        // Look for card* directories
        let entries = fs::read_dir(drm_path)
            .map_err(|e| SensorError::Io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| SensorError::Io(e))?;
            let path = entry.path();
            
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("card") && !name.contains('-') {
                    // Check if this is an Intel GPU
                    if let Ok(driver) = fs::read_to_string(path.join("device/driver/module/srcversion")) {
                        if driver.contains("i915") || driver.contains("xe") {
                            return Ok(path);
                        }
                    }
                    
                    // Alternative: check uevent file
                    if let Ok(uevent) = fs::read_to_string(path.join("device/uevent")) {
                        if uevent.contains("PCI_ID=8086:") {  // Intel PCI vendor ID
                            return Ok(path);
                        }
                    }
                }
            }
        }

        Err(SensorError::unavailable("No Intel GPU found"))
    }

    /// Find GT (Graphics Technology) path for frequency monitoring.
    fn find_gt_path(card_path: &Path) -> Option<PathBuf> {
        // Try common GT paths
        let gt_candidates = ["gt", "gt0", "gt/gt0"];
        
        for candidate in &gt_candidates {
            let gt_path = card_path.join(candidate);
            if gt_path.exists() {
                return Some(gt_path);
            }
        }
        
        None
    }

    /// Read frequency from sysfs file.
    fn read_frequency_mhz(path: &Path) -> Result<u32, SensorError> {
        let content = fs::read_to_string(path)
            .map_err(|e| SensorError::Io(e))?;
        
        let freq = content.trim().parse::<u32>()
            .map_err(|e| SensorError::parse_with_source("Failed to parse frequency", e))?;
        
        Ok(freq)
    }

    /// Read GPU name from sysfs.
    fn read_gpu_name(card_path: &Path) -> String {
        // Try multiple sources for GPU name
        let name_paths = [
            "device/modalias",
            "device/subsystem_device",
            "device/device",
        ];

        for path in &name_paths {
            if let Ok(content) = fs::read_to_string(card_path.join(path)) {
                if !content.trim().is_empty() {
                    return format!("Intel GPU ({})", content.trim());
                }
            }
        }

        "Intel GPU".to_string()
    }

    /// Read driver name from sysfs.
    fn read_driver_name(card_path: &Path) -> String {
        if let Ok(content) = fs::read_to_string(card_path.join("device/driver_override")) {
            return content.trim().to_string();
        }

        if let Ok(link) = fs::read_link(card_path.join("device/driver")) {
            if let Some(name) = link.file_name().and_then(|n| n.to_str()) {
                return name.to_string();
            }
        }

        "i915".to_string()
    }

    /// Query Intel GPU metrics from sysfs.
    fn query_gpu_metrics(&self) -> Result<IntelGpuMetrics, SensorError> {
        let name = Self::read_gpu_name(&self.card_path);
        let driver = Self::read_driver_name(&self.card_path);

        let (current_freq_mhz, max_freq_mhz, min_freq_mhz, frequency_percent) = 
            if let Some(ref gt_path) = self.gt_path {
                // Try to read frequencies from GT path
                let current_freq = Self::read_frequency_mhz(&gt_path.join("rps_cur_freq_mhz")).ok();
                let max_freq = Self::read_frequency_mhz(&gt_path.join("rps_max_freq_mhz")).ok();
                let min_freq = Self::read_frequency_mhz(&gt_path.join("rps_min_freq_mhz")).ok();

                let frequency_percent = if let (Some(current), Some(max), Some(min)) = (current_freq, max_freq, min_freq) {
                    if max > min {
                        ((current - min) as f64 / (max - min) as f64) * 100.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                (current_freq, max_freq, min_freq, frequency_percent)
            } else {
                (None, None, None, 0.0)
            };

        // Power consumption is harder to get on Intel - would need PMT or other interfaces
        let power_watts = None;

        Ok(IntelGpuMetrics {
            current_freq_mhz,
            max_freq_mhz,
            min_freq_mhz,
            frequency_percent,
            power_watts,
            name,
            driver,
        })
    }

    /// Update history for sparklines.
    fn update_history(&mut self, metrics: &IntelGpuMetrics) {
        let max_len = self.config.visuals.sparkline_length;

        // Update frequency history
        self.frequency_history.push(metrics.frequency_percent);
        if self.frequency_history.len() > max_len {
            self.frequency_history.remove(0);
        }

        // For now, use frequency as utilization proxy
        self.utilization_history.push(metrics.frequency_percent);
        if self.utilization_history.len() > max_len {
            self.utilization_history.remove(0);
        }
    }

    /// Create formatted tooltip with GPU information.
    fn create_tooltip(&self, metrics: &IntelGpuMetrics) -> String {
        use waysensor_rs_core::format;

        let mut lines = Vec::new();

        // Basic GPU info
        lines.push(format::key_value("GPU", &metrics.name, &self.config));
        lines.push(format::key_value("Driver", &metrics.driver, &self.config));

        // Frequency information
        if let Some(current_freq) = metrics.current_freq_mhz {
            lines.push(format::key_value(
                "Current Frequency",
                &format!("{}MHz", current_freq),
                &self.config,
            ));
        }

        if let Some(max_freq) = metrics.max_freq_mhz {
            lines.push(format::key_value(
                "Max Frequency",
                &format!("{}MHz", max_freq),
                &self.config,
            ));
        }

        if let Some(min_freq) = metrics.min_freq_mhz {
            lines.push(format::key_value(
                "Min Frequency",
                &format!("{}MHz", min_freq),
                &self.config,
            ));
        }

        // Frequency usage with gauge
        let freq_gauge = Self::create_gauge(metrics.frequency_percent, 12);
        let freq_indicator = Self::get_usage_indicator(metrics.frequency_percent);
        lines.push(format::key_value(
            "Frequency Usage",
            &format!("{} {:.1}% {}", freq_gauge, metrics.frequency_percent, freq_indicator),
            &self.config,
        ));

        // Optional power information with gauge
        if let Some(power) = metrics.power_watts {
            let power_percentage = ((power / 150.0) * 100.0).min(100.0); // Assume 150W max for Intel GPU
            let power_gauge = Self::create_gauge(power_percentage, 12);
            let power_indicator = Self::get_usage_indicator(power_percentage);
            lines.push(format::key_value(
                "Power",
                &format!("{} {:.1}W {}", power_gauge, power, power_indicator),
                &self.config,
            ));
        }

        // Add sparklines if enabled and we have history
        if self.config.visuals.sparklines && self.config.visuals.extended_metadata {
            if self.frequency_history.len() > 1 {
                let sparkline = format::create_sparkline(&self.frequency_history, self.config.visuals.sparkline_style);
                if !sparkline.is_empty() {
                    lines.push("".to_string()); // Empty line separator
                    lines.push(format::key_value(
                        "Freq History",
                        &format::colored_sparkline(&sparkline, self.config.sparkline_color.as_deref()),
                        &self.config,
                    ));
                }
            }
        }

        lines.join("\n")
    }
}

impl Sensor for IntelGpuSensor {
    type Error = SensorError;

    fn read(&mut self) -> Result<WaybarOutput, Self::Error> {
        let metrics = self.query_gpu_metrics()?;
        
        // Update history for sparklines
        self.update_history(&metrics);

        // Build the main text with optional sparkline and status indicator
        let icon = &self.config.icons.gpu;
        let mut text_parts = Vec::new();

        // Add sparkline if enabled and we have history and should show in text
        if self.config.visuals.sparklines && self.config.visuals.sparklines_in_text && self.frequency_history.len() > 1 {
            let sparkline = format::create_sparkline(&self.frequency_history, self.config.visuals.sparkline_style);
            if !sparkline.is_empty() {
                let colored_sparkline = format::colored_sparkline(&sparkline, self.config.sparkline_color.as_deref());
                text_parts.push(colored_sparkline);
            }
        }

        // Add main frequency percentage (as utilization proxy)
        text_parts.push(format!("{:3.0}%", metrics.frequency_percent));


        // Add status indicator if enabled (based on frequency usage)
        if self.config.visuals.status_indicators {
            let status = format::status_indicator(
                metrics.frequency_percent,
                self.warning_threshold,
                self.critical_threshold,
                self.config.visuals.status_indicators,
            );
            if let Some(indicator) = status {
                text_parts.push(indicator.to_string());
            }
        }

        let combined_text = text_parts.join(" ");
        let text = format::with_icon_and_colors(&combined_text, icon, &self.config);

        let tooltip = self.create_tooltip(&metrics);
        let percentage = metrics.frequency_percent.round().clamp(0.0, 100.0) as u8;

        Ok(format::themed_output(
            text,
            Some(tooltip),
            Some(percentage),
            metrics.frequency_percent,
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
        // Check if card path exists
        if !self.card_path.exists() {
            return Err(SensorError::unavailable("Intel GPU card path not found"));
        }

        // Try to read some basic information
        Self::read_gpu_name(&self.card_path);
        
        Ok(())
    }
}