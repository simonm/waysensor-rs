//! Error handling for battery monitoring operations.

use thiserror::Error;

/// Result type for battery operations
pub type Result<T> = std::result::Result<T, BatteryError>;

/// Comprehensive error types for battery monitoring
#[derive(Error, Debug, Clone)]
pub enum BatteryError {
    /// I/O error during battery operations
    #[error("Battery I/O error: {message}")]
    Io { message: String },

    /// Battery not found or unavailable
    #[error("Battery not found: {battery_id}")]
    BatteryNotFound { battery_id: String },

    /// Configuration validation error
    #[error("Invalid configuration: {field} - {reason}")]
    Config { field: String, reason: String },

    /// Parsing error for battery data
    #[error("Failed to parse battery data: {data_type} - {reason}")]
    Parse { data_type: String, reason: String },

    /// Permission denied for battery operations
    #[error("Permission denied: {operation}")]
    Permission { operation: String },

    /// System resource unavailable
    #[error("System resource unavailable: {resource}")]
    Resource { resource: String },

    /// Battery discovery failed
    #[error("Battery discovery failed: {reason}")]
    Discovery { reason: String },

    /// Analytics computation error
    #[error("Analytics error: {computation} - {reason}")]
    Analytics { computation: String, reason: String },

    /// Timeout during battery operations
    #[error("Battery operation timeout: {operation} after {duration_ms}ms")]
    Timeout { operation: String, duration_ms: u64 },

    /// Invalid battery state
    #[error("Invalid battery state: {state} - {reason}")]
    InvalidState { state: String, reason: String },

    /// Monitoring service error
    #[error("Monitoring service error: {service} - {reason}")]
    Service { service: String, reason: String },

    /// Thermal protection error
    #[error("Thermal protection: {reason}")]
    Thermal { reason: String },

    /// Power management error
    #[error("Power management error: {operation} - {reason}")]
    PowerManagement { operation: String, reason: String },

    /// Health analysis error
    #[error("Health analysis error: {reason}")]
    Health { reason: String },

    /// Prediction model error
    #[error("Prediction model error: {model} - {reason}")]
    Prediction { model: String, reason: String },
}

