//! GPU metrics parsing and caching with support for all AMD GPU metrics versions.

use waysensor_rs_core::SensorError;
use std::{
    fmt,
    fs::File,
    io::Read,
    path::Path,
    time::{Duration, Instant},
    collections::HashMap,
    sync::{Arc, Mutex},
};
use memmap2::MmapOptions;

/// Header for GPU metrics structure with version information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub structure_size: u16,
    pub format_revision: u8,
    pub content_revision: u8,
}

impl Header {
    /// Get version string in format "vX.Y".
    pub fn version(&self) -> String {
        format!("v{}.{}", self.format_revision, self.content_revision)
    }
    
    /// Check if this version is supported.
    pub fn is_supported(&self) -> bool {
        matches!(
            (self.format_revision, self.content_revision),
            (1, 0..=3) | (2, 0..=1) | (3, 0)
        )
    }
    
    /// Get expected structure size for this version.
    pub fn expected_size(&self) -> Option<usize> {
        match (self.format_revision, self.content_revision) {
            (1, 0) => Some(96),   // v1.0
            (1, 1) => Some(100),  // v1.1
            (1, 2) => Some(104),  // v1.2
            (1, 3) => Some(108),  // v1.3
            (2, 0) => Some(120),  // v2.0
            (2, 1) => Some(124),  // v2.1
            _ => None,
        }
    }
}

/// Trait for all GPU metrics versions with comprehensive data access.
pub trait GpuMetrics: fmt::Debug + Send + Sync {
    /// Get primary GPU temperature and its source label.
    fn get_temperature(&self) -> (u16, String);
    
    /// Get all available temperature readings.
    fn get_all_temperatures(&self) -> Vec<(String, u16)>;
    
    /// Get current power consumption in watts.
    fn get_power(&self) -> u16;
    
    /// Get detailed power breakdown if available.
    fn get_power_breakdown(&self) -> HashMap<String, u16>;
    
    /// Get GPU activity percentage (0-100).
    fn get_activity(&self) -> u16;
    
    /// Get detailed activity breakdown if available.
    fn get_activity_breakdown(&self) -> HashMap<String, u16>;
    
    /// Get primary GPU frequency in MHz.
    fn get_frequency(&self) -> u16;
    
    /// Get all frequency domains.
    fn get_all_frequencies(&self) -> HashMap<String, u16>;
    
    /// Get throttle status bitmask.
    fn get_throttle_status(&self) -> u64;
    
    /// Get fan speed information.
    fn get_fan_speed(&self) -> (u16, bool);
    
    /// Get header information.
    fn get_header(&self) -> Header;
    
    /// Get memory information if available.
    fn get_memory_info(&self) -> Option<MemoryInfo>;
    
    /// Get voltage information if available.
    fn get_voltage_info(&self) -> Option<VoltageInfo>;
    
    /// Get system clock counter for precise timing.
    fn get_system_clock_counter(&self) -> u64;
    
    /// Calculate power efficiency (performance per watt).
    fn get_power_efficiency(&self) -> f64 {
        let activity = self.get_activity() as f64;
        let power = self.get_power() as f64;
        if power > 0.0 {
            activity / power
        } else {
            0.0
        }
    }
    
    /// Calculate thermal efficiency (performance per degree).
    fn get_thermal_efficiency(&self) -> f64 {
        let activity = self.get_activity() as f64;
        let temp = self.get_temperature().0 as f64;
        if temp > 0.0 {
            activity / temp
        } else {
            0.0
        }
    }
}

/// Memory information structure.
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub total_vram: Option<u64>,
    pub used_vram: Option<u64>,
    pub memory_frequency: Option<u16>,
    pub memory_utilization: Option<u16>,
}

/// Voltage information structure.
#[derive(Debug, Clone)]
pub struct VoltageInfo {
    pub core_voltage: Option<u16>,
    pub soc_voltage: Option<u16>,
    pub memory_voltage: Option<u16>,
}

/// Throttle status with detailed analysis.
#[derive(Debug, Clone, Copy)]
pub struct ThrottleStatus(pub u64);

