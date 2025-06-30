//! # waysensor-rs-core
//!
//! Core library for the waysensor-rs sensor suite providing shared functionality
//! for system monitoring in Waybar-compatible format.
//!
//! ## Features
//!
//! - **Consistent theming system** - Unified styling across all sensors
//! - **Waybar JSON output format** - Native Waybar protocol support
//! - **Common sensor traits** - Standardized sensor interface
//! - **Configuration management** - JSONC-based configuration with validation
//! - **Icon system** - User-customizable Nerd Font icons via configuration
//! - **Error handling** - Comprehensive error types with context
//!
//! ## Quick Start
//!
//! ```rust
//! use waysensor_rs_core::{Sensor, SensorConfig, WaybarOutput, IconStyle};
//!
//! // Implement the Sensor trait for your custom sensor
//! struct MySensor {
//!     name: String,
//!     config: SensorConfig,
//! }
//!
//! impl Sensor for MySensor {
//!     type Error = waysensor_rs_core::SensorError;
//!
//!     fn read(&mut self) -> Result<WaybarOutput, Self::Error> {
//!         // Your sensor implementation
//!         todo!()
//!     }
//!
//!     fn name(&self) -> &str {
//!         &self.name
//!     }
//!
//!     fn configure(&mut self, config: SensorConfig) -> Result<(), Self::Error> {
//!         self.config = config;
//!         Ok(())
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// Standard Waybar output format compliant with Waybar's JSON protocol.
///
/// This structure represents the JSON output that Waybar expects from custom modules.
/// All fields except `text` are optional and will be omitted from JSON serialization
/// if they are `None`.
///
/// # Examples
///
/// ```rust
/// use waysensor_rs_core::WaybarOutput;
///
/// let output = WaybarOutput::new("50%".to_string())
///     .with_tooltip("CPU Usage: 50%")
///     .with_class("normal")
///     .with_percentage(50);
/// ```
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WaybarOutput {
    /// The main text to display in the bar
    pub text: String,
    /// Optional tooltip text shown on hover
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    /// Optional CSS class for styling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    /// Optional percentage value (0-100) for progress indicators
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<u8>,
}

impl WaybarOutput {
    /// Create a new WaybarOutput with just the required text field.
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self {
            text,
            tooltip: None,
            class: None,
            percentage: None,
        }
    }

    /// Create a new WaybarOutput from a string literal.
    #[must_use]
    pub fn from_str(text: &str) -> Self {
        Self::new(text.to_owned())
    }

    /// Add a tooltip to this output.
    #[must_use]
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Add a CSS class to this output.
    #[must_use]
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    /// Add a percentage value to this output.
    ///
    /// # Panics
    ///
    /// Panics if `percentage` is greater than 100.
    #[must_use]
    pub fn with_percentage(mut self, percentage: u8) -> Self {
        assert!(
            percentage <= 100,
            "Percentage must be <= 100, got {}",
            percentage
        );
        self.percentage = Some(percentage);
        self
    }

    /// Set the tooltip on this output (mutable version).
    pub fn set_tooltip(&mut self, tooltip: impl Into<String>) {
        self.tooltip = Some(tooltip.into());
    }

    /// Set the CSS class on this output (mutable version).
    pub fn set_class(&mut self, class: impl Into<String>) {
        self.class = Some(class.into());
    }

    /// Set the percentage on this output (mutable version).
    ///
    /// # Panics
    ///
    /// Panics if `percentage` is greater than 100.
    pub fn set_percentage(&mut self, percentage: u8) {
        assert!(
            percentage <= 100,
            "Percentage must be <= 100, got {}",
            percentage
        );
        self.percentage = Some(percentage);
    }
}

/// Global configuration loaded from ~/.config/waysensor-rs/config.ron
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct GlobalConfig {
    /// Default color settings
    #[serde(default)]
    pub colors: ColorConfig,
    /// Default icon style
    #[serde(default)]
    pub icon_style: IconStyle,
    /// Icon position (before or after the value)
    #[serde(default)]
    pub icon_position: IconPosition,
    /// Number of spaces between icon and text
    #[serde(default = "default_icon_spacing")]
    pub icon_spacing: u8,
    /// Icon definitions for different sensor types
    #[serde(default)]
    pub icons: IconConfig,
    /// Update interval in milliseconds
    #[serde(default = "default_update_interval")]
    pub update_interval: u64,
    /// Visual enhancement settings
    #[serde(default)]
    pub visuals: VisualConfig,
    /// Sensor-specific configurations
    #[serde(default)]
    pub sensors: HashMap<String, serde_json::Value>,
}

/// Icon configuration for different sensor types
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct IconConfig {
    /// CPU sensor icon
    #[serde(default = "default_cpu_icon")]
    pub cpu: String,
    /// Memory sensor icon
    #[serde(default = "default_memory_icon")]
    pub memory: String,
    /// Disk/Storage sensor icon
    #[serde(default = "default_disk_icon")]
    pub disk: String,
    /// Network download icon
    #[serde(default = "default_network_download_icon")]
    pub network_download: String,
    /// Network upload icon
    #[serde(default = "default_network_upload_icon")]
    pub network_upload: String,
    /// Network WiFi icon
    #[serde(default = "default_network_wifi_icon")]
    pub network_wifi: String,
    /// Network Ethernet icon
    #[serde(default = "default_network_ethernet_icon")]
    pub network_ethernet: String,
    /// Battery full icon
    #[serde(default = "default_battery_full_icon")]
    pub battery_full: String,
    /// Battery three quarters icon
    #[serde(default = "default_battery_three_quarters_icon")]
    pub battery_three_quarters: String,
    /// Battery half icon
    #[serde(default = "default_battery_half_icon")]
    pub battery_half: String,
    /// Battery quarter icon
    #[serde(default = "default_battery_quarter_icon")]
    pub battery_quarter: String,
    /// Battery empty icon
    #[serde(default = "default_battery_empty_icon")]
    pub battery_empty: String,
    /// Battery charging icon
    #[serde(default = "default_battery_charging_icon")]
    pub battery_charging: String,
    /// Thermal low temperature icon
    #[serde(default = "default_thermal_low_icon")]
    pub thermal_low: String,
    /// Thermal medium temperature icon
    #[serde(default = "default_thermal_medium_icon")]
    pub thermal_medium: String,
    /// Thermal high temperature icon
    #[serde(default = "default_thermal_high_icon")]
    pub thermal_high: String,
    /// GPU sensor icon
    #[serde(default = "default_gpu_icon")]
    pub gpu: String,
}

impl Default for IconConfig {
    fn default() -> Self {
        Self {
            cpu: default_cpu_icon(),
            memory: default_memory_icon(),
            disk: default_disk_icon(),
            network_download: default_network_download_icon(),
            network_upload: default_network_upload_icon(),
            network_wifi: default_network_wifi_icon(),
            network_ethernet: default_network_ethernet_icon(),
            battery_full: default_battery_full_icon(),
            battery_three_quarters: default_battery_three_quarters_icon(),
            battery_half: default_battery_half_icon(),
            battery_quarter: default_battery_quarter_icon(),
            battery_empty: default_battery_empty_icon(),
            battery_charging: default_battery_charging_icon(),
            thermal_low: default_thermal_low_icon(),
            thermal_medium: default_thermal_medium_icon(),
            thermal_high: default_thermal_high_icon(),
            gpu: default_gpu_icon(),
        }
    }
}

// Default icon functions
fn default_cpu_icon() -> String {
    "\u{f4bc}".to_string()
} // 󰍛
fn default_memory_icon() -> String {
    "\u{efc5}".to_string()
} //
fn default_disk_icon() -> String {
    "\u{f0a0}".to_string()
} //
fn default_network_download_icon() -> String {
    "\u{f019}".to_string()
} //
fn default_network_upload_icon() -> String {
    "\u{f093}".to_string()
} //
fn default_network_wifi_icon() -> String {
    "\u{f05a9}".to_string()
} //
fn default_network_ethernet_icon() -> String {
    "\u{ef44}".to_string()
} //
fn default_battery_full_icon() -> String {
    "\u{f0079}".to_string()
} //
fn default_battery_three_quarters_icon() -> String {
    "\u{f12a3}".to_string()
} //
fn default_battery_half_icon() -> String {
    "\u{f12a2}".to_string()
} //
fn default_battery_quarter_icon() -> String {
    "\u{f12a1}".to_string()
} //
fn default_battery_empty_icon() -> String {
    "\u{f008e}".to_string()
} //
fn default_battery_charging_icon() -> String {
    "\u{f0084}".to_string()
} //
fn default_thermal_low_icon() -> String {
    "\u{f2ca}".to_string()
} //
fn default_thermal_medium_icon() -> String {
    "\u{f2c9}".to_string()
} //
fn default_thermal_high_icon() -> String {
    "\u{fc27}".to_string()
} //
fn default_gpu_icon() -> String {
    "\u{f08ae}".to_string()
} //