impl BatteryError {
    /// Create an I/O error
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
        }
    }

    /// Create a battery not found error
    pub fn battery_not_found(battery_id: impl Into<String>) -> Self {
        Self::BatteryNotFound {
            battery_id: battery_id.into(),
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

    /// Create a permission error
    pub fn permission(operation: impl Into<String>) -> Self {
        Self::Permission {
            operation: operation.into(),
        }
    }

    /// Create a resource error
    pub fn resource(resource: impl Into<String>) -> Self {
        Self::Resource {
            resource: resource.into(),
        }
    }

    /// Create a discovery error
    pub fn discovery(reason: impl Into<String>) -> Self {
        Self::Discovery {
            reason: reason.into(),
        }
    }

    /// Create an analytics error
    pub fn analytics(computation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Analytics {
            computation: computation.into(),
            reason: reason.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(operation: impl Into<String>, duration_ms: u64) -> Self {
        Self::Timeout {
            operation: operation.into(),
            duration_ms,
        }
    }

    /// Create an invalid state error
    pub fn invalid_state(state: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidState {
            state: state.into(),
            reason: reason.into(),
        }
    }

    /// Create a service error
    pub fn service(service: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Service {
            service: service.into(),
            reason: reason.into(),
        }
    }

    /// Create a thermal error
    pub fn thermal(reason: impl Into<String>) -> Self {
        Self::Thermal {
            reason: reason.into(),
        }
    }

    /// Create a power management error
    pub fn power_management(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::PowerManagement {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Create a health error
    pub fn health(reason: impl Into<String>) -> Self {
        Self::Health {
            reason: reason.into(),
        }
    }

    /// Create a prediction error
    pub fn prediction(model: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Prediction {
            model: model.into(),
            reason: reason.into(),
        }
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            BatteryError::Io { .. } => true,
            BatteryError::BatteryNotFound { .. } => false,
            BatteryError::Config { .. } => false,
            BatteryError::Parse { .. } => true,
            BatteryError::Permission { .. } => false,
            BatteryError::Resource { .. } => true,
            BatteryError::Discovery { .. } => true,
            BatteryError::Analytics { .. } => true,
            BatteryError::Timeout { .. } => true,
            BatteryError::InvalidState { .. } => true,
            BatteryError::Service { .. } => true,
            BatteryError::Thermal { .. } => false, // Safety critical
            BatteryError::PowerManagement { .. } => true,
            BatteryError::Health { .. } => true,
            BatteryError::Prediction { .. } => true,
        }
    }

    /// Get error category for logging and monitoring
    pub fn category(&self) -> &'static str {
        match self {
            BatteryError::Io { .. } => "io",
            BatteryError::BatteryNotFound { .. } => "not_found",
            BatteryError::Config { .. } => "config",
            BatteryError::Parse { .. } => "parse",
            BatteryError::Permission { .. } => "permission",
            BatteryError::Resource { .. } => "resource",
            BatteryError::Discovery { .. } => "discovery",
            BatteryError::Analytics { .. } => "analytics",
            BatteryError::Timeout { .. } => "timeout",
            BatteryError::InvalidState { .. } => "state",
            BatteryError::Service { .. } => "service",
            BatteryError::Thermal { .. } => "thermal",
            BatteryError::PowerManagement { .. } => "power",
            BatteryError::Health { .. } => "health",
            BatteryError::Prediction { .. } => "prediction",
        }
    }

    /// Get suggested retry delay for recoverable errors
    pub fn retry_delay(&self) -> Option<std::time::Duration> {
        if !self.is_recoverable() {
            return None;
        }

        match self {
            BatteryError::Timeout { .. } => Some(std::time::Duration::from_millis(500)),
            BatteryError::Resource { .. } => Some(std::time::Duration::from_secs(1)),
            BatteryError::Service { .. } => Some(std::time::Duration::from_secs(2)),
            BatteryError::Analytics { .. } => Some(std::time::Duration::from_millis(100)),
            _ => Some(std::time::Duration::from_millis(250)),
        }
    }

    /// Check if error indicates critical safety condition
    pub fn is_safety_critical(&self) -> bool {
        matches!(self, BatteryError::Thermal { .. })
    }
}

impl From<std::io::Error> for BatteryError {
    fn from(err: std::io::Error) -> Self {
        BatteryError::io(err.to_string())
    }
}

impl From<std::num::ParseIntError> for BatteryError {
    fn from(err: std::num::ParseIntError) -> Self {
        BatteryError::parse("integer", err.to_string())
    }
}

impl From<std::num::ParseFloatError> for BatteryError {
    fn from(err: std::num::ParseFloatError) -> Self {
        BatteryError::parse("float", err.to_string())
    }
}

impl From<serde_json::Error> for BatteryError {
    fn from(err: serde_json::Error) -> Self {
        BatteryError::parse("json", err.to_string())
    }
}

/// Error recovery strategy for battery operations
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
    /// Enable safety-critical error handling
    pub safety_critical_handling: bool,
}

impl Default for RecoveryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: std::time::Duration::from_millis(200),
            backoff_multiplier: 2.0,
            max_delay: std::time::Duration::from_secs(10),
            safety_critical_handling: true,
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
    pub fn should_retry(&self, attempt: u32, error: &BatteryError) -> bool {
        // Never retry safety-critical errors
        if self.safety_critical_handling && error.is_safety_critical() {
            return false;
        }

        attempt < self.max_retries && error.is_recoverable()
    }

    /// Handle safety-critical errors
    pub fn handle_safety_critical(&self, error: &BatteryError) -> Result<()> {
        if error.is_safety_critical() && self.safety_critical_handling {
            // Log critical error and potentially take protective action
            eprintln!("CRITICAL BATTERY ERROR: {}", error);
            // In a real implementation, this might trigger system shutdown
            // or other protective measures
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = BatteryError::battery_not_found("BAT0");
        assert_eq!(err.category(), "not_found");
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_safety_critical() {
        let thermal_err = BatteryError::thermal("overheating");
        assert!(thermal_err.is_safety_critical());
        assert!(!thermal_err.is_recoverable());

        let io_err = BatteryError::io("read failed");
        assert!(!io_err.is_safety_critical());
        assert!(io_err.is_recoverable());
    }

    #[test]
    fn test_recovery_strategy() {
        let strategy = RecoveryStrategy::default();
        let recoverable_err = BatteryError::io("temporary failure");
        let safety_err = BatteryError::thermal("overheating");

        assert!(strategy.should_retry(0, &recoverable_err));
        assert!(!strategy.should_retry(0, &safety_err));
        assert!(!strategy.should_retry(5, &recoverable_err)); // Too many attempts

        let delay1 = strategy.delay_for_attempt(1);
        let delay2 = strategy.delay_for_attempt(2);
        assert!(delay2 > delay1);
    }

    #[test]
    fn test_error_conversions() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let battery_err: BatteryError = io_err.into();
        assert!(matches!(battery_err, BatteryError::Io { .. }));
    }
}