use waysensor_rs_core::SensorError;
use std::fmt;

/// Header for GPU metrics structure
#[derive(Debug, Clone)]
pub struct Header {
    pub structure_size: u16,
    pub format_revision: u8,
    pub content_revision: u8,
}

impl Header {
    pub fn version(&self) -> String {
        format!("v{}.{}", self.format_revision, self.content_revision)
    }
}

/// Trait for all GPU metrics versions
pub trait GpuMetrics: fmt::Debug {
    fn get_temperature(&self) -> (u16, String);
    fn get_power(&self) -> u16;
    fn get_activity(&self) -> u16;
    fn get_frequency(&self) -> u16;
    fn get_throttle_status(&self) -> u64;
    fn get_fan_speed(&self) -> (u16, bool);
    fn get_header(&self) -> Header;
}

/// Throttle status with bit flags and helper methods
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

    pub fn is_throttling(&self) -> bool {
        self.0 != 0
    }

    pub fn active_flags(&self) -> Vec<String> {
        let mut flags = Vec::new();
        for (name, flag) in Self::THROTTLE_FLAGS {
            if self.0 & flag != 0 {
                flags.push(name.to_string());
            }
        }
        flags
    }
}

// GPU Metrics v1.0
#[derive(Debug, Clone)]
pub struct GpuMetricsV1_0 {
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
}

impl GpuMetrics for GpuMetricsV1_0 {
    fn get_temperature(&self) -> (u16, String) {
        (self.temperature_edge, "Edge".to_string())
    }

    fn get_power(&self) -> u16 {
        self.average_socket_power
    }

    fn get_activity(&self) -> u16 {
        self.average_gfx_activity
    }

    fn get_frequency(&self) -> u16 {
        self.average_gfxclk_frequency
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
}

// Similar implementations for other GPU metrics versions would go here
// For brevity, I'll implement v2.0 as an example

#[derive(Debug, Clone)]
pub struct GpuMetricsV2_0 {
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
    pub padding: [u16; 3],
}

impl GpuMetrics for GpuMetricsV2_0 {
    fn get_temperature(&self) -> (u16, String) {
        (self.temperature_gfx, "GFX".to_string())
    }

    fn get_power(&self) -> u16 {
        self.average_socket_power
    }

    fn get_activity(&self) -> u16 {
        self.average_gfx_activity
    }

    fn get_frequency(&self) -> u16 {
        self.average_gfxclk_frequency
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
}

/// Find GPU metrics file automatically
pub fn find_gpu_metrics_file() -> Result<Option<std::path::PathBuf>, SensorError> {
    use std::path::Path;
    
    let patterns = [
        "/sys/class/drm/card*/device/gpu_metrics",
        "/sys/devices/pci*/*/drm/card*/device/gpu_metrics",
    ];

    for pattern in &patterns {
        if let Ok(paths) = glob::glob(pattern) {
            for path_result in paths {
                if let Ok(path) = path_result {
                    if Path::new(&path).exists() {
                        return Ok(Some(path));
                    }
                }
            }
        }
    }

    Err(SensorError::Unavailable {
        reason: "No AMD GPU found".to_string(),
        is_temporary: false,
    })
}