impl ThrottleStatus {
    const THROTTLE_FLAGS: &'static [(&'static str, u64)] = &[
        ("PROCHOT_CPU", 1 << 0),
        ("PROCHOT_GFX", 1 << 1),
        ("PPT0", 1 << 16),
        ("PPT1", 1 << 17),
        ("PPT2", 1 << 18),
        ("PPT3", 1 << 19),
        ("SPL", 1 << 20),
        ("FPPT", 1 << 21),
        ("SPPT", 1 << 22),
        ("SPPT_APU", 1 << 23),
        ("THM_CORE", 1 << 32),
        ("THM_GFX", 1 << 33),
        ("THM_SOC", 1 << 34),
    ];

    /// Check if any throttling is active.
    pub fn is_throttling(&self) -> bool {
        self.0 != 0
    }

    /// Get list of active throttle flags.
    pub fn active_flags(&self) -> Vec<String> {
        let mut flags = Vec::new();
        for (name, flag) in Self::THROTTLE_FLAGS {
            if self.0 & flag != 0 {
                flags.push(name.to_string());
            }
        }
        flags
    }
    
    /// Get throttle severity (0.0 = none, 1.0 = maximum).
    pub fn severity(&self) -> f64 {
        let active_count = self.active_flags().len();
        let total_flags = Self::THROTTLE_FLAGS.len();
        active_count as f64 / total_flags as f64
    }
    
    /// Check if thermal throttling is active.
    pub fn is_thermal_throttling(&self) -> bool {
        const THERMAL_FLAGS: u64 = (1 << 32) | (1 << 33) | (1 << 34);
        self.0 & THERMAL_FLAGS != 0
    }
    
    /// Check if power throttling is active.
    pub fn is_power_throttling(&self) -> bool {
        const POWER_FLAGS: u64 = (1 << 16) | (1 << 17) | (1 << 18) | (1 << 19) | (1 << 20) | (1 << 21) | (1 << 22) | (1 << 23);
        self.0 & POWER_FLAGS != 0
    }
}

// GPU Metrics v1.x implementation
#[derive(Debug, Clone)]
pub struct GpuMetricsV1 {
    pub header: Header,
    pub system_clock_counter: u64,
    pub temperature_edge: u16,
    pub temperature_hotspot: u16,
    pub temperature_mem: u16,
    pub temperature_vrgfx: u16,
    pub temperature_vrsoc: u16,
    pub temperature_vrmem: u16,
    pub average_gfx_activity: u16,
    pub average_umc_activity: u16,
    pub average_mm_activity: u16,
    pub average_socket_power: u16,
    pub energy_accumulator: u64,
    pub average_gfxclk_frequency: u16,
    pub average_socclk_frequency: u16,
    pub average_uclk_frequency: u16,
    pub average_vclk0_frequency: u16,
    pub average_dclk0_frequency: u16,
    pub average_vclk1_frequency: u16,
    pub average_dclk1_frequency: u16,
    pub current_gfxclk: u16,
    pub current_socclk: u16,
    pub current_uclk: u16,
    pub current_vclk0: u16,
    pub current_dclk0: u16,
    pub current_vclk1: u16,
    pub current_dclk1: u16,
    pub throttle_status: u64,
    pub current_fan_speed: u16,
    pub pcie_link_width: u16,
    pub pcie_link_speed: u16,
    // v1.1+ fields
    pub gfx_voltage: Option<u16>,
    pub soc_voltage: Option<u16>,
    // v1.2+ fields
    pub mem_voltage: Option<u16>,
    pub indep_throttle_status: Option<u64>,
    // v1.3+ fields
    pub current_socket_power: Option<u16>,
    pub vcn_activity: Option<[u16; 4]>,
}

impl GpuMetrics for GpuMetricsV1 {
    fn get_temperature(&self) -> (u16, String) {
        (self.temperature_edge, "Edge".to_string())
    }
    
    fn get_all_temperatures(&self) -> Vec<(String, u16)> {
        vec![
            ("Edge".to_string(), self.temperature_edge),
            ("Hotspot".to_string(), self.temperature_hotspot),
            ("Memory".to_string(), self.temperature_mem),
            ("VR GFX".to_string(), self.temperature_vrgfx),
            ("VR SOC".to_string(), self.temperature_vrsoc),
            ("VR Mem".to_string(), self.temperature_vrmem),
        ]
    }

