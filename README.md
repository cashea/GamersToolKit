# GamersToolKit

A real-time game analysis and assistance overlay that provides contextual gameplay tips through read-only screen parsing.

## Features

- **Screen Capture**: Uses Windows Graphics Capture API for safe, low-latency capture
- **OCR/Vision**: Extracts text and detects visual elements from game frames
- **Rules Engine**: Scriptable game profiles with rhai for custom logic
- **Overlay**: Non-intrusive transparent overlay with click passthrough
- **Anti-Cheat Safe**: Read-only operation - no game memory access or input injection

## Architecture

```
Screen Pixels → OCR/Vision → Rules Engine → Overlay Tips
```

### Layers

| Layer | Technology | Purpose |
|-------|------------|---------|
| Capture | windows-capture | Windows Graphics Capture API |
| Vision | PaddleOCR via ort | Text extraction, template matching |
| Analysis | rhai | Scriptable rules engine |
| Overlay | egui_overlay | Transparent tips display |
| Storage | rusqlite | Profiles & settings |

## Anti-Cheat Compliance

GamersToolKit is designed to be safe from anti-cheat detection:

- ✅ Uses official Windows Graphics Capture API (same as OBS, Discord)
- ✅ Overlay is a standard Windows window (no DirectX hooks)
- ✅ No DLL injection into game processes
- ✅ No game memory reading or writing
- ✅ No input simulation or automation
- ✅ Read-only pixel analysis only

## Requirements

- Windows 10 1903+ or Windows 11
- Rust 1.75+ (for building)
- Game running in windowed or borderless windowed mode (recommended)

## Building

```bash
# Install Rust from https://rustup.rs

# Clone and build
git clone https://github.com/cashea/GamersToolKit.git
cd gamers-toolkit
cargo build --release
```

## Usage

```bash
# Run the application
cargo run --release

# Or run the built executable
./target/release/gamers-toolkit.exe
```

## Game Profiles

Game profiles are JSON files that define:
- OCR regions to monitor (health, mana, cooldowns, etc.)
- Visual templates to detect (icons, status effects)
- Rules that generate tips based on detected values

See `profiles/example.json` for a template.

## License

MIT License - See LICENSE file for details.

## Disclaimer

This tool is for personal use to assist with gameplay. Always check a game's Terms of Service before using any overlay tool. The developers are not responsible for any actions taken by game publishers.
