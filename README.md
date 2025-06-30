# waysensor-rs

A collection of high-performance system monitoring sensors for status bars like waybar, polybar, and others. Written in Rust for speed and efficiency.

## Features

- **🚀 High Performance**: Written in Rust with minimal resource usage
- **🎨 Icon Support**: UTF-8 icons with multiple icon sets (Nerd Fonts, Font Awesome, ASCII)
- **🔧 Configurable**: Flexible configuration options and output formats
- **📊 Comprehensive Monitoring**: CPU, Memory, Disk, Network, Battery, and GPU sensors
- **🎯 Status Bar Ready**: JSON output format compatible with waybar and other status bars

## Available Sensors

| Sensor              | Description                     | Icons Supported |
| ------------------- | ------------------------------- | --------------- |
| `waysensor-cpu`     | CPU usage monitoring            | ✅              |
| `waysensor-memory`  | Memory and swap usage           | ✅              |
| `waysensor-disk`    | Disk usage and I/O              | ✅              |
| `waysensor-network` | Network bandwidth monitoring    | ✅              |
| `waysensor-battery` | Battery status and charge level | ✅              |
| `waysensor-amd-gpu` | AMD GPU monitoring              | ✅              |
| `waysensor-thermal` | Temperature monitoring          | ✅              |

## Icon Support

waysensor-rs supports multiple icon styles to enhance your status bar appearance:

- **🚫 `none`** - No icons (default)
- **📝 `ascii`** - ASCII text icons (works everywhere)
- **🎨 `nerdfont`** - Nerd Font icons (requires Nerd Font)
- **🔤 `fontawesome`** - Font Awesome icons (requires Font Awesome)

### Usage

```bash
# Use Nerd Font icons
waysensor-cpu --icon-style nerdfont

# Use ASCII icons (compatible everywhere)
waysensor-memory --icon-style ascii

# Use Font Awesome icons
waysensor-disk --icon-style fontawesome
```

See [ICONS.md](ICONS.md) for detailed icon reference and installation instructions.

## Quick Start

### Installation

```bash
# Clone and build
git clone https://github.com/simonm/waysensor-rs
cd waysensor-rs
cargo build --release

# Or install directly with cargo
cargo install waysensor-cpu waysensor-memory waysensor-disk
```

### Hardware Availability Check

Before configuring sensors in your status bar, verify which sensors work on your system:

```bash
# Check what hardware is available
waysensor-cpu --check           # Works on all Linux systems
waysensor-memory --check        # Works on all Linux systems
waysensor-disk --check          # Works on all Linux systems
waysensor-nvidia-gpu --check    # Requires NVIDIA drivers + nvidia-smi
waysensor-intel-gpu --check     # Requires Intel GPU + DRM interfaces
waysensor-amd-gpu --check       # Requires AMD GPU + amdgpu driver
waysensor-battery --check       # Requires battery (laptops/UPS)
waysensor-thermal --check       # Requires thermal sensors
```

**Why check?** The sensor binaries can run even if the hardware isn't available, but they'll fail when trying to read actual data. Use `--check` to validate dependencies before adding sensors to your configuration.

### Basic Usage

```bash
# CPU usage with Nerd Font icons
waysensor-cpu --icon-style nerdfont

# Memory usage with warning thresholds
waysensor-memory --warning 80 --critical 95 --icon-style ascii

# Disk usage for root partition
waysensor-disk --path / --icon-style nerdfont

# Network bandwidth monitoring
waysensor-network --icon-style nerdfont

# Battery status (if available)
waysensor-battery --icon-style nerdfont
```

## Waybar Configuration

### With Nerd Font Icons

```json
{
  "custom/cpu": {
    "exec": "waysensor-cpu --icon-style nerdfont",
    "format": "{}",
    "interval": 2,
    "return-type": "json"
  },
  "custom/memory": {
    "exec": "waysensor-memory --icon-style nerdfont",
    "format": "{}",
    "interval": 5,
    "return-type": "json"
  }
}
```

### With ASCII Icons (No Font Requirements)

```json
{
  "custom/cpu": {
    "exec": "waysensor-cpu --icon-style ascii",
    "format": "{}",
    "interval": 2,
    "return-type": "json"
  }
}
```

See the [examples/](examples/) directory for complete configuration files.

## Example Output

### With Nerd Font Icons

- **CPU**: `󰍛 15%`
- **Memory**: `󰘚 42%`
- **Disk**: `󰋊 78%`
- **Network**: `󰈀 󰇚 1.2MB/s 󰕒 256KB/s`
- **Battery**: `󰂁 75%`

### With ASCII Icons

- **CPU**: `[CPU] 15%`
- **Memory**: `[MEM] 42%`
- **Disk**: `[DISK] 78%`
- **Network**: `[ETH] ↓ 1.2MB/s ↑ 256KB/s`
- **Battery**: `[75%] 75%`

### Without Icons

- **CPU**: `15%`
- **Memory**: `42%`
- **Disk**: `78%`
- **Network**: `↓1.2MB/s ↑256KB/s`
- **Battery**: `75%`

## Testing Icons

Run the provided test script to see all sensors with different icon styles:

```bash
./test-icons.sh
```

## Performance

waysensor-rs sensors are designed for efficiency:

- Minimal CPU usage (typically <0.1% per sensor)
- Low memory footprint (<1MB per sensor)
- Fast execution times (<1ms for most sensors)

See [docs/BENCHMARKS.md](docs/BENCHMARKS.md) for detailed performance comparisons.

## License

Licensed under

- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
