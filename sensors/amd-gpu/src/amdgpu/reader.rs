use super::types::*;
use waysensor_rs_core::SensorError;
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Debug)]
pub struct MetricsReader;

impl MetricsReader {
    pub fn new() -> Self {
        Self
    }

    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<Box<dyn GpuMetrics>, SensorError> {
        let mut file = fs::File::open(&path)?;
        
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
            return Err(SensorError::Parse { 
                message: format!("Invalid structure size: {}", header.structure_size),
                source: None,
            });
        }

        // Read remaining data
        let data_size = header.structure_size as usize - 4;
        let mut data_buf = vec![0u8; data_size];
        file.read_exact(&mut data_buf)?;

        // Parse based on version
        self.parse_metrics(header, &data_buf)
    }

    fn parse_metrics(&self, header: Header, data: &[u8]) -> Result<Box<dyn GpuMetrics>, SensorError> {
        match header.format_revision {
            1 => self.parse_v1_metrics(header, data),
            2 => self.parse_v2_metrics(header, data),
            _ => Err(SensorError::Parse { 
                message: format!("Unsupported format version: v{}.{}", header.format_revision, header.content_revision),
                source: None,
            }),
        }
    }

    fn parse_v1_metrics(&self, header: Header, data: &[u8]) -> Result<Box<dyn GpuMetrics>, SensorError> {
        match header.content_revision {
            0 | 1 | 2 | 3 => {
                // All v1.x versions have the same basic structure for the fields we use
                if data.len() < 96 { // Minimum size for v1.x
                    return Err(SensorError::Parse {
                        message: "Insufficient data for v1.x".to_string(),
                        source: None,
                    });
                }
                
                // Parse v1.x binary data (little-endian)
                let metrics = GpuMetricsV1_0 {
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
                };
                
                Ok(Box::new(metrics))
            }
            _ => Err(SensorError::Parse { 
                message: format!("Unsupported v1 content revision: {}", header.content_revision),
                source: None,
            }),
        }
    }

    fn parse_v2_metrics(&self, header: Header, data: &[u8]) -> Result<Box<dyn GpuMetrics>, SensorError> {
        match header.content_revision {
            0 => {
                if data.len() < 96 { // Minimum size for v2.0
                    return Err(SensorError::Parse {
                        message: "Insufficient data for v2.0".to_string(),
                        source: None,
                    });
                }
                
                // Parse v2.0 binary data (little-endian)
                let mut temperature_core = [0u16; 8];
                for i in 0..8 {
                    temperature_core[i] = read_u16_le(data, 16 + i * 2);
                }
                
                let mut temperature_l3 = [0u16; 2];
                for i in 0..2 {
                    temperature_l3[i] = read_u16_le(data, 32 + i * 2);
                }

                let mut average_core_power = [0u16; 8];
                for i in 0..8 {
                    average_core_power[i] = read_u16_le(data, 50 + i * 2);
                }

                let mut current_coreclk = [0u16; 8];
                for i in 0..8 {
                    current_coreclk[i] = read_u16_le(data, 84 + i * 2);
                }

                let mut current_l3clk = [0u16; 2];
                for i in 0..2 {
                    current_l3clk[i] = read_u16_le(data, 100 + i * 2);
                }
                
                let metrics = GpuMetricsV2_0 {
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
                    average_gfxclk_frequency: read_u16_le(data, 66),
                    average_socclk_frequency: read_u16_le(data, 68),
                    average_uclk_frequency: read_u16_le(data, 70),
                    average_fclk_frequency: read_u16_le(data, 72),
                    average_vclk_frequency: read_u16_le(data, 74),
                    average_dclk_frequency: read_u16_le(data, 76),
                    current_gfxclk: read_u16_le(data, 78),
                    current_socclk: read_u16_le(data, 80),
                    current_uclk: read_u16_le(data, 82),
                    current_fclk: read_u16_le(data, 84),
                    current_vclk: read_u16_le(data, 86),
                    current_dclk: read_u16_le(data, 88),
                    current_coreclk,
                    current_l3clk,
                    throttle_status: read_u64_le(data, 104),
                    fan_pwm: read_u16_le(data, 112),
                    padding: [0u16; 3],
                };
                
                Ok(Box::new(metrics))
            }
            _ => Err(SensorError::Parse { 
                message: format!("Unsupported v2 content revision: {}", header.content_revision),
                source: None,
            }),
        }
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