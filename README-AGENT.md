# Smart Home Agent 🏠

A fast, Rust-based CLI tool for controlling your smart home devices via Homebridge API.

## Features

- 🚀 **Fast & Reliable** - Written in Rust for performance
- 🔐 **Secure** - Automatically loads credentials from Kubernetes secrets
- 🔍 **Smart Device Discovery** - Finds devices by partial name matching
- 💡 **Multi-Device Control** - Control lights, switches, and outlets
- 🎛️ **Brightness Control** - Adjust light brightness (0-100%)
- ⚡ **Quick Commands** - Shortcuts for common operations like kitchen lights

## Installation

### Build from source:
```bash
cargo build --release
```

The binary will be at `./target/release/smart-home-agent`

### Optional: Install globally
```bash
cargo install --path .
```

## Usage

The agent automatically loads Homebridge credentials from your Kubernetes secret (`homebridge-credentials` in the `default` namespace).

### List all devices
```bash
smart-home-agent list
```

### Turn devices on/off
```bash
# Turn on
smart-home-agent on "wohnzimmer"

# Turn off
smart-home-agent off "wohnzimmer"
```

**Note**: Device names support partial matching!

### Control brightness
```bash
smart-home-agent brightness "wohnzimmer" 50
```

### Kitchen lights shortcut
```bash
# Turn on
smart-home-agent kitchen ein
smart-home-agent kitchen on

# Turn off
smart-home-agent kitchen aus
smart-home-agent kitchen off
```

### Manual credentials (optional)
```bash
smart-home-agent --username USER --password PASS list
```

## Examples

```bash
# List all your devices with their current state
smart-home-agent list

# Turn on living room lights
smart-home-agent on "wohnzimmer deckenlampe"

# Set bedroom light to 30% brightness
smart-home-agent brightness "bruno schrank" 30

# Quick access to kitchen lights
smart-home-agent kitchen ein
```

## Device Matching

The agent uses smart device matching:
- **Exact match** - If you type the full device name
- **Partial match** - If your query is part of a device name
- **Case insensitive** - "WOHNZIMMER" = "wohnzimmer"

If multiple devices match, you'll see a list of matches to be more specific.

## Architecture

- **Language**: Rust 🦀
- **HTTP Client**: reqwest (blocking)
- **CLI Framework**: clap v4
- **Credentials**: Automatically sourced from Kubernetes secrets via kubectl
- **API**: Homebridge Config UI X REST API

## Configuration

### Default Settings
- **Homebridge URL**: `http://192.168.178.67:8581`
- **Credentials Source**: `kubectl get secret homebridge-credentials -n default`
- **Kubeconfig**: `~/.kube/config-squid`

### Override URL
```bash
smart-home-agent --url http://other-homebridge:8581 list
```

## Command Reference

| Command | Description | Example |
|---------|-------------|---------|
| `list` | Show all controllable devices | `smart-home-agent list` |
| `on <device>` | Turn device on | `smart-home-agent on kitchen` |
| `off <device>` | Turn device off | `smart-home-agent off kitchen` |
| `brightness <device> <0-100>` | Set brightness | `smart-home-agent brightness lamp 50` |
| `kitchen <state>` | Control both kitchen lights | `smart-home-agent kitchen on` |

## Integration

This agent integrates with:
- ✅ Homebridge API
- ✅ Kubernetes secrets
- ✅ Zigbee2MQTT devices (via Homebridge)
- ✅ All HomeKit-compatible devices

## Development

```bash
# Build in debug mode
cargo build

# Run with logging
RUST_LOG=debug cargo run -- list

# Run tests
cargo test

# Check code
cargo clippy
```

## Troubleshooting

### "Failed to load credentials"
- Ensure kubectl is configured with `~/.kube/config-squid`
- Verify the secret exists: `kubectl get secret homebridge-credentials -n default`
- Or provide credentials manually with `--username` and `--password`

### "Device not found"
- Run `smart-home-agent list` to see available devices
- Try a partial match instead of the full name
- Device names are case-insensitive

### Authentication Failed
- Check Homebridge is running: `curl http://192.168.178.67:8581`
- Verify credentials in the Kubernetes secret
- Try manual credentials to test

## License

Part of the Genesis infrastructure project.
