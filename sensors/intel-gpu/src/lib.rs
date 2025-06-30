//! Intel GPU monitoring sensor for waysensor-rs.
//!
//! This module provides Intel GPU monitoring by reading from Linux sysfs
//! and DRM interfaces to extract GPU frequency, power, and utilization metrics.

pub mod intel_gpu;

pub use intel_gpu::IntelGpuSensor;