/// Color configuration for waysensor-rs
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ColorConfig {
    /// Icon color (hex format like "#7aa2f7")
    pub icon_color: Option<String>,
    /// Text color (hex format like "#c0caf5")
    pub text_color: Option<String>,
    /// Tooltip label color (hex format like "#bb9af7")
    pub tooltip_label_color: Option<String>,
    /// Tooltip value color (hex format like "#9ece6a")
    pub tooltip_value_color: Option<String>,
    /// Sparkline color (hex format like "#f7768e")
    pub sparkline_color: Option<String>,
    /// Status indicator colors for different states
    pub status_colors: StatusColorConfig,
}

/// Status indicator color configuration
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StatusColorConfig {
    /// Excellent/good status color
    pub excellent: Option<String>,
    /// Good status color
    pub good: Option<String>,
    /// Warning status color
    pub warning: Option<String>,
    /// Critical status color
    pub critical: Option<String>,
    /// Unknown/unavailable status color
    pub unknown: Option<String>,
}

impl Default for StatusColorConfig {
    fn default() -> Self {
        Self {
            excellent: None,
            good: None,
            warning: None,
            critical: None,
            unknown: None,
        }
    }
}

/// Visual enhancement configuration
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct VisualConfig {
    /// Enable sparklines/mini-charts
    #[serde(default = "default_true")]
    pub sparklines: bool,
    /// Sparkline length (number of data points)
    #[serde(default = "default_sparkline_length")]
    pub sparkline_length: usize,
    /// Sparkline style (blocks, braille, dots)
    #[serde(default)]
    pub sparkline_style: SparklineStyle,
    /// Show sparklines in main text (true) or tooltip only (false)
    #[serde(default = "default_true")]
    pub sparklines_in_text: bool,
    /// Enable status indicators (emoji/symbols)
    #[serde(default = "default_true")]
    pub status_indicators: bool,
    /// Enable additional metadata display
    #[serde(default = "default_true")]
    pub extended_metadata: bool,
    /// Tooltip detail level (basic, detailed, expert)
    #[serde(default)]
    pub tooltip_detail: TooltipDetail,
    /// Enable gauge bars in tooltips
    #[serde(default = "default_true")]
    pub tooltip_gauges: bool,
    /// Width of gauge bars in characters
    #[serde(default = "default_gauge_width")]
    pub gauge_width: usize,
    /// Style of gauge bars
    #[serde(default)]
    pub gauge_style: GaugeStyle,
    /// Show top processes in tooltips
    #[serde(default = "default_true")]
    pub show_top_processes: bool,
    /// Number of top processes to show (1-20)
    #[serde(default = "default_top_processes_count")]
    pub top_processes_count: u8,
    /// Maximum length for process names (truncated if longer)
    #[serde(default = "default_process_name_length")]
    pub process_name_max_length: u8,
}

impl Default for VisualConfig {
    fn default() -> Self {
        Self {
            sparklines: true,
            sparkline_length: default_sparkline_length(),
            sparkline_style: SparklineStyle::default(),
            sparklines_in_text: true,
            status_indicators: true,
            extended_metadata: true,
            tooltip_detail: TooltipDetail::default(),
            tooltip_gauges: true,
            gauge_width: default_gauge_width(),
            gauge_style: GaugeStyle::default(),
            show_top_processes: true,
            top_processes_count: default_top_processes_count(),
            process_name_max_length: default_process_name_length(),
        }
    }
}

/// Sparkline rendering style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SparklineStyle {
    /// Unicode block characters (▁▂▃▄▅▆▇█)
    Blocks,
    /// Braille patterns for higher density
    Braille,
    /// Simple dots and dashes
    Dots,
    /// Disabled
    None,
}

/// Gauge bar rendering style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GaugeStyle {
    /// Unicode block characters (█░)
    Blocks,
    /// ASCII characters ([#-])
    Ascii,
    /// Dots and spaces (●○)
    Dots,
    /// Equal signs ([==  ])
    Equals,
    /// Custom characters (requires gauge_chars config)
    Custom,
}

impl Default for SparklineStyle {
    fn default() -> Self {
        Self::Blocks
    }
}

impl Default for GaugeStyle {
    fn default() -> Self {
        Self::Blocks
    }
}

/// Tooltip detail level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TooltipDetail {
    /// Basic information only
    Basic,
    /// Standard detailed information
    Detailed,
    /// Expert-level comprehensive information
    Expert,
}

impl Default for TooltipDetail {
    fn default() -> Self {
        Self::Detailed
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            icon_color: None,
            text_color: None,
            tooltip_label_color: None,
            tooltip_value_color: None,
            sparkline_color: None,
            status_colors: StatusColorConfig::default(),
        }
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            colors: ColorConfig::default(),
            icon_style: IconStyle::default(),
            icon_position: IconPosition::default(),
            icon_spacing: default_icon_spacing(),
            icons: IconConfig::default(),
            update_interval: default_update_interval(),
            visuals: VisualConfig::default(),
            sensors: HashMap::new(),
        }
    }
}

fn default_update_interval() -> u64 {
    1000
}

fn default_icon_spacing() -> u8 {
    1  // Default to 1 space
}

fn default_true() -> bool {
    true
}

fn default_sparkline_length() -> usize {
    8
}

fn default_gauge_width() -> usize {
    12
}

fn default_top_processes_count() -> u8 {
    10
}

fn default_process_name_length() -> u8 {
    20
}

impl GlobalConfig {
    /// Load configuration from the standard config file location.
    ///
    /// Searches for config in:
    /// 1. ~/.config/waysensor-rs/config.ron
    /// 2. ~/.waysensor-rs/config.ron (fallback)
    ///
    /// Returns default config if no file is found.
    pub fn load() -> Result<Self, SensorError> {
        if let Some(config_path) = Self::find_config_file() {
            Self::load_from_file(&config_path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load configuration from a specific file path.
    pub fn load_from_file(path: &PathBuf) -> Result<Self, SensorError> {
        let content = std::fs::read_to_string(path).map_err(|e| SensorError::Io(e))?;

        let config: GlobalConfig = ron::from_str(&content).map_err(|e| SensorError::Parse {
            message: format!("Failed to parse config file: {}", e),
            source: None,
        })?;

        Ok(config)
    }

    /// Find the config file in standard locations.
    pub fn find_config_file() -> Option<PathBuf> {
        // Try XDG config directory first
        if let Some(config_dir) = dirs::config_dir() {
            let xdg_path = config_dir.join("waysensor-rs").join("config.ron");
            if xdg_path.exists() {
                return Some(xdg_path);
            }
        }

        // Try home directory fallback
        if let Some(home_dir) = dirs::home_dir() {
            let home_path = home_dir.join(".waysensor-rs").join("config.ron");
            if home_path.exists() {
                return Some(home_path);
            }
        }

        None
    }

    /// Get the default config file path for writing.
    pub fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("waysensor-rs").join("config.ron"))
    }

    /// Save configuration to the default config file location.
    pub fn save(&self) -> Result<(), SensorError> {
        if let Some(config_path) = Self::default_config_path() {
            self.save_to_file(&config_path)
        } else {
            Err(SensorError::Unavailable {
                reason: "Could not determine config directory".to_string(),
                is_temporary: false,
            })
        }
    }

    /// Save configuration to a specific file path.
    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), SensorError> {
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SensorError::Io(e))?;
        }

        let content = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).map_err(|e| SensorError::Parse {
            message: format!("Failed to serialize config: {}", e),
            source: None,
        })?;

        std::fs::write(path, content).map_err(|e| SensorError::Io(e))?;

        Ok(())
    }

    /// Convert GlobalConfig to SensorConfig, applying defaults and overrides.
    pub fn to_sensor_config(&self) -> SensorConfig {
        SensorConfig {
            update_interval: self.update_interval,
            theme: Theme::default(),
            icon_style: self.icon_style,
            icon_position: self.icon_position,
            icon_spacing: self.icon_spacing,
            icons: self.icons.clone(),
            icon_color: self.colors.icon_color.clone(),
            text_color: self.colors.text_color.clone(),
            tooltip_label_color: self.colors.tooltip_label_color.clone(),
            tooltip_value_color: self.colors.tooltip_value_color.clone(),
            sparkline_color: self.colors.sparkline_color.clone(),
            visuals: self.visuals.clone(),
            custom: HashMap::new(),
        }
    }

    /// Create an example configuration file with common settings.
    pub fn example_config() -> Self {
        let mut config = Self::default();

        // Example color scheme (Tokyo Night theme)
        config.colors.icon_color = Some("#7aa2f7".to_string());
        config.colors.text_color = Some("#c0caf5".to_string());
        config.colors.tooltip_label_color = Some("#bb9af7".to_string());
        config.colors.tooltip_value_color = Some("#9ece6a".to_string());
        config.colors.sparkline_color = Some("#f7768e".to_string());

        // Status indicator colors
        config.colors.status_colors.excellent = Some("#9ece6a".to_string());
        config.colors.status_colors.good = Some("#73daca".to_string());
        config.colors.status_colors.warning = Some("#e0af68".to_string());
        config.colors.status_colors.critical = Some("#f7768e".to_string());
        config.colors.status_colors.unknown = Some("#565f89".to_string());

        config.icon_style = IconStyle::NerdFont;
        config.update_interval = 1000;

        // Visual enhancements
        config.visuals.sparklines = true;
        config.visuals.sparkline_length = 8;
        config.visuals.sparkline_style = SparklineStyle::Blocks;
        config.visuals.status_indicators = true;
        config.visuals.extended_metadata = true;
        config.visuals.tooltip_detail = TooltipDetail::Detailed;

        // Add some sensor-specific examples
        let mut cpu_config = HashMap::new();
        cpu_config.insert(
            "warning_threshold".to_string(),
            serde_json::Value::Number(75.into()),
        );
        cpu_config.insert(
            "critical_threshold".to_string(),
            serde_json::Value::Number(90.into()),
        );
        config.sensors.insert(
            "cpu".to_string(),
            serde_json::Value::Object(cpu_config.into_iter().map(|(k, v)| (k, v)).collect()),
        );

        let mut memory_config = HashMap::new();
        memory_config.insert(
            "warning_threshold".to_string(),
            serde_json::Value::Number(80.into()),
        );
        memory_config.insert("include_swap".to_string(), serde_json::Value::Bool(true));
        config.sensors.insert(
            "memory".to_string(),
            serde_json::Value::Object(memory_config.into_iter().map(|(k, v)| (k, v)).collect()),
        );

        let mut thermal_config = HashMap::new();
        thermal_config.insert(
            "warning_threshold".to_string(),
            serde_json::Value::Number(70.into()),
        );
        thermal_config.insert(
            "critical_threshold".to_string(),
            serde_json::Value::Number(85.into()),
        );
        config.sensors.insert(
            "thermal".to_string(),
            serde_json::Value::Object(thermal_config.into_iter().map(|(k, v)| (k, v)).collect()),
        );

        config
    }

    /// Save example configuration with full documentation to a file.
    pub fn save_example_config_to_file(path: &PathBuf) -> Result<(), SensorError> {
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SensorError::Io(e))?;
        }

        let template = r##"// waysensor-rs Configuration File
