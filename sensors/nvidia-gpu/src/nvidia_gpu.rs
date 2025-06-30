//! NVIDIA GPU monitoring using nvidia-smi parsing.

use waysensor_rs_core::{
    format, Sensor, SensorConfig, SensorError, WaybarOutput,
};
use std::process::Command;
use std::str;

/// NVIDIA GPU sensor that monitors GPU utilization, temperature, memory, and power.
#[derive(Debug)]
pub struct NvidiaGpuSensor {
    name: String,
    config: SensorConfig,
    warning_threshold: f64,
    critical_threshold: f64,
    gpu_id: Option<u32>,
    utilization_history: Vec<f64>,
    temperature_history: Vec<f64>,
    memory_usage_history: Vec<f64>,
}

/// NVIDIA GPU metrics parsed from nvidia-smi output.
#[derive(Debug, Clone, PartialEq)]
pub struct NvidiaGpuMetrics {
    /// GPU utilization percentage (0-100)
    pub utilization_gpu: f64,
    /// GPU temperature in Celsius
    pub temperature: f64,
    /// Memory usage in MB
    pub memory_used: u64,
    /// Total memory in MB
    pub memory_total: u64,
    /// Power draw in Watts
    pub power_draw: Option<f64>,
    /// GPU name/model
    pub name: String,
    /// Driver version
    pub driver_version: String,
    /// GPU clock in MHz
    pub gpu_clock: Option<u32>,
    /// Memory clock in MHz
    pub memory_clock: Option<u32>,
}

impl NvidiaGpuMetrics {
    /// Calculate memory usage percentage.
    pub fn memory_usage_percent(&self) -> f64 {
        if self.memory_total > 0 {
            (self.memory_used as f64 / self.memory_total as f64) * 100.0
        } else {
            0.0
        }
    }
}

impl NvidiaGpuSensor {
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

    /// Create a new NVIDIA GPU sensor.
    pub fn new(warning_threshold: u8, critical_threshold: u8) -> Result<Self, SensorError> {
        if critical_threshold <= warning_threshold {
            return Err(SensorError::config(format!(
                "Critical threshold ({}) must be greater than warning threshold ({})",
                critical_threshold, warning_threshold
            )));
        }

        Ok(Self {
            name: "nvidia-gpu".to_owned(),
            config: SensorConfig::default(),
            warning_threshold: f64::from(warning_threshold),
            critical_threshold: f64::from(critical_threshold),
            gpu_id: None,
            utilization_history: Vec::new(),
            temperature_history: Vec::new(),
            memory_usage_history: Vec::new(),
        })
    }

    /// Create a new NVIDIA GPU sensor for a specific GPU ID.
    pub fn new_with_gpu_id(
        warning_threshold: u8,
        critical_threshold: u8,
        gpu_id: u32,
    ) -> Result<Self, SensorError> {
        let mut sensor = Self::new(warning_threshold, critical_threshold)?;
        sensor.gpu_id = Some(gpu_id);
        sensor.name = format!("nvidia-gpu-{}", gpu_id);
        Ok(sensor)
    }

    /// Create a new NVIDIA GPU sensor with default thresholds (80% warning, 95% critical).
    pub fn with_defaults() -> Result<Self, SensorError> {
        Self::new(80, 95)
    }

    /// Parse nvidia-smi output to extract GPU metrics.
    fn parse_nvidia_smi_output(output: &str) -> Result<NvidiaGpuMetrics, SensorError> {
        // Parse nvidia-smi CSV output
        // Expected format: name, driver_version, temperature.gpu, utilization.gpu,
        // memory.used, memory.total, power.draw, clocks.current.graphics, clocks.current.memory
        
        let lines: Vec<&str> = output.trim().lines().collect();
        if lines.len() < 2 {
            return Err(SensorError::parse("Invalid nvidia-smi output format"));
        }

        let data_line = lines[1]; // Skip header
        let fields: Vec<&str> = data_line.split(", ").collect();

        if fields.len() < 6 {
            return Err(SensorError::parse(format!(
                "Insufficient nvidia-smi data fields: expected at least 6, got {}",
                fields.len()
            )));
        }

        let name = fields[0].trim().to_string();
        let driver_version = fields[1].trim().to_string();
        
        let temperature = fields[2].trim()
            .parse::<f64>()
            .map_err(|e| SensorError::parse_with_source("Failed to parse temperature", e))?;

        let utilization_gpu = fields[3].trim()
            .parse::<f64>()
            .map_err(|e| SensorError::parse_with_source("Failed to parse GPU utilization", e))?;

        let memory_used = fields[4].trim()
            .split_whitespace()
            .next()
            .unwrap_or("0")
            .parse::<u64>()
            .map_err(|e| SensorError::parse_with_source("Failed to parse memory used", e))?;

        let memory_total = fields[5].trim()
            .split_whitespace()
            .next()
            .unwrap_or("0")
            .parse::<u64>()
            .map_err(|e| SensorError::parse_with_source("Failed to parse memory total", e))?;

        let power_draw = if fields.len() > 6 {
            fields[6].trim()
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<f64>().ok())
        } else {
            None
        };

