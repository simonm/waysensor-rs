//! NVIDIA GPU monitoring sensor for waysensor-rs.
//!
//! This module provides NVIDIA GPU monitoring by parsing nvidia-smi output
//! and extracting key metrics like temperature, utilization, memory usage, and power.

pub mod nvidia_gpu;

pub use nvidia_gpu::NvidiaGpuSensor;