// ================================
// Complete configuration reference with all available options.
// Copy this to ~/.config/waysensor-rs/config.ron and customize as needed.
//
// Note: Command line arguments override these settings.

(
    // Default icon style for all sensors
    // Options: nerdfont, none
    icon_style: nerdfont,

    // Icon position relative to text in main waybar display
    // Options: before, after
    // - before: Icon appears before value (e.g., "󰍛 50%")
    // - after: Icon appears after value (e.g., "50% 󰍛")
    icon_position: before,

    // Number of spaces between icon and text (1-10)
    // Examples: 1 = "󰍛 50%", 2 = "󰍛  50%", 3 = "󰍛   50%"
    icon_spacing: 1,

    // Default update interval in milliseconds (minimum 100ms)
    // This is the internal update rate for persistent processes
    update_interval: 1000,

    // =============================================================================
    // ICON CONFIGURATION
    // =============================================================================
    // Configure the Unicode icons used by each sensor type.
    // These can be customized to your preference - change any icon to your liking!
    // RON supports perfect Unicode escapes: \u{F0779} for 5-digit codes!

    icons: (
        // CPU sensor icon
        cpu: "\u{F4BC}",                    // 󰒼 CPU chip icon

        // Memory sensor icon
        memory: "\u{EFC5}",                 // 󰿅 Memory/RAM icon

        // Disk/Storage sensor icon
        disk: "\u{F0A0}",                   //  Hard drive icon

        // Network sensor icons (4 variants)
        network_download: "\u{F019}",       //  Download arrow
        network_upload: "\u{F093}",         //  Upload arrow
        network_wifi: "\u{F05A9}",          // 󰖩 WiFi signal
        network_ethernet: "\u{F0200}",      // 󰈀 Ethernet cable

        // Battery sensor icons (6 charge levels)
        battery_full: "\u{F0079}",          // 󰁹 Battery 100%
        battery_three_quarters: "\u{F12A3}", // 󱊣 Battery 75%
        battery_half: "\u{F12A2}",          // 󱊢 Battery 50%
        battery_quarter: "\u{F12A1}",       // 󱊡 Battery 25%
        battery_empty: "\u{F008E}",         // 󰂎 Battery 0%
        battery_charging: "\u{F0084}",      // 󰂄 Battery charging

        // Thermal sensor icons (3 temperature levels)
        thermal_low: "\u{F2CA}",            //  Temperature low
        thermal_medium: "\u{F2C9}",         //  Thermometer medium
        thermal_high: "\u{F2C7}",           //  Temperature high

        // GPU sensor icon
        gpu: "\u{F08AE}",                   // 󰢮 Graphics card icon
    ),

    // =============================================================================
    // COLOR CONFIGURATION
    // =============================================================================
    // All colors use hex format like "#7aa2f7" or RGB like "rgb(122, 162, 247)"
    // Colors support Pango markup for waybar compatibility

    colors: (
        // Icon color (applies to sensor icons. Examples from "Tokyo Night")
        icon_color: Some("#7aa2f7"),        // Blue

        // Main text color (sensor values)
        text_color: Some("#c0caf5"),        // Light blue/gray

        // Tooltip label/key color (left side of key: value pairs)
        tooltip_label_color: Some("#bb9af7"),   // Purple

        // Tooltip value color (right side of key: value pairs)
        tooltip_value_color: Some("#9ece6a"),   // Green

        // Sparkline chart color
        sparkline_color: Some("#f7768e"),       // Red/pink

        // Status indicator colors for different health states
        status_colors: (
            // Excellent status (very low usage, optimal state)
            excellent: Some("#9ece6a"),         // Green
            // Good status (normal usage, healthy)
            good: Some("#73daca"),              // Teal
            // Warning status (elevated usage, needs attention)
            warning: Some("#e0af68"),           // Yellow/orange
            // Critical status (high usage, immediate attention)
            critical: Some("#f7768e"),          // Red
            // Unknown/unavailable status (no data, error state)
            unknown: Some("#565f89"),           // Gray
        ),
    ),

    // =============================================================================
    // VISUAL ENHANCEMENT SETTINGS
    // =============================================================================

    visuals: (
        // Enable sparkline mini-charts showing recent history
        sparklines: true,

        // Show sparklines in main bar text (true) or tooltip only (false)
        // When true: sparklines appear before the percentage value in the bar
        // When false: sparklines only appear in the tooltip as "Usage History"
        sparklines_in_text: true,

        // Number of data points to maintain for sparklines
        // Range: 4-16 recommended (default: 8)
        sparkline_length: 8,

        // Sparkline rendering style
        // Options: blocks, braille, dots, none
        // - blocks: ▁▂▃▄▅▆▇█ (requires Unicode support)
        // - braille: ⠀⠁⠃⠇⠏⠟⠿⡿⣿ (higher density, requires Braille font)
        // - dots: .:·• (basic ASCII, works everywhere)
        // - none: Disable sparklines
        sparkline_style: blocks,

        // Enable status indicator emojis
        // Shows 🟢🟡🟠🔴⚪ based on threshold levels
        status_indicators: false,

        // Enable additional metadata in tooltips
        // Adds extra system information beyond basic metrics
        extended_metadata: true,

        // Tooltip detail level
        // Options: basic, detailed, expert
        // - basic: Essential information only
        // - detailed: Standard view with all key metrics
        // - expert: Maximum information including technical details
        tooltip_detail: detailed,

        // Enable gauge bars in tooltips
        // Shows visual progress bars for percentage values
        tooltip_gauges: true,

        // Width of gauge bars in characters
        // Range: 4-20 recommended (default: 12)
        gauge_width: 12,

        // Style of gauge bars
        // Options: blocks, ascii, dots, equals, custom
        // - blocks: █████░░░░░ (requires Unicode support)
        // - ascii: [#####-----] (basic ASCII, works everywhere)
        // - dots: ●●●●●○○○○○ (Unicode dots, good fallback)
        // - equals: [=====     ] (ASCII equals, simple)
        // - custom: Uses custom characters (requires additional config)
        gauge_style: blocks,

        // Show top processes in tooltips (CPU sensor shows top CPU, memory shows top memory)
        show_top_processes: true,

        // Number of top processes to display (1-20)
        top_processes_count: 10,

        // Maximum length for process names (truncated with ... if longer)
        process_name_max_length: 20,
    ),

    // =============================================================================
    // SENSOR-SPECIFIC CONFIGURATIONS
    // =============================================================================
    // Each sensor can override global settings and add specific options

    sensors: {
        "cpu": {
            "warning_threshold": 75,
            "critical_threshold": 90,
            "show_per_core": true,
            "max_cores_display": 0,
        },
        "memory": {
            "warning_threshold": 80,
            "critical_threshold": 95,
            "include_swap": true,
            "show_breakdown": true,
        },
        "thermal": {
            "warning_threshold": 70,
            "critical_threshold": 85,
            "temperature_unit": "celsius",
        },
        "amd-gpu": {
            "warning_threshold": 80,
            "critical_threshold": 95,
            "display_format": "compact",
            // Control which values appear in waybar text
            "show_temperature": true,
            "show_power": true,
            "show_utilization": true,
            "show_memory": false,
            "show_frequency": false,
            // Custom display order (when all are shown)
            "display_order": ["temperature", "power", "utilization"],
        },
        "nvidia-gpu": {
            "warning_threshold": 80,
            "critical_threshold": 95,
            "gpu_id": 0,
            "show_temperature": true,
            "show_power": true,
            "show_utilization": true,
            "show_memory": true,
            "show_clocks": true,
        },
        "intel-gpu": {
            "warning_threshold": 80,
            "critical_threshold": 95,
            "show_frequency": true,
        },
    },
)

