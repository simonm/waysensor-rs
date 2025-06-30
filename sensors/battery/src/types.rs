//! Core types and data structures for battery monitoring.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

/// Temperature thresholds (Celsius)
pub const CRITICAL_TEMPERATURE_THRESHOLD: f64 = 60.0;
pub const WARNING_TEMPERATURE_THRESHOLD: f64 = 45.0;

/// Comprehensive battery metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryMetrics {
    /// Battery information
    pub info: BatteryInfo,
    /// Current battery state
    pub state: BatteryState,
    /// Energy metrics
    pub energy: EnergyMetrics,
    /// Health metrics
    pub health: BatteryHealth,
    /// Thermal state
    pub thermal: ThermalState,
    /// Power profile information
    pub power_profile: PowerProfile,
    /// Measurement timestamp
    pub timestamp: DateTime<Utc>,
    /// Time since last measurement
    pub measurement_duration: Duration,
}

/// Battery information and specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    /// Battery identifier
    pub id: String,
    /// Manufacturer
    pub manufacturer: Option<String>,
    /// Model name
    pub model: Option<String>,
    /// Serial number
    pub serial_number: Option<String>,
    /// Technology type (Li-ion, Li-Po, etc.)
    pub technology: BatteryTechnology,
    /// Design capacity (mWh)
    pub design_capacity: f64,
    /// Design voltage (V)
    pub design_voltage: f64,
    /// Manufacture date
    pub manufacture_date: Option<DateTime<Utc>>,
    /// Cycle count
    pub cycle_count: Option<u32>,
}

/// Battery technology types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatteryTechnology {
    /// Lithium-ion
    LithiumIon,
    /// Lithium Polymer
    LithiumPolymer,
    /// Nickel Metal Hydride
    NickelMetalHydride,
    /// Nickel Cadmium
    NickelCadmium,
    /// Lead Acid
    LeadAcid,
    /// Unknown technology
    Unknown,
}

impl fmt::Display for BatteryTechnology {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BatteryTechnology::LithiumIon => write!(f, "Li-ion"),
            BatteryTechnology::LithiumPolymer => write!(f, "Li-Po"),
            BatteryTechnology::NickelMetalHydride => write!(f, "NiMH"),
            BatteryTechnology::NickelCadmium => write!(f, "NiCd"),
            BatteryTechnology::LeadAcid => write!(f, "Lead Acid"),
            BatteryTechnology::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Current battery state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryState {
    /// Current charge level (0.0 to 1.0)
    pub charge_level: f64,
    /// Charging state
    pub charging_state: ChargingState,
    /// Current voltage (V)
    pub voltage: f64,
    /// Current amperage (A) - positive for charging, negative for discharging
    pub current: f64,
    /// Power consumption/generation (W)
    pub power: f64,
    /// Estimated time remaining
    pub time_remaining: Option<Duration>,
    /// Battery present flag
    pub present: bool,
    /// AC adapter connected
    pub ac_connected: bool,
}

impl BatteryState {
    /// Get charge percentage
    pub fn charge_percentage(&self) -> f64 {
        (self.charge_level * 100.0).clamp(0.0, 100.0)
    }

    /// Check if battery is charging
    pub fn is_charging(&self) -> bool {
        matches!(self.charging_state, ChargingState::Charging)
    }

    /// Check if battery is discharging
    pub fn is_discharging(&self) -> bool {
        matches!(self.charging_state, ChargingState::Discharging)
    }

    /// Check if battery is full
    pub fn is_full(&self) -> bool {
        matches!(self.charging_state, ChargingState::Full) || self.charge_level >= 0.99
    }

    /// Check if battery is critically low
    pub fn is_critical(&self) -> bool {
        self.charge_level <= 0.05 // 5% or below
    }

    /// Check if battery is in warning state
    pub fn is_warning(&self) -> bool {
        self.charge_level <= 0.15 && !self.is_critical() // 15% or below but not critical
    }

    /// Get power consumption rate (W)
    pub fn power_consumption(&self) -> f64 {
        -self.power.min(0.0) // Convert negative power to positive consumption
    }

    /// Get charging rate (W)
    pub fn charging_rate(&self) -> f64 {
        self.power.max(0.0) // Positive power indicates charging
    }
}

/// Charging state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChargingState {
    /// Battery is charging
    Charging,
    /// Battery is discharging
    Discharging,
    /// Battery is full
    Full,
    /// Battery state is unknown
    Unknown,
    /// Battery is not charging (AC connected but not charging)
    NotCharging,
}

impl fmt::Display for ChargingState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChargingState::Charging => write!(f, "Charging"),
            ChargingState::Discharging => write!(f, "Discharging"),
            ChargingState::Full => write!(f, "Full"),
            ChargingState::Unknown => write!(f, "Unknown"),
            ChargingState::NotCharging => write!(f, "Not Charging"),
        }
    }
}