    fn get_power(&self) -> u16 {
        self.current_socket_power.unwrap_or(self.average_socket_power)
    }
    
    fn get_power_breakdown(&self) -> HashMap<String, u16> {
        let mut breakdown = HashMap::new();
        breakdown.insert("Socket".to_string(), self.average_socket_power);
        if let Some(current) = self.current_socket_power {
            breakdown.insert("Current Socket".to_string(), current);
        }
        breakdown
    }

    fn get_activity(&self) -> u16 {
        self.average_gfx_activity
    }
    
    fn get_activity_breakdown(&self) -> HashMap<String, u16> {
        let mut breakdown = HashMap::new();
        breakdown.insert("GFX".to_string(), self.average_gfx_activity);
        breakdown.insert("UMC".to_string(), self.average_umc_activity);
        breakdown.insert("MM".to_string(), self.average_mm_activity);
        
        if let Some(vcn_activity) = &self.vcn_activity {
            for (i, &activity) in vcn_activity.iter().enumerate() {
                breakdown.insert(format!("VCN{}", i), activity);
            }
        }
        
        breakdown
    }

    fn get_frequency(&self) -> u16 {
        self.current_gfxclk
    }
    
    fn get_all_frequencies(&self) -> HashMap<String, u16> {
        let mut frequencies = HashMap::new();
        frequencies.insert("GFX Current".to_string(), self.current_gfxclk);
        frequencies.insert("GFX Average".to_string(), self.average_gfxclk_frequency);
        frequencies.insert("SOC Current".to_string(), self.current_socclk);
        frequencies.insert("SOC Average".to_string(), self.average_socclk_frequency);
        frequencies.insert("UCLK Current".to_string(), self.current_uclk);
        frequencies.insert("UCLK Average".to_string(), self.average_uclk_frequency);
        frequencies.insert("VCLK0 Current".to_string(), self.current_vclk0);
        frequencies.insert("VCLK0 Average".to_string(), self.average_vclk0_frequency);
        frequencies.insert("DCLK0 Current".to_string(), self.current_dclk0);
        frequencies.insert("DCLK0 Average".to_string(), self.average_dclk0_frequency);
        frequencies.insert("VCLK1 Current".to_string(), self.current_vclk1);
        frequencies.insert("VCLK1 Average".to_string(), self.average_vclk1_frequency);
        frequencies.insert("DCLK1 Current".to_string(), self.current_dclk1);
        frequencies.insert("DCLK1 Average".to_string(), self.average_dclk1_frequency);
        frequencies
    }

    fn get_throttle_status(&self) -> u64 {
        self.throttle_status
    }

    fn get_fan_speed(&self) -> (u16, bool) {
        // Convert PWM to percentage if needed
        let speed = if self.current_fan_speed > 100 {
            ((self.current_fan_speed as f64 / 255.0) * 100.0) as u16
        } else {
            self.current_fan_speed
        };
        (speed, self.current_fan_speed > 0)
    }

    fn get_header(&self) -> Header {
        self.header.clone()
    }
    
    fn get_memory_info(&self) -> Option<MemoryInfo> {
        Some(MemoryInfo {
            total_vram: None,
            used_vram: None,
            memory_frequency: Some(self.current_uclk),
            memory_utilization: Some(self.average_umc_activity),
        })
    }
    
    fn get_voltage_info(&self) -> Option<VoltageInfo> {
        Some(VoltageInfo {
            core_voltage: self.gfx_voltage,
            soc_voltage: self.soc_voltage,
            memory_voltage: self.mem_voltage,
        })
    }
    
    fn get_system_clock_counter(&self) -> u64 {
        self.system_clock_counter
    }
}