// =============================================================================
// EXAMPLES AND NOTES
// =============================================================================
//
// 1. RON (Rusty Object Notation) Format:
//    - Native Rust format with perfect serde integration
//    - Supports Unicode escapes: \u{F0779} for 5-digit codes!
//    - Comments allowed with // (line) and /* block */ syntax
//    - Familiar syntax for Rust developers
//
// 2. Unicode Icon Support:
//    - Perfect 5-digit Unicode support: \u{F0779}
//    - 4-digit codes also work: \u{F079}
//    - All Nerd Font icons supported seamlessly
//    - Find codes at https://www.nerdfonts.com/cheat-sheet
//
// 3. Minimal Configuration:
//    Just set icon_style and colors, everything else uses sensible defaults
//
// 4. Performance Tuning:
//    - Increase update_interval for lower CPU usage
//    - Disable sparklines and extended_metadata for minimal overhead
//    - Set tooltip_detail to Basic for less processing
//
// 5. Visual Customization:
//    - Match colors to your waybar theme
//    - Try different sparkline_style options
//    - Adjust sparkline_length for more/less history
//
// 6. Per-Sensor Overrides:
//    Sensors respect their specific settings over global ones
//
// 7. Command Line Priority:
//    CLI arguments override this config file
//    Example: --icon-style none overrides icon_style setting
"##;

        std::fs::write(path, template).map_err(|e| SensorError::Io(e))?;

        Ok(())
    }
}

/// Icon position relative to text in the main waybar display.
///
/// Controls whether icons appear before or after the sensor value.
///
/// # Examples
///
/// ```rust
/// use waysensor_rs_core::IconPosition;
/// use std::str::FromStr;
///
/// let pos = IconPosition::from_str("before").unwrap();
/// assert_eq!(pos, IconPosition::Before);
///
/// let pos: IconPosition = "after".parse().unwrap();
/// assert_eq!(pos, IconPosition::After);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IconPosition {
    /// Icon appears before the value (e.g., "󰍛 50%")
    Before,
    /// Icon appears after the value (e.g., "50% 󰍛")
    After,
}

impl Default for IconPosition {
    /// Default to icon before text for consistency with common patterns.
    fn default() -> Self {
        Self::Before
    }
}

impl fmt::Display for IconPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Before => "before",
            Self::After => "after",
        };
        f.write_str(name)
    }
}

impl std::str::FromStr for IconPosition {
    type Err = IconPositionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "before" | "pre" | "left" => Ok(Self::Before),
            "after" | "post" | "right" => Ok(Self::After),
            _ => Err(IconPositionParseError {
                input: s.to_owned(),
                valid_options: &["before", "after"],
            }),
        }
    }
}

/// Error type for parsing [`IconPosition`] from string.
#[derive(Debug, thiserror::Error)]
#[error("Invalid icon position '{input}'. Valid options: {}", valid_options.join(", "))]
pub struct IconPositionParseError {
    input: String,
    valid_options: &'static [&'static str],
}

/// Icon style variants for sensor display.
///
/// Simplified icon system with two options:
/// - **NerdFont**: Unicode icons from Nerd Font (user-customizable via config)
/// - **None**: No icons, text-only output
///
/// # Examples
///
/// ```rust
/// use waysensor_rs_core::IconStyle;
/// use std::str::FromStr;
///
/// let style = IconStyle::from_str("nerdfont").unwrap();
/// assert_eq!(style, IconStyle::NerdFont);
///
/// let style: IconStyle = "none".parse().unwrap();
/// assert_eq!(style, IconStyle::None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IconStyle {
    /// Nerd Font icons (requires Nerd Font installation, customizable via config)
    NerdFont,
    /// No icons, text-only output
    None,
}

impl Default for IconStyle {
    /// Default to no icons for maximum compatibility.
    fn default() -> Self {
        Self::None
    }
}

impl fmt::Display for IconStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::NerdFont => "nerdfont",
            Self::None => "none",
        };
        f.write_str(name)
    }
}

impl std::str::FromStr for IconStyle {
    type Err = IconStyleParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "nerdfont" | "nerd" | "nf" => Ok(Self::NerdFont),
            "none" | "no" | "" => Ok(Self::None),
            _ => Err(IconStyleParseError {
                input: s.to_owned(),
                valid_options: &["nerdfont", "none"],
            }),
        }
    }
}

/// Error type for parsing [`IconStyle`] from string.
#[derive(Debug, thiserror::Error)]
#[error("Invalid icon style '{input}'. Valid options: {}", valid_options.join(", "))]
pub struct IconStyleParseError {
    input: String,
    valid_options: &'static [&'static str],
}

/// Theme configuration for consistent styling across sensors.
///
/// Defines CSS class names for different states that sensors can report.
/// These classes should correspond to styling definitions in your Waybar CSS.
///
/// # Examples
///
/// ```rust
/// use waysensor_rs_core::Theme;
///
/// // Use default theme
/// let theme = Theme::default();
///
/// // Create custom theme
/// let theme = Theme::new()
///     .with_normal("my-normal")
///     .with_warning("my-warning")
///     .with_critical("my-critical");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Theme {
    /// CSS class for normal/neutral state
    pub normal: String,
    /// CSS class for warning state (moderate concern)
    pub warning: String,
    /// CSS class for critical state (high concern)
    pub critical: String,
    /// CSS class for good/positive state
    pub good: String,
    /// CSS class for unknown/unavailable state
    pub unknown: String,
}

impl Theme {
    /// Create a new theme with default class names.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the normal state class name.
    #[must_use]
    pub fn with_normal(mut self, class: impl Into<String>) -> Self {
        self.normal = class.into();
        self
    }

    /// Set the warning state class name.
    #[must_use]
    pub fn with_warning(mut self, class: impl Into<String>) -> Self {
        self.warning = class.into();
        self
    }

    /// Set the critical state class name.
    #[must_use]
    pub fn with_critical(mut self, class: impl Into<String>) -> Self {
        self.critical = class.into();
        self
    }

    /// Set the good state class name.
    #[must_use]
    pub fn with_good(mut self, class: impl Into<String>) -> Self {
        self.good = class.into();
        self
    }

    /// Set the unknown state class name.
    #[must_use]
    pub fn with_unknown(mut self, class: impl Into<String>) -> Self {
        self.unknown = class.into();
        self
    }

    /// Get the appropriate class name for a threshold-based value.
    ///
    /// Returns the CSS class name based on comparing `value` against the thresholds:
    /// - `critical` if `value >= critical_threshold`
    /// - `warning` if `value >= warning_threshold`
    /// - `normal` otherwise
    #[must_use]
    pub fn class_for_thresholds(
        &self,
        value: f64,
        warning_threshold: f64,
        critical_threshold: f64,
    ) -> &str {
        if value >= critical_threshold {
            &self.critical
        } else if value >= warning_threshold {
            &self.warning
        } else {
            &self.normal
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            normal: "normal".to_owned(),
            warning: "warning".to_owned(),
            critical: "critical".to_owned(),
            good: "good".to_owned(),
            unknown: "unknown".to_owned(),
        }
    }
}

