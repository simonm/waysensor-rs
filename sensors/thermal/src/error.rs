//! Error handling for thermal monitoring operations.

use std::fmt;
use thiserror::Error;

/// Result type for thermal operations
pub type Result<T> = std::result::Result<T, ThermalError>;

/// Comprehensive error types for thermal monitoring
#[derive(Error, Debug, Clone)]
pub enum ThermalError {
    /// I/O error during thermal operations
    #[error("Thermal I/O error: {message}")]
    Io { message: String },

    /// Thermal sensor not found or unavailable
    #[error("Thermal sensor not found: {sensor_id}")]
    SensorNotFound { sensor_id: String },

    /// Configuration validation error
    #[error("Invalid thermal configuration: {field} - {reason}")]
    Config { field: String, reason: String },

    /// Parsing error for thermal data
    #[error("Failed to parse thermal data: {data_type} - {reason}")]
    Parse { data_type: String, reason: String },

    /// Permission denied for thermal operations
    #[error("Permission denied: {operation}")]
    Permission { operation: String },

    /// System resource unavailable
    #[error("System resource unavailable: {resource}")]
    Resource { resource: String },

    /// Thermal zone discovery failed
    #[error("Thermal discovery failed: {reason}")]
    Discovery { reason: String },

    /// Analytics computation error
    #[error("Thermal analytics error: {computation} - {reason}")]
    Analytics { computation: String, reason: String },

    /// Timeout during thermal operations
    #[error("Thermal operation timeout: {operation} after {duration_ms}ms")]
    Timeout { operation: String, duration_ms: u64 },

    /// Invalid thermal state
    #[error("Invalid thermal state: {state} - {reason}")]
    InvalidState { state: String, reason: String },

    /// Monitoring service error
    #[error("Thermal monitoring service error: {service} - {reason}")]
    Service { service: String, reason: String },

    /// Critical temperature condition
    #[error("Critical temperature: {temperature}°C on {sensor}")]
    CriticalTemperature { sensor: String, temperature: f64 },

    /// Thermal throttling detected
    #[error("Thermal throttling active: {reason}")]
    ThermalThrottling { reason: String },

    /// Fan control error
    #[error("Fan control error: {fan_id} - {reason}")]
    FanControl { fan_id: String, reason: String },

    /// Cooling system failure
    #[error("Cooling system failure: {system} - {reason}")]
    CoolingFailure { system: String, reason: String },

    /// Alert system error
    #[error("Alert system error: {alert_type} - {reason}")]
    AlertSystem { alert_type: String, reason: String },

    /// Prediction model error
    #[error("Thermal prediction error: {model} - {reason}")]
    Prediction { model: String, reason: String },
}

