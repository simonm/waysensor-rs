//! Memory usage monitoring for waysensor-rs.
//!
//! This crate provides memory usage monitoring capabilities for the waysensor-rs
//! system monitoring suite. It reads memory statistics from `/proc/meminfo` and
//! calculates usage percentages suitable for display in Waybar.
//!
//! # Examples
//!
//! ```rust
//! use waysensor_memory::MemorySensor;
//! use waysensor_rs_core::Sensor;
//!
//! // Create a memory sensor with 70% warning and 90% critical thresholds
//! let mut sensor = MemorySensor::new(70, 90, false, false)?;
//!
//! // Read current memory usage
//! let output = sensor.read()?;
//! println!("Memory usage: {}", output.text);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod memory;

pub use memory::{MemoryInfo, MemorySensor};