// GPU Metrics v2.x implementation
#[derive(Debug, Clone)]
pub struct GpuMetricsV2 {
    pub header: Header,
    pub system_clock_counter: u64,
    pub temperature_gfx: u16,
    pub temperature_soc: u16,
    pub temperature_core: [u16; 8],
    pub temperature_l3: [u16; 2],
    pub average_gfx_activity: u16,
    pub average_mm_activity: u16,
    pub average_socket_power: u16,
    pub average_cpu_power: u16,
    pub average_soc_power: u16,
    pub average_gfx_power: u16,
    pub average_core_power: [u16; 8],
    pub average_gfxclk_frequency: u16,
    pub average_socclk_frequency: u16,
    pub average_uclk_frequency: u16,
    pub average_fclk_frequency: u16,
    pub average_vclk_frequency: u16,
    pub average_dclk_frequency: u16,
    pub current_gfxclk: u16,
    pub current_socclk: u16,
    pub current_uclk: u16,
    pub current_fclk: u16,
    pub current_vclk: u16,
    pub current_dclk: u16,
    pub current_coreclk: [u16; 8],
    pub current_l3clk: [u16; 2],
    pub throttle_status: u64,
    pub fan_pwm: u16,
    // v2.1+ fields
    pub voltage_soc: Option<u16>,
    pub voltage_gfx: Option<u16>,
    pub voltage_mem: Option<u16>,
}

impl GpuMetrics for GpuMetricsV2 {
    fn get_temperature(&self) -> (u16, String) {
        (self.temperature_gfx, "GFX".to_string())
    }
    
    fn get_all_temperatures(&self) -> Vec<(String, u16)> {
        let mut temps = vec![
            ("GFX".to_string(), self.temperature_gfx),
            ("SOC".to_string(), self.temperature_soc),
        ];
        
        for (i, &temp) in self.temperature_core.iter().enumerate() {
            temps.push((format!("Core{}", i), temp));
        }
        
        for (i, &temp) in self.temperature_l3.iter().enumerate() {
            temps.push((format!("L3_{}", i), temp));
        }
        
        temps
    }

    fn get_power(&self) -> u16 {
        self.average_socket_power
    }
    
    fn get_power_breakdown(&self) -> HashMap<String, u16> {
        let mut breakdown = HashMap::new();
        breakdown.insert("Socket".to_string(), self.average_socket_power);
        breakdown.insert("CPU".to_string(), self.average_cpu_power);
        breakdown.insert("SOC".to_string(), self.average_soc_power);
        breakdown.insert("GFX".to_string(), self.average_gfx_power);
        
        for (i, &power) in self.average_core_power.iter().enumerate() {
            breakdown.insert(format!("Core{}", i), power);
        }
        
        breakdown
    }

    fn get_activity(&self) -> u16 {
        self.average_gfx_activity
    }
    
    fn get_activity_breakdown(&self) -> HashMap<String, u16> {
        let mut breakdown = HashMap::new();
        breakdown.insert("GFX".to_string(), self.average_gfx_activity);
        breakdown.insert("MM".to_string(), self.average_mm_activity);
        breakdown
    }

    fn get_frequency(&self) -> u16 {
        self.current_gfxclk
    }
    
    fn get_all_frequencies(&self) -> HashMap<String, u16> {
        let mut frequencies = HashMap::new();
        frequencies.insert("GFX Current".to_string(), self.current_gfxclk);
        frequencies.insert("GFX Average".to_string(), self.average_gfxclk_frequency);
        frequencies.insert("SOC Current".to_string(), self.current_socclk);
        frequencies.insert("SOC Average".to_string(), self.average_socclk_frequency);
        frequencies.insert("UCLK Current".to_string(), self.current_uclk);
        frequencies.insert("UCLK Average".to_string(), self.average_uclk_frequency);
        frequencies.insert("FCLK Current".to_string(), self.current_fclk);
        frequencies.insert("FCLK Average".to_string(), self.average_fclk_frequency);
        frequencies.insert("VCLK Current".to_string(), self.current_vclk);
        frequencies.insert("VCLK Average".to_string(), self.average_vclk_frequency);
        frequencies.insert("DCLK Current".to_string(), self.current_dclk);
        frequencies.insert("DCLK Average".to_string(), self.average_dclk_frequency);
        
        for (i, &freq) in self.current_coreclk.iter().enumerate() {
            frequencies.insert(format!("Core{}_CLK", i), freq);
        }
        
        for (i, &freq) in self.current_l3clk.iter().enumerate() {
            frequencies.insert(format!("L3_{}_CLK", i), freq);
        }
        
        frequencies
    }

    fn get_throttle_status(&self) -> u64 {
        self.throttle_status
    }

    fn get_fan_speed(&self) -> (u16, bool) {
        // fan_pwm is PWM value, convert to percentage
        let speed = ((self.fan_pwm as f64 / 255.0) * 100.0) as u16;
        (speed, self.fan_pwm > 0)
    }