impl ThermalError {
    /// Create an I/O error
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
        }
    }

    /// Create a sensor not found error
    pub fn sensor_not_found(sensor_id: impl Into<String>) -> Self {
        Self::SensorNotFound {
            sensor_id: sensor_id.into(),
        }
    }

    /// Create a configuration error
    pub fn config(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Config {
            field: field.into(),
            reason: reason.into(),
        }
    }

    /// Create a parsing error
    pub fn parse(data_type: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Parse {
            data_type: data_type.into(),
            reason: reason.into(),
        }
    }

    /// Create a critical temperature error
    pub fn critical_temperature(sensor: impl Into<String>, temperature: f64) -> Self {
        Self::CriticalTemperature {
            sensor: sensor.into(),
            temperature,
        }
    }

    /// Create a thermal throttling error
    pub fn thermal_throttling(reason: impl Into<String>) -> Self {
        Self::ThermalThrottling {
            reason: reason.into(),
        }
    }

    /// Create a fan control error
    pub fn fan_control(fan_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::FanControl {
            fan_id: fan_id.into(),
            reason: reason.into(),
        }
    }

    /// Create a cooling failure error
    pub fn cooling_failure(system: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::CoolingFailure {
            system: system.into(),
            reason: reason.into(),
        }
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            ThermalError::Io { .. } => true,
            ThermalError::SensorNotFound { .. } => false,
            ThermalError::Config { .. } => false,
            ThermalError::Parse { .. } => true,
            ThermalError::Permission { .. } => false,
            ThermalError::Resource { .. } => true,
            ThermalError::Discovery { .. } => true,
            ThermalError::Analytics { .. } => true,
            ThermalError::Timeout { .. } => true,
            ThermalError::InvalidState { .. } => true,
            ThermalError::Service { .. } => true,
            ThermalError::CriticalTemperature { .. } => false, // Safety critical
            ThermalError::ThermalThrottling { .. } => false, // System protection
            ThermalError::FanControl { .. } => true,
            ThermalError::CoolingFailure { .. } => false, // Safety critical
            ThermalError::AlertSystem { .. } => true,
            ThermalError::Prediction { .. } => true,
        }
    }

    /// Check if error indicates safety-critical condition
    pub fn is_safety_critical(&self) -> bool {
        matches!(
            self,
            ThermalError::CriticalTemperature { .. } | 
            ThermalError::CoolingFailure { .. } |
            ThermalError::ThermalThrottling { .. }
        )
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            ThermalError::CriticalTemperature { .. } => ErrorSeverity::Critical,
            ThermalError::CoolingFailure { .. } => ErrorSeverity::Critical,
            ThermalError::ThermalThrottling { .. } => ErrorSeverity::High,
            ThermalError::FanControl { .. } => ErrorSeverity::High,
            ThermalError::Config { .. } => ErrorSeverity::Medium,
            ThermalError::Permission { .. } => ErrorSeverity::Medium,
            _ => ErrorSeverity::Low,
        }
    }

    /// Get error category for logging and monitoring
    pub fn category(&self) -> &'static str {
        match self {
            ThermalError::Io { .. } => "io",
            ThermalError::SensorNotFound { .. } => "sensor",
            ThermalError::Config { .. } => "config",
            ThermalError::Parse { .. } => "parse",
            ThermalError::Permission { .. } => "permission",
            ThermalError::Resource { .. } => "resource",
            ThermalError::Discovery { .. } => "discovery",
            ThermalError::Analytics { .. } => "analytics",
            ThermalError::Timeout { .. } => "timeout",
            ThermalError::InvalidState { .. } => "state",
            ThermalError::Service { .. } => "service",
            ThermalError::CriticalTemperature { .. } => "critical_temp",
            ThermalError::ThermalThrottling { .. } => "throttling",
            ThermalError::FanControl { .. } => "fan_control",
            ThermalError::CoolingFailure { .. } => "cooling_failure",
            ThermalError::AlertSystem { .. } => "alerts",
            ThermalError::Prediction { .. } => "prediction",
        }
    }

    /// Get suggested retry delay for recoverable errors
    pub fn retry_delay(&self) -> Option<std::time::Duration> {
        if !self.is_recoverable() {
            return None;
        }

        match self {
            ThermalError::Timeout { .. } => Some(std::time::Duration::from_millis(500)),
            ThermalError::Resource { .. } => Some(std::time::Duration::from_secs(1)),
            ThermalError::Service { .. } => Some(std::time::Duration::from_secs(2)),
            ThermalError::FanControl { .. } => Some(std::time::Duration::from_millis(100)),
            _ => Some(std::time::Duration::from_millis(250)),
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Low severity - informational
    Low,
    /// Medium severity - warning
    Medium,
    /// High severity - error
    High,
    /// Critical severity - system safety
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Low => write!(f, "LOW"),
            ErrorSeverity::Medium => write!(f, "MEDIUM"),
            ErrorSeverity::High => write!(f, "HIGH"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

impl From<std::io::Error> for ThermalError {
    fn from(err: std::io::Error) -> Self {
        ThermalError::io(err.to_string())
    }
}

impl From<std::num::ParseFloatError> for ThermalError {
    fn from(err: std::num::ParseFloatError) -> Self {
        ThermalError::parse("float", err.to_string())
    }
}

impl From<std::num::ParseIntError> for ThermalError {
    fn from(err: std::num::ParseIntError) -> Self {
        ThermalError::parse("integer", err.to_string())
    }
}

impl From<serde_json::Error> for ThermalError {
    fn from(err: serde_json::Error) -> Self {
        ThermalError::parse("json", err.to_string())
    }
}

/// Error recovery strategy for thermal operations
#[derive(Debug, Clone)]
pub struct RecoveryStrategy {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Base delay between retries
    pub base_delay: std::time::Duration,
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    /// Maximum delay between retries
    pub max_delay: std::time::Duration,
    /// Enable emergency cooling protocols
    pub emergency_cooling: bool,
    /// Emergency shutdown temperature
    pub emergency_shutdown_temp: f64,
}

impl Default for RecoveryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: std::time::Duration::from_millis(100),
            backoff_multiplier: 2.0,
            max_delay: std::time::Duration::from_secs(5),
            emergency_cooling: true,
            emergency_shutdown_temp: 100.0, // 100°C emergency shutdown
        }
    }
}