/// Energy-related metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyMetrics {
    /// Current capacity (mWh)
    pub current_capacity: f64,
    /// Full charge capacity (mWh)
    pub full_capacity: f64,
    /// Energy consumed since last charge (mWh)
    pub energy_consumed: f64,
    /// Energy rate (mW) - average over measurement period
    pub energy_rate: f64,
    /// Efficiency percentage (energy out / energy in)
    pub efficiency: Option<f64>,
    /// Charge cycles completed
    pub charge_cycles: f64,
}

impl EnergyMetrics {
    /// Calculate capacity utilization (current/full)
    pub fn capacity_utilization(&self) -> f64 {
        if self.full_capacity > 0.0 {
            self.current_capacity / self.full_capacity
        } else {
            0.0
        }
    }

    /// Calculate energy remaining (mWh)
    pub fn energy_remaining(&self) -> f64 {
        self.full_capacity - self.current_capacity
    }

    /// Estimate time to full charge (based on current rate)
    pub fn time_to_full(&self) -> Option<Duration> {
        if self.energy_rate > 0.0 {
            let hours = self.energy_remaining() / self.energy_rate;
            Some(Duration::from_secs_f64(hours * 3600.0))
        } else {
            None
        }
    }

    /// Estimate time to empty (based on current rate)
    pub fn time_to_empty(&self) -> Option<Duration> {
        if self.energy_rate < 0.0 {
            let hours = self.current_capacity / (-self.energy_rate);
            Some(Duration::from_secs_f64(hours * 3600.0))
        } else {
            None
        }
    }
}

/// Battery health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryHealth {
    /// Health percentage (0.0 to 1.0)
    pub health_percentage: f64,
    /// Capacity degradation rate (% per cycle)
    pub degradation_rate: f64,
    /// Estimated remaining life (cycles)
    pub estimated_life_cycles: Option<u32>,
    /// Estimated remaining life (time)
    pub estimated_life_duration: Option<Duration>,
    /// Health status
    pub health_status: HealthStatus,
    /// Internal resistance (mΩ)
    pub internal_resistance: Option<f64>,
    /// Voltage sag under load (V)
    pub voltage_sag: Option<f64>,
}

impl BatteryHealth {
    /// Check if battery health is good (>80%)
    pub fn is_healthy(&self) -> bool {
        self.health_percentage > 0.8
    }

    /// Check if battery health is poor (<70%)
    pub fn is_poor(&self) -> bool {
        self.health_percentage < 0.7
    }

    /// Get health rating
    pub fn health_rating(&self) -> &'static str {
        if self.health_percentage >= 0.9 {
            "Excellent"
        } else if self.health_percentage >= 0.8 {
            "Good"
        } else if self.health_percentage >= 0.7 {
            "Fair"
        } else if self.health_percentage >= 0.5 {
            "Poor"
        } else {
            "Critical"
        }
    }
}

/// Health status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Battery health is excellent
    Excellent,
    /// Battery health is good
    Good,
    /// Battery health is fair
    Fair,
    /// Battery health is poor
    Poor,
    /// Battery health is critical
    Critical,
    /// Health status unknown
    Unknown,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Excellent => write!(f, "Excellent"),
            HealthStatus::Good => write!(f, "Good"),
            HealthStatus::Fair => write!(f, "Fair"),
            HealthStatus::Poor => write!(f, "Poor"),
            HealthStatus::Critical => write!(f, "Critical"),
            HealthStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Thermal state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalState {
    /// Current temperature (°C)
    pub temperature: f64,
    /// Thermal zone
    pub thermal_zone: ThermalZone,
}

impl ThermalState {
    /// Check if temperature is in safe range
    pub fn is_safe_temperature(&self) -> bool {
        self.temperature < WARNING_TEMPERATURE_THRESHOLD
    }

    /// Check if temperature is critical
    pub fn is_critical_temperature(&self) -> bool {
        self.temperature >= CRITICAL_TEMPERATURE_THRESHOLD
    }

    /// Get temperature status
    pub fn temperature_status(&self) -> TemperatureStatus {
        if self.is_critical_temperature() {
            TemperatureStatus::Critical
        } else if self.temperature >= WARNING_TEMPERATURE_THRESHOLD {
            TemperatureStatus::Warning
        } else {
            TemperatureStatus::Normal
        }
    }
}


/// Thermal zone classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThermalZone {
    /// Safe operating zone
    Safe,
    /// Warning zone
    Warning,
    /// Critical zone
    Critical,
}

/// Temperature reading with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureReading {
    /// Temperature value (°C)
    pub temperature: f64,
    /// Reading timestamp
    pub timestamp: DateTime<Utc>,
}

/// Temperature status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemperatureStatus {
    /// Normal temperature
    Normal,
    /// Warning temperature
    Warning,
    /// Critical temperature
    Critical,
}

