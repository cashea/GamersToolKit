# GamersToolKit - Master TODO

> **Status Legend**: 🔴 Not Started | 🟡 In Progress | 🟢 Complete | ⏸️ Blocked

---

## Phase 1: Foundation 🟡

### Capture Layer 🟢
- [x] Windows Graphics Capture API integration
- [x] Window enumeration and selection
- [x] Monitor enumeration and selection
- [x] Frame rate limiting
- [x] BGRA frame buffer capture
- [x] Non-blocking frame delivery via channels

### Overlay Layer 🟡
- [x] Basic egui overlay window
- [x] Multi-monitor support
- [x] Always-on-top positioning
- [x] Click-through transparency
- [x] Overlay positioning/sizing controls
- [ ] Hotkey to toggle visibility

### Dashboard 🟡
- [x] Basic dashboard window with sidebar
- [x] Navigation between views
- [x] Theme system (dark/light)
- [ ] Settings persistence
- [ ] Window state restoration

---

## Phase 2: Vision 🔴

### OCR Integration
- [ ] ONNX Runtime setup with `ort` crate
- [ ] PaddleOCR detection model loading
- [ ] PaddleOCR recognition model loading
- [ ] Image preprocessing pipeline
- [ ] Text detection (bounding box extraction)
- [ ] Text recognition (character inference)
- [ ] Confidence threshold filtering
- [ ] Region-of-interest (ROI) cropping

### Template Matching
- [ ] Template loading from profile assets
- [ ] Multi-scale template matching
- [ ] Icon/element detection
- [ ] Detection caching for performance

### Model Management
- [ ] Model download on first run
- [ ] Model version checking
- [ ] GPU acceleration support (CUDA/DirectML)
- [ ] CPU fallback mode

---

## Phase 3: Analysis 🔴

### Rhai Scripting Engine
- [ ] Rhai engine initialization
- [ ] Custom function registration
- [ ] Sandboxing configuration
- [ ] Script compilation and caching
- [ ] Hot-reload support for development

### Game State Management
- [ ] Game state struct definition
- [ ] State update pipeline
- [ ] State history for trend detection
- [ ] Event emission system

### Rules Engine
- [ ] Rule definition schema
- [ ] Rule evaluation loop
- [ ] Priority and ordering
- [ ] Cooldown/throttling per rule
- [ ] Conditional chaining

---

## Phase 4: Profiles 🔴

### Profile System
- [ ] JSON profile schema definition
- [ ] Profile loading and validation
- [ ] Profile hot-reload
- [ ] Profile selection UI

### Profile Components
- [ ] Screen region definitions
- [ ] OCR zone configurations
- [ ] Template asset references
- [ ] Rule script associations
- [ ] Tip/alert message templates

### First Game Profile
- [ ] Select MVP target game
- [ ] Define HUD regions
- [ ] Create detection rules
- [ ] Write overlay tips
- [ ] Test and validate

---

## Phase 5: Integration 🔴

### End-to-End Pipeline
- [ ] Capture → Vision → Analysis → Overlay flow
- [ ] Frame processing coordinator
- [ ] Async pipeline with backpressure
- [ ] Performance monitoring

### Settings UI
- [ ] General settings panel
- [ ] Capture settings (FPS, target)
- [ ] Overlay settings (position, opacity)
- [ ] Profile management
- [ ] Hotkey configuration

### Persistence
- [ ] SQLite database setup
- [ ] Settings storage
- [ ] Profile metadata storage
- [ ] Session logging (optional)

---

## Phase 6: Polish 🔴

### Performance
- [ ] Profile CPU/memory usage
- [ ] Optimize hot paths
- [ ] Reduce allocations in frame loop
- [ ] GPU memory management

### Error Handling
- [ ] Graceful capture failure recovery
- [ ] OCR error handling
- [ ] User-friendly error messages
- [ ] Crash reporting (local logs)

### Documentation
- [ ] User guide
- [ ] Profile authoring guide
- [ ] API documentation
- [ ] Contribution guidelines

### Distribution
- [ ] Release build configuration
- [ ] Installer/packaging
- [ ] Auto-update mechanism (optional)
- [ ] License and legal review

---

## Backlog / Nice-to-Have

### Features
- [ ] Audio alerts (TTS or sound effects)
- [ ] Screenshot capture for debugging
- [ ] VOD/replay analysis mode
- [ ] Multi-game profile switching
- [ ] Community profile sharing

### Technical
- [ ] Localization/i18n support
- [ ] Accessibility improvements
- [ ] Telemetry opt-in (usage stats)
- [ ] Plugin system for extensions

---

## Known Issues

_No known issues yet._

---

## Notes

- All capture uses official Windows Graphics Capture API (same as OBS, Discord)
- No game memory access, input injection, or automation
- Designed to be anti-cheat compliant (read-only overlay)
