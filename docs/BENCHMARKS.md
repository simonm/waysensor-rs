## Overview

Performance benchmarks for waysensor-rs sensors using Criterion.rs, the de facto standard for Rust benchmarking.

## Benchmark Results

### CPU Sensor (`waysensor-rs-cpu`)

| Operation               | Time       | Description                            |
| ----------------------- | ---------- | -------------------------------------- |
| `parse_proc_stat`       | **140 ns** | Parse /proc/stat CPU line              |
| `parse_cpu_info`        | **435 ns** | Parse /proc/cpuinfo for processor info |
| `cpu_usage_calculation` | **1.8 ns** | Calculate CPU usage percentage         |
| `cpu_stats_total`       | **429 ps** | Calculate total CPU ticks              |
| `cpu_stats_active`      | **426 ps** | Calculate active CPU ticks             |

### Memory Sensor (`waysensor-rs-memory`)

| Operation               | Time       | Description                |
| ----------------------- | ---------- | -------------------------- |
| `parse_meminfo`         | **3.3 µs** | Parse entire /proc/meminfo |
| `mem_used_calculation`  | **428 ps** | Calculate used memory      |
| `mem_used_percentage`   | **456 ps** | Calculate memory usage %   |
| `swap_used_calculation` | **426 ps** | Calculate swap usage       |
| `parse_meminfo_line`    | **51 ns**  | Parse single meminfo line  |

### Disk Sensor (`waysensor-rs-disk`)

| Operation              | Time       | Description                    |
| ---------------------- | ---------- | ------------------------------ |
| `parse_df_output`      | **177 ns** | Parse df command output        |
| `parse_df_line`        | **163 ns** | Parse single df line           |
| `disk_used_percentage` | **449 ps** | Calculate disk usage %         |
| `format_bytes_*`       | **122 ns** | Format bytes to human readable |

### AMD GPU Sensor (`waysensor-rs-amd-gpu`)

| Operation                  | Time       | Description               |
| -------------------------- | ---------- | ------------------------- |
| `parse_header`             | **766 ps** | Parse GPU metrics header  |
| `parse_v1_full_metrics`    | **1.7 ns** | Parse v1.x GPU metrics    |
| `parse_v2_full_metrics`    | **1.4 ns** | Parse v2.x GPU metrics    |
| `parse_single_temperature` | **879 ps** | Extract temperature value |
| `parse_single_frequency`   | **871 ps** | Extract frequency value   |
| `match_gpu_file_pattern`   | **1.1 µs** | Match GPU file patterns   |

### Network Sensor (`waysensor-rs-network`)

| Operation                 | Time          | Description              |
| ------------------------- | ------------- | ------------------------ |
| `parse_network_stat`      | **7.3 ns**    | Parse network statistics |
| `calculate_network_speed` | **4.6 ns**    | Calculate network speed  |
| `parse_proc_net_dev_line` | **3.0 ns**    | Parse /proc/net/dev line |
| `format_speed_*`          | **66-155 ns** | Format network speeds    |
| `interface_name_checks`   | **1.3 ns**    | Check interface validity |

## Key Performance Insights

1. **Sub-microsecond operations**: Most core parsing operations complete in nanoseconds
2. **Efficient parsing**: Line-by-line parsing typically under 200ns
3. **Minimal calculation overhead**: Mathematical operations complete in picoseconds
4. **Fast formatting**: Human-readable formatting under 200ns