/// Configuration for sensor behavior and appearance.
///
/// Provides common configuration options that all sensors can use,
/// along with support for sensor-specific custom configuration via
/// the `custom` field.
///
/// # Examples
///
/// ```rust
/// use waysensor_rs_core::{SensorConfig, Theme, IconStyle};
/// use std::time::Duration;
///
/// let config = SensorConfig::new()
///     .with_update_interval(Duration::from_millis(500))
///     .with_icon_style(IconStyle::NerdFont)
///     .with_theme(Theme::new().with_critical("danger"));
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SensorConfig {
    /// Update interval in milliseconds (minimum 100ms)
    #[serde(deserialize_with = "validate_update_interval")]
    pub update_interval: u64,
    /// Theme configuration for CSS styling
    #[serde(default)]
    pub theme: Theme,
    /// Icon style preference
    #[serde(default)]
    pub icon_style: IconStyle,
    /// Icon position (before or after the value)
    #[serde(default)]
    pub icon_position: IconPosition,
    /// Number of spaces between icon and text
    #[serde(default = "default_icon_spacing")]
    pub icon_spacing: u8,
    /// Icon definitions for different sensor types
    #[serde(default)]
    pub icons: IconConfig,
    /// Optional color for icons (hex format like "#7aa2f7")
    #[serde(default)]
    pub icon_color: Option<String>,
    /// Optional color for text (hex format like "#c0caf5")
    #[serde(default)]
    pub text_color: Option<String>,
    /// Optional color for tooltip labels/keys (hex format like "#bb9af7")
    #[serde(default)]
    pub tooltip_label_color: Option<String>,
    /// Optional color for tooltip values (hex format like "#9ece6a")
    #[serde(default)]
    pub tooltip_value_color: Option<String>,
    /// Optional color for sparklines (hex format like "#f7768e")
    #[serde(default)]
    pub sparkline_color: Option<String>,
    /// Visual enhancement settings
    #[serde(default)]
    pub visuals: VisualConfig,
    /// Sensor-specific custom configuration
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl SensorConfig {
    /// Minimum allowed update interval in milliseconds.
    pub const MIN_UPDATE_INTERVAL: u64 = 100;

    /// Create a new sensor configuration with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the update interval from a Duration.
    ///
    /// # Panics
    ///
    /// Panics if the duration is less than 100ms or cannot be represented as milliseconds.
    #[must_use]
    pub fn with_update_interval(mut self, interval: std::time::Duration) -> Self {
        let millis = interval.as_millis();
        assert!(
            millis >= Self::MIN_UPDATE_INTERVAL as u128,
            "Update interval must be at least {}ms",
            Self::MIN_UPDATE_INTERVAL
        );
        assert!(millis <= u64::MAX as u128, "Update interval too large");
        self.update_interval = millis as u64;
        self
    }

    /// Set the update interval in milliseconds.
    ///
    /// # Panics
    ///
    /// Panics if `millis` is less than [`Self::MIN_UPDATE_INTERVAL`].
    #[must_use]
    pub fn with_update_interval_ms(mut self, millis: u64) -> Self {
        assert!(
            millis >= Self::MIN_UPDATE_INTERVAL,
            "Update interval must be at least {}ms",
            Self::MIN_UPDATE_INTERVAL
        );
        self.update_interval = millis;
        self
    }

    /// Set the theme configuration.
    #[must_use]
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the icon style.
    #[must_use]
    pub fn with_icon_style(mut self, style: IconStyle) -> Self {
        self.icon_style = style;
        self
    }

    /// Set the icon position.
    #[must_use]
    pub fn with_icon_position(mut self, position: IconPosition) -> Self {
        self.icon_position = position;
        self
    }

    /// Set the icon color (Pango markup format, e.g., "#7aa2f7").
    #[must_use]
    pub fn with_icon_color(mut self, color: impl Into<String>) -> Self {
        self.icon_color = Some(color.into());
        self
    }

    /// Set the text color (Pango markup format, e.g., "#c0caf5").
    #[must_use]
    pub fn with_text_color(mut self, color: impl Into<String>) -> Self {
        self.text_color = Some(color.into());
        self
    }

    /// Set the tooltip label color (Pango markup format, e.g., "#bb9af7").
    #[must_use]
    pub fn with_tooltip_label_color(mut self, color: impl Into<String>) -> Self {
        self.tooltip_label_color = Some(color.into());
        self
    }

    /// Set the tooltip value color (Pango markup format, e.g., "#9ece6a").
    #[must_use]
    pub fn with_tooltip_value_color(mut self, color: impl Into<String>) -> Self {
        self.tooltip_value_color = Some(color.into());
        self
    }

    /// Apply command line color overrides to the configuration.
    #[must_use]
    pub fn apply_color_overrides(
        mut self,
        icon_color: Option<String>,
        text_color: Option<String>,
        tooltip_label_color: Option<String>,
        tooltip_value_color: Option<String>,
    ) -> Self {
        if let Some(color) = icon_color {
            self.icon_color = Some(color);
        }
        if let Some(color) = text_color {
            self.text_color = Some(color);
        }
        if let Some(color) = tooltip_label_color {
            self.tooltip_label_color = Some(color);
        }
        if let Some(color) = tooltip_value_color {
            self.tooltip_value_color = Some(color);
        }
        self
    }

    /// Add a custom configuration value.
    #[must_use]
    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }

    /// Get the update interval as a Duration.
    #[must_use]
    pub fn update_interval_duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.update_interval)
    }

    /// Get a custom configuration value by key.
    #[must_use]
    pub fn get_custom(&self, key: &str) -> Option<&serde_json::Value> {
        self.custom.get(key)
    }
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            update_interval: 1000, // 1 second
            theme: Theme::default(),
            icon_style: IconStyle::default(),
            icon_position: IconPosition::default(),
            icon_spacing: default_icon_spacing(),
            icons: IconConfig::default(),
            icon_color: None,
            text_color: None,
            tooltip_label_color: None,
            tooltip_value_color: None,
            sparkline_color: None,
            visuals: VisualConfig::default(),
            custom: HashMap::new(),
        }
    }
}

/// Validate update interval during deserialization.
fn validate_update_interval<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let interval = u64::deserialize(deserializer)?;
    if interval < SensorConfig::MIN_UPDATE_INTERVAL {
        return Err(serde::de::Error::custom(format!(
            "Update interval must be at least {}ms, got {}ms",
            SensorConfig::MIN_UPDATE_INTERVAL,
            interval
        )));
    }
    Ok(interval)
}

/// Trait for all system sensors providing Waybar-compatible output.
///
/// This trait defines the common interface that all sensors must implement
/// to provide consistent behavior across the waysensor-rs sensor suite.
///
/// # Examples
///
/// ```rust
/// use waysensor_rs_core::{Sensor, SensorConfig, WaybarOutput, SensorError};
///
/// struct CpuSensor {
///     name: String,
///     config: SensorConfig,
/// }
///
/// impl Sensor for CpuSensor {
///     type Error = SensorError;
///
///     fn read(&mut self) -> Result<WaybarOutput, Self::Error> {
///         // Read CPU data and format for Waybar
///         Ok(WaybarOutput::from_str("50%")
///             .with_tooltip("CPU Usage: 50%")
///             .with_percentage(50))
///     }
///
///     fn name(&self) -> &str {
///         &self.name
///     }
///
///     fn configure(&mut self, config: SensorConfig) -> Result<(), Self::Error> {
///         self.config = config;
///         Ok(())
///     }
/// }
/// ```
pub trait Sensor {
    /// Error type for sensor operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Read current sensor data and return Waybar-formatted output.
    ///
    /// This method should be lightweight and suitable for frequent calling
    /// based on the configured update interval.
    ///
    /// # Errors
    ///
    /// Returns an error if the sensor data cannot be read or parsed.
    fn read(&mut self) -> Result<WaybarOutput, Self::Error>;

    /// Get the unique name/identifier for this sensor.
    ///
    /// This name is used for logging, configuration, and identification
    /// purposes. It should be stable across sensor instances.
    fn name(&self) -> &str;

    /// Update the sensor configuration.
    ///
    /// This method allows dynamic reconfiguration of sensor behavior
    /// without recreating the sensor instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid or cannot be applied.
    fn configure(&mut self, config: SensorConfig) -> Result<(), Self::Error>;

    /// Check if the sensor is available on this system.
    ///
    /// Default implementation returns `Ok(())`. Sensors should override
    /// this if they have specific system requirements.
    ///
    /// # Errors
    ///
    /// Returns an error if the sensor is not available or supported.
    fn check_availability(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Get the current sensor configuration.
    ///
    /// Default implementation returns a default configuration. Sensors
    /// that maintain configuration state should override this.
    fn config(&self) -> &SensorConfig {
        use std::sync::LazyLock;
        static DEFAULT_CONFIG: LazyLock<SensorConfig> = LazyLock::new(|| SensorConfig {
            update_interval: 1000,
            theme: Theme::default(),
            icon_style: IconStyle::None,
            icon_position: IconPosition::default(),
            icon_spacing: default_icon_spacing(),
            icons: IconConfig::default(),
            icon_color: None,
            text_color: None,
            tooltip_label_color: None,
            tooltip_value_color: None,
            sparkline_color: None,
            visuals: VisualConfig::default(),
            custom: HashMap::new(),
        });
        &DEFAULT_CONFIG
    }
}

/// Utility functions for formatting sensor data and creating Waybar output.
///
/// This module provides common formatting utilities that sensors can use
/// to create consistent, well-formatted output.
pub mod format {
    use super::{IconPosition, IconStyle, SensorConfig, Theme, WaybarOutput};

    /// Combine text with an icon based on the specified icon style and position.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use waysensor_rs_core::{format, IconStyle, IconPosition};
    ///
    /// let result = format::with_icon("50%", "󰍛", IconStyle::NerdFont, IconPosition::Before);
    /// assert_eq!(result, "󰍛 50%");
    ///
    /// let result = format::with_icon("50%", "󰍛", IconStyle::NerdFont, IconPosition::After);
    /// assert_eq!(result, "50% 󰍛");
    ///
    /// let result = format::with_icon("50%", "󰍛", IconStyle::None, IconPosition::Before);
    /// assert_eq!(result, "50%");
    /// ```
    #[must_use]
    pub fn with_icon(text: &str, icon: &str, style: IconStyle, position: IconPosition, spacing: u8) -> String {
        match style {
            IconStyle::None => text.to_owned(),
            _ if icon.is_empty() => text.to_owned(),
            IconStyle::NerdFont => {
                let spacer = " ".repeat(spacing as usize);
                match position {
                    IconPosition::Before => format!("{icon}{spacer}{text}"),
                    IconPosition::After => format!("{text}{spacer}{icon}"),
                }
            },
        }
    }