impl RecoveryStrategy {
    /// Calculate delay for a specific retry attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> std::time::Duration {
        if attempt == 0 {
            return self.base_delay;
        }

        let delay_ms = (self.base_delay.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32)) as u64;
        
        let delay = std::time::Duration::from_millis(delay_ms);
        std::cmp::min(delay, self.max_delay)
    }

    /// Check if we should retry for the given attempt number
    pub fn should_retry(&self, attempt: u32, error: &ThermalError) -> bool {
        // Never retry safety-critical errors
        if error.is_safety_critical() {
            return false;
        }

        attempt < self.max_retries && error.is_recoverable()
    }

    /// Handle emergency thermal conditions
    pub fn handle_emergency(&self, temperature: f64) -> Result<()> {
        if temperature >= self.emergency_shutdown_temp {
            eprintln!("EMERGENCY: Temperature {}°C exceeds shutdown threshold {}°C", 
                     temperature, self.emergency_shutdown_temp);
            
            if self.emergency_cooling {
                // In a real implementation, this would trigger emergency cooling
                // or system shutdown protocols
                eprintln!("Activating emergency cooling protocols");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = ThermalError::critical_temperature("cpu", 95.0);
        assert_eq!(err.category(), "critical_temp");
        assert!(!err.is_recoverable());
        assert!(err.is_safety_critical());
        assert_eq!(err.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_error_severity() {
        let critical = ThermalError::critical_temperature("cpu", 100.0);
        let config = ThermalError::config("interval", "invalid value");
        let io = ThermalError::io("read failed");

        assert_eq!(critical.severity(), ErrorSeverity::Critical);
        assert_eq!(config.severity(), ErrorSeverity::Medium);
        assert_eq!(io.severity(), ErrorSeverity::Low);
    }

    #[test]
    fn test_recovery_strategy() {
        let strategy = RecoveryStrategy::default();
        let recoverable_err = ThermalError::io("temporary failure");
        let critical_err = ThermalError::critical_temperature("cpu", 95.0);

        assert!(strategy.should_retry(0, &recoverable_err));
        assert!(!strategy.should_retry(0, &critical_err));
        assert!(!strategy.should_retry(5, &recoverable_err)); // Too many attempts

        let delay1 = strategy.delay_for_attempt(1);
        let delay2 = strategy.delay_for_attempt(2);
        assert!(delay2 > delay1);
    }

    #[test]
    fn test_error_conversions() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "sensor not found");
        let thermal_err: ThermalError = io_err.into();
        assert!(matches!(thermal_err, ThermalError::Io { .. }));
    }
}