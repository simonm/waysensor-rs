#!/bin/bash
# Simple build script for waysensor-rs sensors

set -e

# Check if --install flag is passed
INSTALL_MODE=false
if [[ "$1" == "--install" ]]; then
    INSTALL_MODE=true
fi

echo "🔧 Building waysensor-rs sensors..."
echo "=============================="
echo ""

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Cargo (Rust) is required but not installed."
    echo "   Please install Rust from https://rustup.rs/"
    echo "   Quick install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "✅ Rust/Cargo found: $(cargo --version)"
echo ""

# Build all sensors
echo "🚀 Building all sensors..."
cargo build --release --bins

echo ""
echo "✅ Build complete!"
echo ""

# Show what was built
echo "📦 Built binaries:"
for binary in target/release/waysensor-rs-*; do
    if [ -f "$binary" ] && [ -x "$binary" ]; then
        name=$(basename "$binary")
        size=$(ls -lh "$binary" | awk '{print $5}')
        echo "  • $name ($size)"
    fi
done

echo ""
echo "🧪 Quick test:"
echo "--------------"
if [ -f "target/release/waysensor-rs-cpu" ]; then
    echo -n "CPU sensor: "
    ./target/release/waysensor-rs-cpu --once --icon-style none 2>/dev/null | jq -r .text || echo "Error"
fi

if [ -f "target/release/waysensor-rs-memory" ]; then
    echo -n "Memory sensor: "
    ./target/release/waysensor-rs-memory --once --icon-style none 2>/dev/null | jq -r .text || echo "Error"
fi

echo ""
echo "📋 Next steps:"
echo "1. Install binaries and config:"
echo "   ./build.sh --install"
echo ""
echo "2. Generate waybar configuration:"
echo "   waysensor-rs-discover --setup"
echo ""
echo "💡 Tips:"
echo "  • Use './build.sh --install' to build and install binaries + config"
echo "  • Use 'waysensor-rs-discover --setup' to generate waybar configuration files"
echo "  • Install a Nerd Font for icon support (https://www.nerdfonts.com)"
echo "  • Customize icons by editing the 'icons' section in config.jsonc"

# If --install flag was passed, perform installation
if [ "$INSTALL_MODE" = true ]; then
    echo ""
    echo "📦 Installing waysensor-rs..."
    echo "============================="
    
    # Determine install directories
    BIN_DIR="${HOME}/.local/bin"
    CONFIG_DIR="${XDG_CONFIG_HOME:-${HOME}/.config}/waysensor-rs"
    
    # Create bin directory if it doesn't exist
    if [ ! -d "$BIN_DIR" ]; then
        echo "📁 Creating $BIN_DIR..."
        mkdir -p "$BIN_DIR"
    fi
    
    # Create config directory
    echo "📁 Creating config directory: $CONFIG_DIR"
    mkdir -p "$CONFIG_DIR"
    
    # Install binaries
    echo "📦 Installing binaries to $BIN_DIR..."
    for binary in target/release/waysensor-rs-*; do
        if [ -f "$binary" ] && [ -x "$binary" ] && [[ ! "$binary" == *.d ]]; then
            name=$(basename "$binary")
            cp "$binary" "$BIN_DIR/"
            echo "  ✓ Installed $name"
        fi
    done
    
    # Install config file (generate example config)
    echo ""
    echo "📝 Installing default config..."
    if [ -f "$CONFIG_DIR/config.jsonc" ]; then
        echo "  ⚠️  Config file already exists at $CONFIG_DIR/config.jsonc"
        echo "  💡 Backing up existing config to config.jsonc.bak"
        cp "$CONFIG_DIR/config.jsonc" "$CONFIG_DIR/config.jsonc.bak"
    fi
    
    # Generate example config using any sensor
    if [ -f "target/release/waysensor-rs-cpu" ]; then
        ./target/release/waysensor-rs-cpu --generate-config >/dev/null 2>&1
        echo "  ✓ Generated config.jsonc with example settings"
    else
        echo "  ⚠️  Could not generate config.jsonc - no sensors built"
    fi
    
    # Note: No icon pack installation needed - icons come from config
    
    # Check if ~/.local/bin is in PATH
    if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
        echo ""
        echo "⚠️  Warning: $BIN_DIR is not in your PATH"
        echo "   Add this line to your ~/.bashrc or ~/.zshrc:"
        echo "   export PATH=\"\$HOME/.local/bin:\$PATH\""
    fi
    
    echo ""
    echo "✅ Installation complete!"
    echo ""
    echo "📋 Next steps:"
    echo "1. Generate waybar configuration:"
    echo "   waysensor-rs-discover --setup"
    echo ""
    echo "   This will create:"
    echo "   • waybar-config.json - Waybar module configuration"
    echo "   • waybar-style.css - Recommended CSS styling"
    echo ""
    echo "2. Copy the generated modules to your waybar config"
    echo "3. Add the CSS styling to your waybar style.css"
    echo "4. Restart waybar"
    echo ""
    echo "🎨 Icon configuration:"
    echo "   • Install a Nerd Font for icon support"
    echo "   • Set icon_style = \"nerdfont\" in config.jsonc (default)"
    echo "   • Set icon_style = \"none\" for text-only output"
    echo "   • Customize any icon in the \"icons\" section of config.jsonc"
    echo ""
    echo "💡 Customize: Edit $CONFIG_DIR/config.jsonc to modify colors, icons, and sensor settings"
fi