/// Power profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerProfile {
    /// Current power profile name
    pub profile_name: String,
    /// CPU frequency scaling
    pub cpu_scaling: CpuScaling,
    /// Display brightness (0.0 to 1.0)
    pub display_brightness: f64,
    /// Power saving features enabled
    pub power_saving_enabled: bool,
    /// Estimated impact on battery life
    pub battery_life_impact: PowerImpact,
    /// Profile optimization recommendations
    pub recommendations: Vec<String>,
}

/// CPU frequency scaling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CpuScaling {
    /// Performance mode
    Performance,
    /// Balanced mode
    Balanced,
    /// Power saving mode
    PowerSave,
    /// On-demand scaling
    OnDemand,
    /// Conservative scaling
    Conservative,
}

/// Power impact assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerImpact {
    /// Positive impact (extends battery life)
    Positive,
    /// Neutral impact
    Neutral,
    /// Negative impact (reduces battery life)
    Negative,
    /// Unknown impact
    Unknown,
}

/// Multi-battery system metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiBatteryMetrics {
    /// Individual battery metrics
    pub batteries: HashMap<String, BatteryMetrics>,
    /// Combined system metrics
    pub system: SystemPowerMetrics,
    /// Power distribution information
    pub power_distribution: PowerDistribution,
    /// Overall health assessment
    pub overall_health: f64,
    /// System power profile
    pub system_profile: PowerProfile,
}

/// System-wide power metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPowerMetrics {
    /// Total charge level (weighted average)
    pub total_charge_level: f64,
    /// Total capacity (mWh)
    pub total_capacity: f64,
    /// Total power consumption (W)
    pub total_power: f64,
    /// Estimated system runtime
    pub estimated_runtime: Option<Duration>,
    /// Number of batteries present
    pub battery_count: usize,
    /// Number of healthy batteries
    pub healthy_battery_count: usize,
}

/// Power distribution across batteries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerDistribution {
    /// Load balancing efficiency
    pub load_balance_efficiency: f64,
    /// Battery utilization distribution
    pub utilization_distribution: HashMap<String, f64>,
    /// Charge balancing status
    pub charge_balance_status: ChargeBalanceStatus,
}

/// Charge balance status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChargeBalanceStatus {
    /// Batteries are well balanced
    Balanced,
    /// Minor imbalance
    MinorImbalance,
    /// Significant imbalance
    MajorImbalance,
    /// Critical imbalance
    CriticalImbalance,
}

/// Historical data point for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint<T> {
    /// Data value
    pub value: T,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl<T> DataPoint<T> {
    /// Create new data point
    pub fn new(value: T) -> Self {
        Self {
            value,
            timestamp: Utc::now(),
        }
    }

    /// Get age of data point
    pub fn age(&self) -> Duration {
        let now = Utc::now();
        (now - self.timestamp).to_std().unwrap_or_default()
    }

    /// Check if data point is fresh
    pub fn is_fresh(&self, max_age: Duration) -> bool {
        self.age() < max_age
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_state() {
        let state = BatteryState {
            charge_level: 0.75,
            charging_state: ChargingState::Discharging,
            voltage: 11.4,
            current: -2.5,
            power: -28.5,
            time_remaining: Some(Duration::from_secs(7200)),
            present: true,
            ac_connected: false,
        };

        assert_eq!(state.charge_percentage(), 75.0);
        assert!(!state.is_charging());
        assert!(state.is_discharging());
        assert!(!state.is_critical());
        assert_eq!(state.power_consumption(), 28.5);
    }

    #[test]
    fn test_battery_health() {
        let health = BatteryHealth {
            health_percentage: 0.85,
            degradation_rate: 0.02,
            estimated_life_cycles: Some(500),
            estimated_life_duration: Some(Duration::from_secs(365 * 24 * 3600)),
            health_status: HealthStatus::Good,
            internal_resistance: Some(150.0),
            voltage_sag: Some(0.2),
        };

        assert!(health.is_healthy());
        assert!(!health.is_poor());
        assert_eq!(health.health_rating(), "Good");
    }

    #[test]
    fn test_thermal_state() {
        let thermal = ThermalState {
            temperature: 42.0,
            thermal_zone: ThermalZone::Safe,
        };

        assert!(thermal.is_safe_temperature());
        assert!(!thermal.is_critical_temperature());
        assert_eq!(thermal.temperature_status(), TemperatureStatus::Normal);
    }

    #[test]
    fn test_energy_metrics() {
        let energy = EnergyMetrics {
            current_capacity: 45000.0,
            full_capacity: 50000.0,
            energy_consumed: 5000.0,
            energy_rate: -10000.0, // 10W consumption
            efficiency: Some(0.92),
            charge_cycles: 250.5,
        };

        assert_eq!(energy.capacity_utilization(), 0.9);
        assert_eq!(energy.energy_remaining(), 5000.0);

        let time_to_empty = energy.time_to_empty().unwrap();
        assert!(time_to_empty.as_secs() > 0);
    }

    #[test]
    fn test_data_point() {
        let point = DataPoint::new(42.0);
        assert!(point.is_fresh(Duration::from_secs(1)));
        assert_eq!(point.value, 42.0);
    }
}