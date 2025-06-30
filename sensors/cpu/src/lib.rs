//! CPU usage monitoring for waysensor-rs.
//!
//! This crate provides CPU usage monitoring capabilities for the waysensor-rs
//! system monitoring suite. It reads CPU statistics from `/proc/stat` and
//! calculates usage percentages suitable for display in Waybar.
//!
//! # Examples
//!
//! ```rust
//! use waysensor_cpu::CpuSensor;
//! use waysensor_rs_core::Sensor;
//!
//! // Create a CPU sensor with 70% warning and 90% critical thresholds
//! let mut sensor = CpuSensor::new(70, 90)?;
//!
//! // Read current CPU usage
//! let output = sensor.read()?;
//! println!("CPU usage: {}", output.text);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod cpu;

pub use cpu::{CpuInfo, CpuSensor, CpuStats};