        let gpu_clock = if fields.len() > 7 {
            fields[7].trim()
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u32>().ok())
        } else {
            None
        };

        let memory_clock = if fields.len() > 8 {
            fields[8].trim()
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u32>().ok())
        } else {
            None
        };

        Ok(NvidiaGpuMetrics {
            utilization_gpu,
            temperature,
            memory_used,
            memory_total,
            power_draw,
            name,
            driver_version,
            gpu_clock,
            memory_clock,
        })
    }

    /// Query NVIDIA GPU metrics using nvidia-smi.
    fn query_gpu_metrics(&self) -> Result<NvidiaGpuMetrics, SensorError> {
        let mut cmd = Command::new("nvidia-smi");
        
        // CSV format with specific fields
        cmd.arg("--query-gpu=name,driver_version,temperature.gpu,utilization.gpu,memory.used,memory.total,power.draw,clocks.current.graphics,clocks.current.memory")
           .arg("--format=csv,noheader,nounits");

        if let Some(gpu_id) = self.gpu_id {
            cmd.arg(format!("--id={}", gpu_id));
        }

        let output = cmd.output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SensorError::unavailable("nvidia-smi command not found. Please install NVIDIA drivers.")
                } else {
                    SensorError::Io(e)
                }
            })?;

        if !output.status.success() {
            let stderr = str::from_utf8(&output.stderr).unwrap_or("Unknown error");
            return Err(SensorError::unavailable(format!(
                "nvidia-smi failed: {}", stderr
            )));
        }

        let stdout = str::from_utf8(&output.stdout)
            .map_err(|e| SensorError::parse_with_source("Invalid UTF-8 in nvidia-smi output", e))?;

        Self::parse_nvidia_smi_output(stdout)
    }

    /// Update history for sparklines.
    fn update_history(&mut self, metrics: &NvidiaGpuMetrics) {
        let max_len = self.config.visuals.sparkline_length;

        // Update utilization history
        self.utilization_history.push(metrics.utilization_gpu);
        if self.utilization_history.len() > max_len {
            self.utilization_history.remove(0);
        }

        // Update temperature history
        self.temperature_history.push(metrics.temperature);
        if self.temperature_history.len() > max_len {
            self.temperature_history.remove(0);
        }

        // Update memory usage history
        self.memory_usage_history.push(metrics.memory_usage_percent());
        if self.memory_usage_history.len() > max_len {
            self.memory_usage_history.remove(0);
        }
    }

    /// Create formatted tooltip with GPU information.
    fn create_tooltip(&self, metrics: &NvidiaGpuMetrics) -> String {
        use waysensor_rs_core::format;

        let mut lines = Vec::new();

        // Basic GPU info
        lines.push(format::key_value("GPU", &metrics.name, &self.config));
        lines.push(format::key_value("Driver", &metrics.driver_version, &self.config));

        // Usage metrics with gauges
        let gpu_gauge = Self::create_gauge(metrics.utilization_gpu, 12);
        let gpu_indicator = Self::get_usage_indicator(metrics.utilization_gpu);
        lines.push(format::key_value(
            "GPU Usage",
            &format!("{} {:.1}% {}", gpu_gauge, metrics.utilization_gpu, gpu_indicator),
            &self.config,
        ));

        let temp_percentage = ((metrics.temperature / 100.0) * 100.0).min(100.0);
        let temp_gauge = Self::create_gauge(temp_percentage, 12);
        let temp_indicator = Self::get_usage_indicator(temp_percentage);
        lines.push(format::key_value(
            "Temperature",
            &format!("{} {:.0}°C {}", temp_gauge, metrics.temperature, temp_indicator),
            &self.config,
        ));

        let memory_percent = metrics.memory_usage_percent();
        let memory_gauge = Self::create_gauge(memory_percent, 12);
        let memory_indicator = Self::get_usage_indicator(memory_percent);
        lines.push(format::key_value(
            "Memory Usage",
            &format!("{} {:.1}% ({} / {} MB) {}",
                memory_gauge, memory_percent, metrics.memory_used, metrics.memory_total, memory_indicator
            ),
            &self.config,
        ));

        // Optional metrics with gauges
        if let Some(power) = metrics.power_draw {
            let power_percentage = ((power / 400.0) * 100.0).min(100.0); // Assume 400W max for NVIDIA GPU
            let power_gauge = Self::create_gauge(power_percentage, 12);
            let power_indicator = Self::get_usage_indicator(power_percentage);
            lines.push(format::key_value(
                "Power Draw",
                &format!("{} {:.1}W {}", power_gauge, power, power_indicator),
                &self.config,
            ));
        }

        if let Some(gpu_clock) = metrics.gpu_clock {
            lines.push(format::key_value(
                "GPU Clock",
                &format!("{}MHz", gpu_clock),
                &self.config,
            ));
        }

        if let Some(memory_clock) = metrics.memory_clock {
            lines.push(format::key_value(
                "Memory Clock",
                &format!("{}MHz", memory_clock),
                &self.config,
            ));
        }

        // Add sparklines if enabled and we have history
        if self.config.visuals.sparklines && self.config.visuals.extended_metadata {
            if self.utilization_history.len() > 1 {
                let sparkline = format::create_sparkline(&self.utilization_history, self.config.visuals.sparkline_style);
                if !sparkline.is_empty() {
                    lines.push("".to_string()); // Empty line separator
                    lines.push(format::key_value(
                        "Usage History",
                        &format::colored_sparkline(&sparkline, self.config.sparkline_color.as_deref()),
                        &self.config,
                    ));
                }
            }

            if self.temperature_history.len() > 1 {
                let sparkline = format::create_sparkline(&self.temperature_history, self.config.visuals.sparkline_style);
                if !sparkline.is_empty() {
                    lines.push(format::key_value(
                        "Temp History",
                        &format::colored_sparkline(&sparkline, self.config.sparkline_color.as_deref()),
                        &self.config,
                    ));
                }
            }
        }

        lines.join("\n")
    }
}

