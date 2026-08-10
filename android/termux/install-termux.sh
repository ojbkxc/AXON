#!/data/data/com.termux/files/usr/bin/bash
set -euo pipefail

BIN="${1:-}"
INSTALL_DIR="${PREFIX:-$HOME/.local}/bin"
CONFIG_DIR="$HOME/.axon"

mkdir -p "$INSTALL_DIR" "$CONFIG_DIR"

if [ -z "$BIN" ]; then
    echo "AXON Termux installer"
    echo ""
    echo "Usage: bash install-termux.sh <path-to-axon-binary>"
    echo ""
    echo "Download the axon binary (aarch64-linux-android) from GitHub Actions artifacts:"
    echo "  https://github.com/ojbkxc/AXON/actions"
    echo "Look for the 'axon-apk-arm64-v8a' artifact, or the cross-compile step output."
    echo ""
    echo "Alternatively, build from source in Termux:"
    echo "  pkg install -y rust clang make pkg-config openssl"
    echo "  git clone https://github.com/ojbkxc/AXON && cd AXON"
    echo "  cargo build --release -p axon-server"
    echo "  bash install-termux.sh target/release/axon"
    exit 1
fi

if [ ! -f "$BIN" ]; then
    echo "Error: binary not found at $BIN"
    exit 1
fi

cp "$BIN" "$INSTALL_DIR/axon"
chmod +x "$INSTALL_DIR/axon"

if [ ! -f "$CONFIG_DIR/config.yaml" ]; then
    cat > "$CONFIG_DIR/config.yaml" <<'YAML'
server:
  addr: 127.0.0.1:8080
  log_level: info

storage:
  path: ~/.axon/axon.db
  max_connections: 4

gateway:
  models: []
  routes: []

agents: []
tools: []
YAML
    echo "Default config written to $CONFIG_DIR/config.yaml"
fi

SIZE=$(ls -lh "$INSTALL_DIR/axon" | awk '{print $5}')
echo "AXON installed to $INSTALL_DIR/axon ($SIZE)"
echo "Config: $CONFIG_DIR/config.yaml"
echo ""
echo "Start with:"
echo "  axon --config ~/.axon/config.yaml"
echo "Then open http://127.0.0.1:8080/ui/ in a browser."