    fn get_header(&self) -> Header {
        self.header.clone()
    }
    
    fn get_memory_info(&self) -> Option<MemoryInfo> {
        Some(MemoryInfo {
            total_vram: None,
            used_vram: None,
            memory_frequency: Some(self.current_uclk),
            memory_utilization: None,
        })
    }
    
    fn get_voltage_info(&self) -> Option<VoltageInfo> {
        Some(VoltageInfo {
            core_voltage: self.voltage_gfx,
            soc_voltage: self.voltage_soc,
            memory_voltage: self.voltage_mem,
        })
    }
    
    fn get_system_clock_counter(&self) -> u64 {
        self.system_clock_counter
    }
}

/// Cached metrics entry with timestamp and validation.
#[derive(Debug, Clone)]
struct CachedMetrics {
    metrics: Box<dyn GpuMetrics>,
    timestamp: Instant,
    read_count: usize,
    last_validation: Instant,
}

/// Advanced metrics reader with intelligent caching and error recovery.
#[derive(Debug)]
pub struct MetricsReader {
    cache_strategy: CacheStrategy,
    cache: Arc<Mutex<Option<CachedMetrics>>>,
    memory_map: Arc<Mutex<Option<memmap2::Mmap>>>,
    error_count: usize,
    last_successful_read: Option<Instant>,
}

impl MetricsReader {
    /// Create a new metrics reader with default caching.
    pub fn new() -> Self {
        Self::with_cache_strategy(CacheStrategy::default())
    }
    
    /// Create a metrics reader with specific cache strategy.
    pub fn with_cache_strategy(strategy: CacheStrategy) -> Self {
        Self {
            cache_strategy: strategy,
            cache: Arc::new(Mutex::new(None)),
            memory_map: Arc::new(Mutex::new(None)),
            error_count: 0,
            last_successful_read: None,
        }
    }
    
    /// Read GPU metrics from file with caching and error recovery.
    pub fn read_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Box<dyn GpuMetrics>, SensorError> {
        // Check cache first
        if let Some(cached) = self.check_cache()? {
            return Ok(cached);
        }
        
        // Read fresh data
        let metrics = match self.cache_strategy {
            CacheStrategy::MemoryMapped => self.read_with_mmap(path.as_ref())?,
            _ => self.read_direct(path.as_ref())?,
        };
        
        // Update cache
        self.update_cache(metrics.clone())?;
        
        // Reset error state on successful read
        self.error_count = 0;
        self.last_successful_read = Some(Instant::now());
        
        Ok(metrics)
    }
    
    /// Check if cached data is valid and return it if so.
    fn check_cache(&self) -> Result<Option<Box<dyn GpuMetrics>>, SensorError> {
        match self.cache_strategy {
            CacheStrategy::None => Ok(None),
            CacheStrategy::Basic { max_age } => {
                let cache = self.cache.lock().unwrap();
                if let Some(ref cached) = *cache {
                    if cached.timestamp.elapsed() < max_age {
                        return Ok(Some(cached.metrics.clone()));
                    }
                }
                Ok(None)
            },
            CacheStrategy::Aggressive { max_age, change_threshold: _ } => {
                let cache = self.cache.lock().unwrap();
                if let Some(ref cached) = *cache {
                    if cached.timestamp.elapsed() < max_age {
                        return Ok(Some(cached.metrics.clone()));
                    }
                }
                Ok(None)
            },
            CacheStrategy::MemoryMapped => {
                // Memory mapped files are always "cached"
                Ok(None)
            },
        }
    }
    
    /// Update cache with new metrics.
    fn update_cache(&self, metrics: Box<dyn GpuMetrics>) -> Result<(), SensorError> {
        match self.cache_strategy {
            CacheStrategy::None => Ok(()),
            _ => {
                let mut cache = self.cache.lock().unwrap();
                *cache = Some(CachedMetrics {
                    metrics,
                    timestamp: Instant::now(),
                    read_count: cache.as_ref().map_or(1, |c| c.read_count + 1),
                    last_validation: Instant::now(),
                });
                Ok(())
            }
        }
    }
    
