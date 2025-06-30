//! # waysensor-rs-disk
//!
//! Advanced disk monitoring library for the waysensor-rs sensor suite with sophisticated
//! multi-disk support, performance tracking, and predictive analytics.

mod disk;
mod multi_disk;

pub use disk::{DiskSensor, DiskSensorBuilder, DiskError, CacheConfig};
pub use multi_disk::{MultiDiskSensor, DisplayMode};