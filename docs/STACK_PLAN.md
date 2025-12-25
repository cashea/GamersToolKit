# GamersToolKit Stack Plan

## Overview

This document defines the technology stack, dependencies, and architecture for GamersToolKit - a read-only game analysis overlay.

---

## Core Technology

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust 1.75+ | Performance, safety, Windows ecosystem support |
| Build | Cargo | Standard Rust toolchain |
| Target | Windows 10 1903+ / Windows 11 | Windows Graphics Capture API requirement |

---

## Layer Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Presentation Layer                    │
│                   (egui_overlay + audio)                 │
├─────────────────────────────────────────────────────────┤
│                     Analysis Layer                       │
│                  (rhai scripting engine)                 │
├─────────────────────────────────────────────────────────┤
│                      Vision Layer                        │
│              (PaddleOCR via ort + templates)             │
├─────────────────────────────────────────────────────────┤
│                     Capture Layer                        │
│              (windows-capture / WGC API)                 │
├─────────────────────────────────────────────────────────┤
│                     Storage Layer                        │
│                (rusqlite + JSON profiles)                │
└─────────────────────────────────────────────────────────┘
```

---

## Dependencies

### Capture Layer

| Crate | Version | Purpose |
|-------|---------|---------|
| `windows-capture` | 1.x | Windows Graphics Capture API wrapper |
| `image` | 0.25 | Image buffer handling |

**Notes:**
- Uses official WGC API (same as OBS, Discord)
- No DLL injection or DirectX hooks
- Supports windowed, borderless, and fullscreen capture

### Vision Layer

| Crate | Version | Purpose |
|-------|---------|---------|
| `ort` | 2.x | ONNX Runtime bindings |
| `ndarray` | 0.16 | Tensor manipulation |
| `imageproc` | 0.25 | Image processing utilities |

**OCR Model:**
- PaddleOCR (ONNX format)
  - `det` - Text detection model
  - `rec` - Text recognition model
  - `cls` - Text direction classifier (optional)

**Notes:**
- Models bundled or downloaded on first run
- GPU acceleration optional via ONNX Runtime providers

### Analysis Layer

| Crate | Version | Purpose |
|-------|---------|---------|
| `rhai` | 1.x | Embedded scripting engine |
| `serde` | 1.x | Serialization framework |
| `serde_json` | 1.x | JSON parsing for profiles |

**Notes:**
- Rhai chosen for safety (sandboxed, no FFI)
- Game profiles define rules in Rhai scripts
- Hot-reload support for development

### Presentation Layer

| Crate | Version | Purpose |
|-------|---------|---------|
| `egui` | 0.29 | Immediate mode GUI |
| `egui_overlay` | 0.x | Transparent overlay window |
| `eframe` | 0.29 | egui framework integration |

**Notes:**
- Click-through transparency
- Always-on-top positioning
- Minimal GPU overhead

### Storage Layer

| Crate | Version | Purpose |
|-------|---------|---------|
| `rusqlite` | 0.32 | SQLite database |
| `directories` | 5.x | Platform-specific paths |

**Database Schema:**
- `settings` - User preferences
- `profiles` - Game profile metadata
- `logs` - Session history (optional)

### Utility Crates

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1.x | Async runtime |
| `tracing` | 0.1 | Structured logging |
| `tracing-subscriber` | 0.3 | Log output formatting |
| `anyhow` | 1.x | Error handling |
| `thiserror` | 2.x | Custom error types |
| `clap` | 4.x | CLI argument parsing |

---

## Project Structure

```
gamers-toolkit/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── docs/
│   ├── STACK_PLAN.md
│   └── ARCHITECTURE.md
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library root
│   ├── capture/
│   │   ├── mod.rs
│   │   └── wgc.rs           # Windows Graphics Capture
│   ├── vision/
│   │   ├── mod.rs
│   │   ├── ocr.rs           # PaddleOCR wrapper
│   │   └── template.rs      # Template matching
│   ├── analysis/
│   │   ├── mod.rs
│   │   ├── engine.rs        # Rhai engine setup
│   │   └── rules.rs         # Rule evaluation
│   ├── overlay/
│   │   ├── mod.rs
│   │   ├── window.rs        # Overlay window
│   │   └── widgets.rs       # Tip/alert widgets
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── db.rs            # SQLite operations
│   │   └── config.rs        # Settings management
│   └── profile/
│       ├── mod.rs
│       └── loader.rs        # Profile loading/validation
├── profiles/
│   └── example.json         # Example game profile
├── models/
│   └── .gitkeep             # OCR models directory
└── tests/
    ├── capture_tests.rs
    ├── ocr_tests.rs
    └── integration_tests.rs
```

---

## Build Configuration

### Cargo.toml Features

```toml
[features]
default = ["cpu"]
cpu = []                     # CPU-only inference
cuda = ["ort/cuda"]          # NVIDIA GPU acceleration
directml = ["ort/directml"]  # DirectX ML acceleration
```

### Build Profiles

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

[profile.dev]
opt-level = 1                # Faster dev builds
```

---

## Runtime Requirements

### System
- Windows 10 version 1903+ or Windows 11
- 4GB+ RAM recommended
- ~500MB disk space (including models)

### Optional
- NVIDIA GPU with CUDA 11.x+ (for GPU acceleration)
- DirectX 12 compatible GPU (for DirectML acceleration)

---

## Anti-Cheat Compliance

| Technique | Status | Notes |
|-----------|--------|-------|
| WGC API | Safe | Official Windows API, same as streaming software |
| Overlay window | Safe | Standard Win32 window, no hooks |
| OCR inference | Safe | Local CPU/GPU processing only |
| Memory access | None | No game process interaction |
| Input injection | None | Read-only operation |
| DLL injection | None | Standalone executable |

---

## Development Phases

### Phase 1: Foundation
- [ ] Project scaffolding (Cargo.toml, module structure)
- [ ] Capture layer implementation
- [ ] Basic overlay window

### Phase 2: Vision
- [ ] PaddleOCR integration
- [ ] OCR pipeline (detect -> recognize)
- [ ] Template matching utilities

### Phase 3: Analysis
- [ ] Rhai engine setup
- [ ] Profile schema definition
- [ ] Rule evaluation loop

### Phase 4: Integration
- [ ] End-to-end pipeline
- [ ] Settings UI
- [ ] First game profile

### Phase 5: Polish
- [ ] Performance optimization
- [ ] Error handling improvements
- [ ] Documentation

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Frame capture latency | < 16ms (60 FPS capable) |
| OCR processing time | < 50ms per frame |
| Overlay render time | < 2ms |
| Memory usage | < 500MB |
| CPU overhead | < 5% (modern CPU) |

---

## Security Considerations

1. **Sandboxed scripting**: Rhai has no filesystem/network access by default
2. **Local-only operation**: No network requests, no telemetry
3. **User data control**: All data stored locally, user can delete anytime
4. **No elevated privileges**: Runs as standard user process