    /// Combine text with an icon and apply optional color styling using Pango markup.
    ///
    /// This function creates properly formatted output with optional color styling
    /// for both icon and text components using Pango markup supported by Waybar.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use waysensor_rs_core::{format, SensorConfig, IconStyle};
    ///
    /// let config = SensorConfig::new()
    ///     .with_icon_style(IconStyle::NerdFont)
    ///     .with_icon_color("#7aa2f7");
    ///
    /// let result = format::with_icon_and_colors("50%", "󰍛", &config);
    /// assert_eq!(result, "<span color=\"#7aa2f7\">󰍛</span> 50%");
    /// ```
    #[must_use]
    pub fn with_icon_and_colors(text: &str, icon: &str, config: &SensorConfig) -> String {
        // Check if icon is effectively empty (empty or whitespace-only)
        // Waybar/Pango handles font fallback automatically - we just output UTF-8 characters
        let icon_is_empty = icon.trim().is_empty();

        match config.icon_style {
            IconStyle::None => {
                if let Some(color) = &config.text_color {
                    format!("<span color=\"{}\">{}</span>", color, text)
                } else {
                    text.to_owned()
                }
            }
            IconStyle::NerdFont if icon_is_empty => {
                if let Some(color) = &config.text_color {
                    format!("<span color=\"{}\">{}</span>", color, text)
                } else {
                    text.to_owned()
                }
            }
            IconStyle::NerdFont => {
                let icon_part = if let Some(color) = &config.icon_color {
                    format!("<span color=\"{}\">{}</span>", color, icon)
                } else {
                    icon.to_owned()
                };

                let text_part = if let Some(color) = &config.text_color {
                    format!("<span color=\"{}\">{}</span>", color, text)
                } else {
                    text.to_owned()
                };

                let spacer = " ".repeat(config.icon_spacing as usize);
                match config.icon_position {
                    IconPosition::Before => format!("{}{}{}", icon_part, spacer, text_part),
                    IconPosition::After => format!("{}{}{}", text_part, spacer, icon_part),
                }
            }
        }
    }

    /// Format a key-value pair with optional coloring for tooltips.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use waysensor_rs_core::{format, SensorConfig};
    ///
    /// let config = SensorConfig::new()
    ///     .with_tooltip_label_color("#bb9af7")
    ///     .with_tooltip_value_color("#9ece6a");
    ///
    /// let result = format::key_value("CPU", "AMD Ryzen 9", &config);
    /// assert_eq!(result, "<span color=\"#bb9af7\">CPU:</span> <span color=\"#9ece6a\">AMD Ryzen 9</span>");
    /// ```
    #[must_use]
    pub fn key_value(key: &str, value: &str, config: &SensorConfig) -> String {
        let key_part = if let Some(color) = &config.tooltip_label_color {
            format!("<span color=\"{}\">{key}:</span>", color)
        } else {
            format!("{key}:")
        };

        let value_part = if let Some(color) = &config.tooltip_value_color {
            format!("<span color=\"{}\">{value}</span>", color)
        } else {
            value.to_owned()
        };

        format!("{} {}", key_part, value_part)
    }

    /// Format just a key/label with optional coloring.
    #[must_use]
    pub fn key_only(key: &str, config: &SensorConfig) -> String {
        if let Some(color) = &config.tooltip_label_color {
            format!("<span color=\"{}\">{key}:</span>", color)
        } else {
            format!("{key}:")
        }
    }

    /// Format just a value with optional coloring.
    #[must_use]
    pub fn value_only(value: &str, config: &SensorConfig) -> String {
        if let Some(color) = &config.tooltip_value_color {
            format!("<span color=\"{}\">{value}</span>", color)
        } else {
            value.to_owned()
        }
    }

    /// Format bytes into a human-readable string with appropriate units.
    ///
    /// Uses binary units (1024-based) and shows 1 decimal place for values >= 1KB.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use waysensor_rs_core::format;
    ///
    /// assert_eq!(format::bytes_to_human(512), "512B");
    /// assert_eq!(format::bytes_to_human(1024), "1.0KB");
    /// assert_eq!(format::bytes_to_human(1536), "1.5KB");
    /// assert_eq!(format::bytes_to_human(1048576), "1.0MB");
    /// ```
    #[must_use]
    pub fn bytes_to_human(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
        const THRESHOLD: f64 = 1024.0;

        if bytes == 0 {
            return "0B".to_owned();
        }

        let mut size = bytes as f64;
        let mut unit_idx = 0;

        while size >= THRESHOLD && unit_idx < UNITS.len() - 1 {
            size /= THRESHOLD;
            unit_idx += 1;
        }

        if unit_idx == 0 {
            format!("{size:.0}{}", UNITS[unit_idx])
        } else {
            format!("{size:.1}{}", UNITS[unit_idx])
        }
    }

    /// Format a rate (bytes per second) into a human-readable string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use waysensor_rs_core::format;
    ///
    /// assert_eq!(format::rate_to_human(1024), "1.0KB/s");
    /// assert_eq!(format::rate_to_human(1048576), "1.0MB/s");
    /// ```
    #[must_use]
    pub fn rate_to_human(bytes_per_second: u64) -> String {
        format!("{}/s", bytes_to_human(bytes_per_second))
    }

    /// Format a frequency in Hz to a human-readable string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use waysensor_rs_core::format;
    ///
    /// assert_eq!(format::frequency_to_human(2400000000), "2.4GHz");
    /// assert_eq!(format::frequency_to_human(1500000), "1.5MHz");
    /// ```
    #[must_use]
    pub fn frequency_to_human(hz: u64) -> String {
        const UNITS: &[&str] = &["Hz", "KHz", "MHz", "GHz"];
        const THRESHOLD: f64 = 1000.0;

        let mut freq = hz as f64;
        let mut unit_idx = 0;

        while freq >= THRESHOLD && unit_idx < UNITS.len() - 1 {
            freq /= THRESHOLD;
            unit_idx += 1;
        }

        if unit_idx == 0 {
            format!("{freq:.0}{}", UNITS[unit_idx])
        } else {
            format!("{freq:.1}{}", UNITS[unit_idx])
        }
    }

    /// Create a gauge bar visualization based on percentage and configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use waysensor_rs_core::{format, GaugeStyle};
    ///
    /// // Using blocks style
    /// assert_eq!(format::create_gauge(50.0, 10, GaugeStyle::Blocks), "█████░░░░░");
    ///
    /// // Using ASCII style
    /// assert_eq!(format::create_gauge(30.0, 10, GaugeStyle::Ascii), "[###-------]");
    /// ```
    #[must_use]
    pub fn create_gauge(percentage: f64, width: usize, style: crate::GaugeStyle) -> String {
        let filled = ((percentage.clamp(0.0, 100.0) / 100.0) * width as f64).round() as usize;
        let empty = width.saturating_sub(filled);

        match style {
            crate::GaugeStyle::Blocks => {
                let filled_char = '█';
                let empty_char = '░';
                format!(
                    "{}{}",
                    filled_char.to_string().repeat(filled),
                    empty_char.to_string().repeat(empty)
                )
            }
            crate::GaugeStyle::Ascii => {
                format!("[{}{}]", "#".repeat(filled), "-".repeat(empty))
            }
            crate::GaugeStyle::Dots => {
                let filled_char = '●';
                let empty_char = '○';
                format!(
                    "{}{}",
                    filled_char.to_string().repeat(filled),
                    empty_char.to_string().repeat(empty)
                )
            }
            crate::GaugeStyle::Equals => {
                format!("[{}{}]", "=".repeat(filled), " ".repeat(empty))
            }
            crate::GaugeStyle::Custom => {
                // For now, fall back to blocks style
                // TODO: Support custom characters from config
                let filled_char = '█';
                let empty_char = '░';
                format!(
                    "{}{}",
                    filled_char.to_string().repeat(filled),
                    empty_char.to_string().repeat(empty)
                )
            }
        }
    }