    /// Read GPU metrics using memory mapping for maximum performance.
    fn read_with_mmap(&mut self, path: &Path) -> Result<Box<dyn GpuMetrics>, SensorError> {
        let mut mmap_guard = self.memory_map.lock().unwrap();
        
        // Create or refresh memory map
        if mmap_guard.is_none() {
            let file = File::open(path)?;
            let mmap = unsafe { MmapOptions::new().map(&file)? };
            *mmap_guard = Some(mmap);
        }
        
        let mmap = mmap_guard.as_ref().unwrap();
        self.parse_metrics_from_bytes(mmap)
    }
    
    /// Read GPU metrics directly from file.
    fn read_direct(&mut self, path: &Path) -> Result<Box<dyn GpuMetrics>, SensorError> {
        let mut file = File::open(path)?;
        
        // Read header first (4 bytes)
        let mut header_buf = [0u8; 4];
        file.read_exact(&mut header_buf)?;
        
        let header = Header {
            structure_size: u16::from_le_bytes([header_buf[0], header_buf[1]]),
            format_revision: header_buf[2],
            content_revision: header_buf[3],
        };

        // Validate structure size
        if header.structure_size == 0 || header.structure_size > 1024 {
            return Err(GpuError::MetricsParsingError {
                reason: format!("Invalid structure size: {}", header.structure_size),
            }.into());
        }

        // Read remaining data
        let data_size = header.structure_size as usize - 4;
        let mut data_buf = vec![0u8; data_size];
        file.read_exact(&mut data_buf)?;

        // Combine header and data
        let mut full_data = header_buf.to_vec();
        full_data.extend(data_buf);

        self.parse_metrics_from_bytes(&full_data)
    }
    
    /// Parse metrics from raw bytes.
    fn parse_metrics_from_bytes(&self, data: &[u8]) -> Result<Box<dyn GpuMetrics>, SensorError> {
        if data.len() < 4 {
            return Err(GpuError::MetricsParsingError {
                reason: "Insufficient data for header".to_string(),
            }.into());
        }
        
        let header = Header {
            structure_size: u16::from_le_bytes([data[0], data[1]]),
            format_revision: data[2],
            content_revision: data[3],
        };
        
        if !header.is_supported() {
            return Err(GpuError::MetricsParsingError {
                reason: format!("Unsupported GPU metrics version: {}", header.version()),
            }.into());
        }
        
        let data_slice = &data[4..];
        
        match header.format_revision {
            1 => self.parse_v1_metrics(header, data_slice),
            2 => self.parse_v2_metrics(header, data_slice),
            _ => Err(GpuError::MetricsParsingError {
                reason: format!("Unsupported format version: v{}.{}", 
                    header.format_revision, header.content_revision),
            }.into()),
        }
    }