impl Sensor for NvidiaGpuSensor {
    type Error = SensorError;

    fn read(&mut self) -> Result<WaybarOutput, Self::Error> {
        let metrics = self.query_gpu_metrics()?;
        
        // Update history for sparklines
        self.update_history(&metrics);

        // Build the main text with optional sparkline and status indicator
        let icon = &self.config.icons.gpu;
        let mut text_parts = Vec::new();

        // Add sparkline if enabled and we have history and should show in text
        if self.config.visuals.sparklines && self.config.visuals.sparklines_in_text && self.utilization_history.len() > 1 {
            let sparkline = format::create_sparkline(&self.utilization_history, self.config.visuals.sparkline_style);
            if !sparkline.is_empty() {
                let colored_sparkline = format::colored_sparkline(&sparkline, self.config.sparkline_color.as_deref());
                text_parts.push(colored_sparkline);
            }
        }

        // Add main utilization percentage
        text_parts.push(format!("{:3.0}%", metrics.utilization_gpu));


        // Add status indicator if enabled (based on utilization)
        if self.config.visuals.status_indicators {
            let status = format::status_indicator(
                metrics.utilization_gpu,
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
        let percentage = metrics.utilization_gpu.round().clamp(0.0, 100.0) as u8;

        Ok(format::themed_output(
            text,
            Some(tooltip),
            Some(percentage),
            metrics.utilization_gpu,
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
        // Try to run nvidia-smi to check if it's available
        let output = Command::new("nvidia-smi")
            .arg("--help")
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SensorError::unavailable("nvidia-smi command not found. Please install NVIDIA drivers.")
                } else {
                    SensorError::Io(e)
                }
            })?;

        if !output.status.success() {
            return Err(SensorError::unavailable("nvidia-smi is not working properly"));
        }

        // Try to query GPU information
        self.query_gpu_metrics().map(|_| ())
    }
}