    /// Create Waybar output with automatic theme-based CSS class selection.
    ///
    /// The CSS class is determined by comparing `value` against the thresholds:
    /// - `critical` class if `value >= critical_threshold`
    /// - `warning` class if `value >= warning_threshold`
    /// - `normal` class otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use waysensor_rs_core::{format, Theme};
    ///
    /// let theme = Theme::default();
    /// let output = format::themed_output(
    ///     "85%".to_owned(),
    ///     Some("CPU Usage: 85%".to_owned()),
    ///     Some(85),
    ///     85.0,
    ///     70.0,  // warning threshold
    ///     90.0,  // critical threshold
    ///     &theme,
    /// );
    ///
    /// assert_eq!(output.class.as_deref(), Some("warning"));
    /// ```
    #[must_use]
    pub fn themed_output(
        text: String,
        tooltip: Option<String>,
        percentage: Option<u8>,
        value: f64,
        warning_threshold: f64,
        critical_threshold: f64,
        theme: &Theme,
    ) -> WaybarOutput {
        let class = Some(
            theme
                .class_for_thresholds(value, warning_threshold, critical_threshold)
                .to_owned(),
        );

        WaybarOutput {
            text,
            tooltip,
            class,
            percentage,
        }
    }

    /// Create a simple themed output without percentage.
    ///
    /// Convenience wrapper around [`themed_output`] for sensors that don't report percentages.
    #[must_use]
    pub fn simple_themed_output(
        text: String,
        tooltip: Option<String>,
        value: f64,
        warning_threshold: f64,
        critical_threshold: f64,
        theme: &Theme,
    ) -> WaybarOutput {
        themed_output(
            text,
            tooltip,
            None,
            value,
            warning_threshold,
            critical_threshold,
            theme,
        )
    }

    /// Generate a sparkline from a series of values using Unicode block characters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use waysensor_rs_core::{format, SparklineStyle};
    ///
    /// let data = vec![10.0, 20.0, 50.0, 80.0, 30.0, 60.0];
    /// let sparkline = format::create_sparkline(&data, SparklineStyle::Blocks);
    /// // Returns something like: "▂▃▅▇▄▆"
    /// ```
    #[must_use]
    pub fn create_sparkline(values: &[f64], style: super::SparklineStyle) -> String {
        use super::SparklineStyle;

        if values.is_empty() {
            return String::new();
        }

        match style {
            SparklineStyle::None => String::new(),
            SparklineStyle::Blocks => create_block_sparkline(values),
            SparklineStyle::Braille => create_braille_sparkline(values),
            SparklineStyle::Dots => create_dot_sparkline(values),
        }
    }

    /// Create sparkline using Unicode block characters (▁▂▃▄▅▆▇█).
    #[must_use]
    pub fn create_block_sparkline(values: &[f64]) -> String {
        const BLOCKS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        if values.is_empty() {
            return String::new();
        }

        let min_val = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_val = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        if (max_val - min_val).abs() < f64::EPSILON {
            // All values are the same
            return BLOCKS[BLOCKS.len() / 2].to_string().repeat(values.len());
        }

        values
            .iter()
            .map(|&val| {
                let normalized = (val - min_val) / (max_val - min_val);
                let index = ((normalized * (BLOCKS.len() - 1) as f64).round() as usize)
                    .min(BLOCKS.len() - 1);
                BLOCKS[index]
            })
            .collect()
    }

    /// Create sparkline using Braille patterns for higher density.
    #[must_use]
    pub fn create_braille_sparkline(values: &[f64]) -> String {
        // Braille patterns: dots 1,2,3,4 for left column, dots 5,6,7,8 for right column
        // We'll use a simplified approach with 8 levels per column
        const BRAILLE_BASE: u32 = 0x2800; // Base Braille pattern

        if values.is_empty() {
            return String::new();
        }

        let min_val = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_val = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        if (max_val - min_val).abs() < f64::EPSILON {
            return "⠤".repeat(values.len() / 2 + values.len() % 2);
        }

        let mut result = String::new();
        let mut i = 0;

        while i < values.len() {
            let left_val = values[i];
            let right_val = values.get(i + 1).copied().unwrap_or(left_val);

            let left_norm = (left_val - min_val) / (max_val - min_val);
            let right_norm = (right_val - min_val) / (max_val - min_val);

            let left_level = (left_norm * 3.0).round() as u32;
            let right_level = (right_norm * 3.0).round() as u32;

            // Map levels to Braille dot patterns
            let mut pattern = BRAILLE_BASE;
            match left_level {
                0 => {}
                1 => pattern |= 0x04, // dot 3
                2 => pattern |= 0x06, // dots 2,3
                _ => pattern |= 0x07, // dots 1,2,3
            }
            match right_level {
                0 => {}
                1 => pattern |= 0x20, // dot 6
                2 => pattern |= 0x30, // dots 5,6
                _ => pattern |= 0x38, // dots 4,5,6
            }

            if let Some(braille_char) = char::from_u32(pattern) {
                result.push(braille_char);
            }

            i += 2;
        }

        result
    }

    /// Create sparkline using simple dots and dashes.
    #[must_use]
    pub fn create_dot_sparkline(values: &[f64]) -> String {
        const DOTS: &[char] = &['.', ':', '·', '•'];

        if values.is_empty() {
            return String::new();
        }

        let min_val = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_val = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        if (max_val - min_val).abs() < f64::EPSILON {
            return DOTS[DOTS.len() / 2].to_string().repeat(values.len());
        }

        values
            .iter()
            .map(|&val| {
                let normalized = (val - min_val) / (max_val - min_val);
                let index =
                    ((normalized * (DOTS.len() - 1) as f64).round() as usize).min(DOTS.len() - 1);
                DOTS[index]
            })
            .collect()
    }

    /// Get status indicator emoji based on value and thresholds.
    /// Returns None if status indicators are disabled.
    #[must_use]
    pub fn status_indicator(
        value: f64,
        warning_threshold: f64,
        critical_threshold: f64,
        status_indicators_enabled: bool,
    ) -> Option<&'static str> {
        if !status_indicators_enabled {
            return None;
        }
        
        Some(if value >= critical_threshold {
            "🔴" // Critical
        } else if value >= warning_threshold {
            "🟡" // Warning
        } else if value < warning_threshold * 0.3 {
            "🟢" // Excellent (very low usage)
        } else {
            ""  // No indicator for normal state
        })
    }

    /// Format a sparkline with color support.
    #[must_use]
    pub fn colored_sparkline(sparkline: &str, color: Option<&str>) -> String {
        if let Some(color) = color {
            format!("<span color=\"{}\">{}</span>", color, sparkline)
        } else {
            sparkline.to_owned()
        }
    }

