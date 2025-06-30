# Building waysensor-rs

waysensor-rs is a Rust project that builds into multiple binary sensors for waybar. Here are the different ways to build and install it:

## 🚀 Quick Start (Recommended)

### 1. Auto-detect and Setup

The easiest way to build and configure waysensor-rs for your system:

```bash
# Clone or download waysensor-rs
cd waysensor-rs/

# Run the setup wizard (detects your hardware and generates install script)
cargo run --bin waysensor-discover -- --setup

# Run the generated install script
./generated-install.sh
```

This will:

- Detect your hardware capabilities
- Generate a custom install script for your system
- Build only the sensors you need
- Install them to `~/.local/bin`
- Generate waybar config and CSS

### 2. Manual Build All Sensors

```bash
# Build all sensors at once
cargo build --release --bins

# Install manually
mkdir -p ~/.local/bin
cp target/release/waysensor-* ~/.local/bin/

# Add to PATH if needed
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

## 🛠️ Individual Sensor Builds

You can build individual sensors if you only need specific ones:

```bash
# CPU monitoring
cargo build --release --bin waysensor-cpu

# Memory monitoring
cargo build --release --bin waysensor-memory

# Disk monitoring (with multi-path support)
cargo build --release --bin waysensor-disk

# AMD GPU monitoring
cargo build --release --bin waysensor-amd-gpu

# Network monitoring (with auto-detection)
cargo build --release --bin waysensor-network

# Battery monitoring
cargo build --release --bin waysensor-battery

# Thermal monitoring
cargo build --release --bin waysensor-thermal

# Hardware discovery tool
cargo build --release --bin waysensor-discover
```

## 📋 Requirements

### System Requirements

- **Rust 1.70+** (install from [rustup.rs](https://rustup.rs/))
- **Linux** (uses /proc, /sys filesystems)
- **df command** (for disk monitoring)
- **ip command** (for network detection)

### Optional Requirements

- **AMD GPU** with amdgpu driver for GPU monitoring
- **Nerd Font** or **Font Awesome** for icons
- **jq** for testing JSON output

### Installing Rust

```bash
# Quick install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.bashrc

# Verify installation
cargo --version
```

## 🧪 Testing Your Build

### Test Individual Sensors

```bash
# Test each sensor
waysensor-cpu --once --icon-style ascii
waysensor-memory --once --icon-style nerdfont
waysensor-disk --once --path / --icon-style ascii
waysensor-network --once --detect  # Show interface detection
```

### Run Test Suite

```bash
# Icon support test
./test-icons.sh

# Performance benchmarks
cd waysensor-cpu && cargo bench
cd ../waysensor-memory && cargo bench
```

## 📦 Binary Information

After building, you'll have these binaries in `target/release/`:

| Binary               | Size   | Purpose                        |
| -------------------- | ------ | ------------------------------ |
| `waysensor-cpu`      | ~1.7MB | CPU usage monitoring           |
| `waysensor-memory`   | ~1.7MB | RAM/swap monitoring            |
| `waysensor-disk`     | ~1.7MB | Disk usage (single/multi-path) |
| `waysensor-amd-gpu`  | ~1.7MB | AMD GPU metrics                |
| `waysensor-network`  | ~1.7MB | Network throughput             |
| `waysensor-battery`  | ~1.7MB | Battery status                 |
| `waysensor-thermal`  | ~1.7MB | Temperature monitoring         |
| `waysensor-discover` | ~1.7MB | Hardware detection tool        |

All binaries are statically linked and have no external dependencies.

## 🎯 Waybar Integration

### Basic Setup

1. Build and install sensors (see above)
2. Generate config: `waysensor-discover --complete-config`
3. Copy generated config to your waybar setup
4. Restart waybar

### Manual Configuration

Add to your waybar config:

```json
{
  "custom/waysensor-cpu": {
    "exec": "waysensor-cpu --once --icon-style nerdfont",
    "format": "{}",
    "interval": 2,
    "return-type": "json",
    "tooltip": true
  }
}
```

## 🚨 Troubleshooting

### Build Issues

```bash
# Update Rust
rustup update

# Clean build cache
cargo clean
cargo build --release --bins

# Check for missing system packages
# Ubuntu/Debian: sudo apt install build-essential
# Arch: sudo pacman -S base-devel
```

### Permission Issues

```bash
# Ensure ~/.local/bin exists and is in PATH
mkdir -p ~/.local/bin
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Sensor Issues

```bash
# Test hardware detection
waysensor-discover --smart --verbose

# Test specific sensor directly
waysensor-cpu --once  # Should output JSON
```

## 📊 Performance

Build times (first build):

- **Individual sensor**: 30-60 seconds
- **All sensors**: 2-3 minutes
- **Subsequent builds**: 5-10 seconds

Runtime performance:

- **Execution time**: 3-5ms per sensor (except CPU: 104ms due to measurement delay)
- **Memory usage**: <2MB per sensor
- **CPU impact**: <1% during continuous monitoring

See `BENCHMARKS.md` for detailed performance analysis.