    /// Parse v1.x GPU metrics.
    fn parse_v1_metrics(&self, header: Header, data: &[u8]) -> Result<Box<dyn GpuMetrics>, SensorError> {
        if data.len() < 92 { // Minimum size for v1.0
            return Err(GpuError::MetricsParsingError {
                reason: "Insufficient data for v1.x metrics".to_string(),
            }.into());
        }
        
        let mut metrics = GpuMetricsV1 {
            header,
            system_clock_counter: read_u64_le(data, 0),
            temperature_edge: read_u16_le(data, 8),
            temperature_hotspot: read_u16_le(data, 10),
            temperature_mem: read_u16_le(data, 12),
            temperature_vrgfx: read_u16_le(data, 14),
            temperature_vrsoc: read_u16_le(data, 16),
            temperature_vrmem: read_u16_le(data, 18),
            average_gfx_activity: read_u16_le(data, 20),
            average_umc_activity: read_u16_le(data, 22),
            average_mm_activity: read_u16_le(data, 24),
            average_socket_power: read_u16_le(data, 26),
            energy_accumulator: read_u64_le(data, 28),
            average_gfxclk_frequency: read_u16_le(data, 36),
            average_socclk_frequency: read_u16_le(data, 38),
            average_uclk_frequency: read_u16_le(data, 40),
            average_vclk0_frequency: read_u16_le(data, 42),
            average_dclk0_frequency: read_u16_le(data, 44),
            average_vclk1_frequency: read_u16_le(data, 46),
            average_dclk1_frequency: read_u16_le(data, 48),
            current_gfxclk: read_u16_le(data, 50),
            current_socclk: read_u16_le(data, 52),
            current_uclk: read_u16_le(data, 54),
            current_vclk0: read_u16_le(data, 56),
            current_dclk0: read_u16_le(data, 58),
            current_vclk1: read_u16_le(data, 60),
            current_dclk1: read_u16_le(data, 62),
            throttle_status: read_u64_le(data, 64),
            current_fan_speed: read_u16_le(data, 72),
            pcie_link_width: read_u16_le(data, 74),
            pcie_link_speed: read_u16_le(data, 76),
            gfx_voltage: None,
            soc_voltage: None,
            mem_voltage: None,
            indep_throttle_status: None,
            current_socket_power: None,
            vcn_activity: None,
        };
        
        // Parse version-specific fields
        if header.content_revision >= 1 && data.len() >= 82 {
            metrics.gfx_voltage = Some(read_u16_le(data, 78));
            metrics.soc_voltage = Some(read_u16_le(data, 80));
        }
        
        if header.content_revision >= 2 && data.len() >= 90 {
            metrics.mem_voltage = Some(read_u16_le(data, 82));
            metrics.indep_throttle_status = Some(read_u64_le(data, 84));
        }
        
        if header.content_revision >= 3 && data.len() >= 102 {
            metrics.current_socket_power = Some(read_u16_le(data, 92));
            let mut vcn_activity = [0u16; 4];
            for i in 0..4 {
                vcn_activity[i] = read_u16_le(data, 94 + i * 2);
            }
            metrics.vcn_activity = Some(vcn_activity);
        }
        
        Ok(Box::new(metrics))
    }

    /// Parse v2.x GPU metrics.
    fn parse_v2_metrics(&self, header: Header, data: &[u8]) -> Result<Box<dyn GpuMetrics>, SensorError> {
        if data.len() < 116 { // Minimum size for v2.0
            return Err(GpuError::MetricsParsingError {
                reason: "Insufficient data for v2.x metrics".to_string(),
            }.into());
        }
        
        // Parse temperature arrays
        let mut temperature_core = [0u16; 8];
        for i in 0..8 {
            temperature_core[i] = read_u16_le(data, 16 + i * 2);
        }
        
        let mut temperature_l3 = [0u16; 2];
        for i in 0..2 {
            temperature_l3[i] = read_u16_le(data, 32 + i * 2);
        }

        // Parse power arrays
        let mut average_core_power = [0u16; 8];
        for i in 0..8 {
            average_core_power[i] = read_u16_le(data, 54 + i * 2);
        }

        // Parse frequency arrays
        let mut current_coreclk = [0u16; 8];
        for i in 0..8 {
            current_coreclk[i] = read_u16_le(data, 84 + i * 2);
        }

        let mut current_l3clk = [0u16; 2];
        for i in 0..2 {
            current_l3clk[i] = read_u16_le(data, 100 + i * 2);
        }
        
        let mut metrics = GpuMetricsV2 {
            header,
            system_clock_counter: read_u64_le(data, 0),
            temperature_gfx: read_u16_le(data, 8),
            temperature_soc: read_u16_le(data, 10),
            temperature_core,
            temperature_l3,
            average_gfx_activity: read_u16_le(data, 36),
            average_mm_activity: read_u16_le(data, 38),
            average_socket_power: read_u16_le(data, 40),
            average_cpu_power: read_u16_le(data, 42),
            average_soc_power: read_u16_le(data, 44),
            average_gfx_power: read_u16_le(data, 46),
            average_core_power,
            average_gfxclk_frequency: read_u16_le(data, 70),
            average_socclk_frequency: read_u16_le(data, 72),
            average_uclk_frequency: read_u16_le(data, 74),
            average_fclk_frequency: read_u16_le(data, 76),
            average_vclk_frequency: read_u16_le(data, 78),
            average_dclk_frequency: read_u16_le(data, 80),
            current_gfxclk: read_u16_le(data, 82),
            current_socclk: read_u16_le(data, 84),
            current_uclk: read_u16_le(data, 86),
            current_fclk: read_u16_le(data, 88),
            current_vclk: read_u16_le(data, 90),
            current_dclk: read_u16_le(data, 92),
            current_coreclk,
            current_l3clk,
            throttle_status: read_u64_le(data, 104),
            fan_pwm: read_u16_le(data, 112),
            voltage_soc: None,
            voltage_gfx: None,
            voltage_mem: None,
        };
        
        // Parse v2.1+ fields
        if header.content_revision >= 1 && data.len() >= 122 {
            metrics.voltage_soc = Some(read_u16_le(data, 114));
            metrics.voltage_gfx = Some(read_u16_le(data, 116));
            metrics.voltage_mem = Some(read_u16_le(data, 118));
        }
        
        Ok(Box::new(metrics))
    }
    