    /// Get top processes by CPU usage
    #[must_use]
    pub fn get_top_processes_by_cpu(count: usize, max_name_length: usize) -> Vec<(String, f64)> {
        use std::process::Command;
        
        let output = match Command::new("ps")
            .args(["-eo", "pid,pcpu,comm", "--sort=-pcpu", "--no-headers"])
            .output() {
            Ok(output) => output,
            Err(_) => return Vec::new(),
        };
            
        if !output.status.success() {
            return Vec::new();
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .take(count)
            .filter_map(|line| {
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 3 {
                    let cpu_usage = parts[1].parse::<f64>().ok()?;
                    let mut process_name = parts[2].to_string();
                    
                    // Truncate process name if too long
                    if process_name.len() > max_name_length {
                        process_name.truncate(max_name_length - 3);
                        process_name.push_str("...");
                    }
                    
                    Some((process_name, cpu_usage))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get top processes by memory usage
    #[must_use]
    pub fn get_top_processes_by_memory(count: usize, max_name_length: usize) -> Vec<(String, f64)> {
        use std::process::Command;
        
        let output = match Command::new("ps")
            .args(["-eo", "pid,pmem,comm", "--sort=-pmem", "--no-headers"])
            .output() {
            Ok(output) => output,
            Err(_) => return Vec::new(),
        };
            
        if !output.status.success() {
            return Vec::new();
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .take(count)
            .filter_map(|line| {
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 3 {
                    let mem_usage = parts[1].parse::<f64>().ok()?;
                    let mut process_name = parts[2].to_string();
                    
                    // Truncate process name if too long
                    if process_name.len() > max_name_length {
                        process_name.truncate(max_name_length - 3);
                        process_name.push_str("...");
                    }
                    
                    Some((process_name, mem_usage))
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Format top processes for tooltip display
    #[must_use]
    pub fn format_top_processes(
        processes: &[(String, f64)], 
        metric_name: &str,
        label_color: Option<&str>,
        value_color: Option<&str>
    ) -> String {
        if processes.is_empty() {
            return String::new();
        }
        
        let header = if let Some(color) = label_color {
            format!("\n\n<span color=\"{}\">{}</span>:", color, metric_name)
        } else {
            format!("\n\n{}:", metric_name)
        };
        let mut result = header;
        
        for (name, usage) in processes {
            let formatted_usage = if let Some(color) = value_color {
                format!("<span color=\"{}\">{:.1}%</span>", color, usage)
            } else {
                format!("{:.1}%", usage)
            };
            result.push_str(&format!("\n  {}: {}", name, formatted_usage));
        }
        result
    }
}

/// Common error types for sensor operations.
///
/// This enum provides a comprehensive set of error types that cover
/// the most common failure modes in sensor implementations.
#[derive(Debug, thiserror::Error)]
pub enum SensorError {
    /// I/O error occurred while reading sensor data.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Error parsing sensor data from text format.
    #[error("Parse error: {message}")]
    Parse {
        /// Description of what failed to parse
        message: String,
        /// Optional source error for chaining
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Configuration error (invalid settings, etc.).
    #[error("Configuration error: {message}")]
    Config {
        /// Description of the configuration issue
        message: String,
        /// The invalid configuration value if applicable
        value: Option<String>,
    },

    /// Sensor is not available on this system.
    #[error("Sensor unavailable: {reason}")]
    Unavailable {
        /// Reason why the sensor is unavailable
        reason: String,
        /// Whether this is a temporary or permanent condition
        is_temporary: bool,
    },

    /// Permission denied accessing sensor data.
    #[error("Permission denied: {resource}")]
    PermissionDenied {
        /// The resource that couldn't be accessed
        resource: String,
    },

    /// Timeout occurred while reading sensor data.
    #[error("Timeout after {duration:?} while {operation}")]
    Timeout {
        /// How long the operation took before timing out
        duration: std::time::Duration,
        /// Description of what operation timed out
        operation: String,
    },

    /// Invalid data format or unexpected values.
    #[error("Invalid data: {message}")]
    InvalidData {
        /// Description of what makes the data invalid
        message: String,
        /// The invalid data if it can be safely displayed
        data: Option<String>,
    },
}

impl SensorError {
    /// Create a new parse error with a simple message.
    pub fn parse<S: Into<String>>(message: S) -> Self {
        Self::Parse {
            message: message.into(),
            source: None,
        }
    }

    /// Create a new parse error with a source error.
    pub fn parse_with_source<S: Into<String>, E>(message: S, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Parse {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a new configuration error.
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
            value: None,
        }
    }

    /// Create a new configuration error with the invalid value.
    pub fn config_with_value<S: Into<String>, V: Into<String>>(message: S, value: V) -> Self {
        Self::Config {
            message: message.into(),
            value: Some(value.into()),
        }
    }

    /// Create a new unavailable error.
    pub fn unavailable<S: Into<String>>(reason: S) -> Self {
        Self::Unavailable {
            reason: reason.into(),
            is_temporary: false,
        }
    }

    /// Create a new temporary unavailable error.
    pub fn temporarily_unavailable<S: Into<String>>(reason: S) -> Self {
        Self::Unavailable {
            reason: reason.into(),
            is_temporary: true,
        }
    }

    /// Create a new permission denied error.
    pub fn permission_denied<S: Into<String>>(resource: S) -> Self {
        Self::PermissionDenied {
            resource: resource.into(),
        }
    }

    /// Create a new timeout error.
    pub fn timeout<S: Into<String>>(duration: std::time::Duration, operation: S) -> Self {
        Self::Timeout {
            duration,
            operation: operation.into(),
        }
    }

    /// Create a new invalid data error.
    pub fn invalid_data<S: Into<String>>(message: S) -> Self {
        Self::InvalidData {
            message: message.into(),
            data: None,
        }
    }

    /// Create a new invalid data error with the problematic data.
    pub fn invalid_data_with_value<S: Into<String>, D: Into<String>>(message: S, data: D) -> Self {
        Self::InvalidData {
            message: message.into(),
            data: Some(data.into()),
        }
    }

    /// Check if this error represents a temporary condition.
    #[must_use]
    pub fn is_temporary(&self) -> bool {
        match self {
            Self::Unavailable { is_temporary, .. } => *is_temporary,
            Self::Timeout { .. } => true,
            Self::Io(err) => matches!(
                err.kind(),
                std::io::ErrorKind::Interrupted | std::io::ErrorKind::TimedOut
            ),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_waybar_output_builder() {
        let output = WaybarOutput::from_str("50%")
            .with_tooltip("CPU Usage: 50%")
            .with_class("normal")
            .with_percentage(50);

        assert_eq!(output.text, "50%");
        assert_eq!(output.tooltip, Some("CPU Usage: 50%".to_owned()));
        assert_eq!(output.class, Some("normal".to_owned()));
        assert_eq!(output.percentage, Some(50));
    }

    #[test]
    #[should_panic(expected = "Percentage must be <= 100")]
    fn test_waybar_output_invalid_percentage() {
        let _ = WaybarOutput::from_str("150%").with_percentage(150);
    }

    #[test]
    fn test_icon_style_parse() {
        assert_eq!(
            "nerdfont".parse::<IconStyle>().unwrap(),
            IconStyle::NerdFont
        );
        assert_eq!("nerd".parse::<IconStyle>().unwrap(), IconStyle::NerdFont);
        assert_eq!("nf".parse::<IconStyle>().unwrap(), IconStyle::NerdFont);
        assert_eq!("none".parse::<IconStyle>().unwrap(), IconStyle::None);
        assert_eq!("no".parse::<IconStyle>().unwrap(), IconStyle::None);
        assert_eq!("".parse::<IconStyle>().unwrap(), IconStyle::None);

        assert!("invalid".parse::<IconStyle>().is_err());
    }

    #[test]
    fn test_theme_builder() {
        let theme = Theme::new()
            .with_normal("my-normal")
            .with_warning("my-warning")
            .with_critical("my-critical");

        assert_eq!(theme.normal, "my-normal");
        assert_eq!(theme.warning, "my-warning");
        assert_eq!(theme.critical, "my-critical");
    }

    #[test]
    fn test_theme_class_for_thresholds() {
        let theme = Theme::default();

        assert_eq!(theme.class_for_thresholds(50.0, 70.0, 90.0), &theme.normal);
        assert_eq!(theme.class_for_thresholds(80.0, 70.0, 90.0), &theme.warning);
        assert_eq!(
            theme.class_for_thresholds(95.0, 70.0, 90.0),
            &theme.critical
        );
    }

    #[test]
    fn test_sensor_config_builder() {
        let config = SensorConfig::new()
            .with_update_interval(Duration::from_millis(500))
            .with_icon_style(IconStyle::NerdFont);

        assert_eq!(config.update_interval, 500);
        assert_eq!(config.icon_style, IconStyle::NerdFont);
        assert_eq!(
            config.update_interval_duration(),
            Duration::from_millis(500)
        );
    }

    #[test]
    #[should_panic(expected = "Update interval must be at least 100ms")]
    fn test_sensor_config_invalid_interval() {
        let _ = SensorConfig::new().with_update_interval_ms(50);
    }

    #[test]
    fn test_bytes_to_human() {
        assert_eq!(format::bytes_to_human(0), "0B");
        assert_eq!(format::bytes_to_human(512), "512B");
        assert_eq!(format::bytes_to_human(1024), "1.0KB");
        assert_eq!(format::bytes_to_human(1536), "1.5KB");
        assert_eq!(format::bytes_to_human(1048576), "1.0MB");
        assert_eq!(format::bytes_to_human(1073741824), "1.0GB");
    }

    #[test]
    fn test_rate_to_human() {
        assert_eq!(format::rate_to_human(1024), "1.0KB/s");
        assert_eq!(format::rate_to_human(1048576), "1.0MB/s");
    }

    #[test]
    fn test_frequency_to_human() {
        assert_eq!(format::frequency_to_human(1000), "1.0KHz");
        assert_eq!(format::frequency_to_human(1500000), "1.5MHz");
        assert_eq!(format::frequency_to_human(2400000000), "2.4GHz");
    }

    #[test]
    fn test_with_icon() {
        assert_eq!(format::with_icon("50%", "󰍛", IconStyle::NerdFont, IconPosition::Before, 1), "󰍛 50%");
        assert_eq!(format::with_icon("50%", "󰍛", IconStyle::NerdFont, IconPosition::After, 1), "50% 󰍛");
        assert_eq!(format::with_icon("50%", "󰍛", IconStyle::None, IconPosition::Before, 1), "50%");
        assert_eq!(format::with_icon("50%", "", IconStyle::NerdFont, IconPosition::Before, 1), "50%");
        // Test custom spacing
        assert_eq!(format::with_icon("50%", "󰍛", IconStyle::NerdFont, IconPosition::Before, 2), "󰍛  50%");
        assert_eq!(format::with_icon("50%", "󰍛", IconStyle::NerdFont, IconPosition::After, 3), "50%   󰍛");
    }

    #[test]
    fn test_themed_output() {
        let theme = Theme::default();
        let output = format::themed_output(
            "50%".to_owned(),
            Some("CPU Usage: 50%".to_owned()),
            Some(50),
            50.0,
            70.0,
            90.0,
            &theme,
        );

        assert_eq!(output.text, "50%");
        assert_eq!(output.class, Some("normal".to_owned()));
        assert_eq!(output.percentage, Some(50));
    }

    #[test]
    fn test_sensor_error_constructors() {
        let err = SensorError::parse("Invalid format");
        assert!(matches!(err, SensorError::Parse { .. }));

        let err = SensorError::config_with_value("Invalid setting", "bad_value");
        assert!(matches!(err, SensorError::Config { .. }));

        let err = SensorError::temporarily_unavailable("Service down");
        assert!(err.is_temporary());

        let err = SensorError::unavailable("Not supported");
        assert!(!err.is_temporary());
    }
}