    /// Invalidate cache to force fresh read.
    pub fn invalidate_cache(&mut self) {
        let mut cache = self.cache.lock().unwrap();
        *cache = None;
        
        let mut mmap = self.memory_map.lock().unwrap();
        *mmap = None;
    }
    
    /// Get cache statistics.
    pub fn cache_stats(&self) -> Option<(usize, Duration)> {
        let cache = self.cache.lock().unwrap();
        cache.as_ref().map(|c| (c.read_count, c.timestamp.elapsed()))
    }
}

impl Default for MetricsReader {
    fn default() -> Self {
        Self::new()
    }
}

// Inline helper functions for maximum performance
#[inline]
fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

#[inline]
fn read_u64_le(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
        data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_header_version() {
        let header = Header {
            structure_size: 100,
            format_revision: 1,
            content_revision: 2,
        };
        
        assert_eq!(header.version(), "v1.2");
        assert!(header.is_supported());
    }
    
    #[test]
    fn test_throttle_status() {
        let throttle = ThrottleStatus(0x10001); // PROCHOT_CPU + PPT0
        
        assert!(throttle.is_throttling());
        assert!(!throttle.is_thermal_throttling());
        assert!(throttle.is_power_throttling());
        
        let flags = throttle.active_flags();
        assert!(flags.contains(&"PROCHOT_CPU".to_string()));
        assert!(flags.contains(&"PPT0".to_string()));
    }
    
    #[test]
    fn test_cache_strategies() {
        let strategies = [
            CacheStrategy::None,
            CacheStrategy::Basic { max_age: Duration::from_millis(500) },
            CacheStrategy::Aggressive { 
                max_age: Duration::from_secs(1), 
                change_threshold: 5.0 
            },
            CacheStrategy::MemoryMapped,
        ];
        
        for strategy in &strategies {
            let reader = MetricsReader::with_cache_strategy(*strategy);
            assert_eq!(reader.cache_strategy, *strategy);
        }
    }
    
    #[test]
    fn test_power_efficiency_calculation() {
        let metrics = GpuMetricsV1 {
            header: Header { structure_size: 100, format_revision: 1, content_revision: 0 },
            system_clock_counter: 0,
            temperature_edge: 60,
            temperature_hotspot: 65,
            temperature_mem: 55,
            temperature_vrgfx: 50,
            temperature_vrsoc: 45,
            temperature_vrmem: 40,
            average_gfx_activity: 80,
            average_umc_activity: 50,
            average_mm_activity: 30,
            average_socket_power: 200,
            energy_accumulator: 0,
            average_gfxclk_frequency: 1500,
            average_socclk_frequency: 800,
            average_uclk_frequency: 1000,
            average_vclk0_frequency: 600,
            average_dclk0_frequency: 600,
            average_vclk1_frequency: 0,
            average_dclk1_frequency: 0,
            current_gfxclk: 1500,
            current_socclk: 800,
            current_uclk: 1000,
            current_vclk0: 600,
            current_dclk0: 600,
            current_vclk1: 0,
            current_dclk1: 0,
            throttle_status: 0,
            current_fan_speed: 50,
            pcie_link_width: 16,
            pcie_link_speed: 4,
            gfx_voltage: None,
            soc_voltage: None,
            mem_voltage: None,
            indep_throttle_status: None,
            current_socket_power: None,
            vcn_activity: None,
        };
        
        let power_efficiency = metrics.get_power_efficiency();
        assert_eq!(power_efficiency, 80.0 / 200.0);
        
        let thermal_efficiency = metrics.get_thermal_efficiency();
        assert_eq!(thermal_efficiency, 80.0 / 60.0);
    }
}