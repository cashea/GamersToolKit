# Overlay Layer TODO

Detailed implementation tasks for the transparent game overlay.

---

## 1. Overlay Window

### Window Configuration
- [x] Create transparent window
- [x] Set always-on-top flag
- [x] Multi-monitor support
- [x] Click-through transparency (GLFW passthrough)
- [ ] Proper DPI awareness
- [ ] Borderless window style

### Positioning
- [ ] Anchor system (top-left, top-right, center, etc.)
- [ ] Offset from anchor point
- [ ] Per-monitor positioning
- [ ] Follow game window mode
- [ ] Save/restore position

### Visibility
- [ ] Global show/hide hotkey
- [ ] Fade in/out animations
- [ ] Auto-hide when game not focused
- [ ] Opacity control

---

## 2. Widget System

### Base Widget
```rust
pub trait OverlayWidget {
    fn id(&self) -> &str;
    fn position(&self) -> Position;
    fn size(&self) -> Size;
    fn render(&self, ui: &mut Ui);
    fn is_visible(&self) -> bool;
}
```

### Tip Widget
- [ ] Text display with background
- [ ] Auto-sizing based on content
- [ ] Configurable colors/fonts
- [ ] Fade out after duration
- [ ] Stack multiple tips

### Alert Widget
- [ ] Prominent styling
- [ ] Icon support
- [ ] Pulsing animation for critical
- [ ] Dismiss button/hotkey
- [ ] Sound trigger integration

### Status Bar Widget
- [ ] Compact horizontal layout
- [ ] Multiple value displays
- [ ] Progress bars
- [ ] Icon indicators

### Info Panel Widget
- [ ] Expandable/collapsible
- [ ] Key-value pairs
- [ ] Scrollable content
- [ ] Pinnable

---

## 3. Layout System

### Layout Modes
- [ ] Fixed position layout
- [ ] Anchor-based layout
- [ ] Flow layout (auto-arrange)
- [ ] Grid layout

### Conflict Resolution
- [ ] Overlap detection
- [ ] Auto-repositioning
- [ ] Z-order management
- [ ] Widget priority

### Responsive Design
- [ ] Scale with game resolution
- [ ] Minimum/maximum sizes
- [ ] Breakpoints for different resolutions

---

## 4. Rendering

### Performance
- [ ] Minimize draw calls
- [ ] Dirty rect optimization
- [ ] Frame rate limiting
- [ ] GPU memory management

### Styling
- [ ] Theme system
- [ ] Color schemes
- [ ] Font configuration
- [ ] Transparency levels

### Animations
- [ ] Fade in/out
- [ ] Slide animations
- [ ] Pulse/glow effects
- [ ] Smooth value transitions

---

## 5. Input Handling

### Hotkeys
- [ ] Global hotkey registration
- [ ] Hotkey configuration UI
- [ ] Default hotkey presets
- [ ] Conflict detection

### Mouse Interaction
- [x] Click-through by default
- [x] Interactive mode toggle (set_click_through / toggle_click_through)
- [ ] Drag to reposition
- [ ] Right-click context menu

### Focus Management
- [ ] Never steal game focus
- [ ] Proper window activation
- [ ] Focus pass-through

---

## 6. Integration

### Message Receiving
- [ ] Receive tips from analysis engine
- [ ] Receive alerts from analysis engine
- [ ] Receive state updates for display
- [ ] Handle overlay commands

### State Display
- [ ] Show current game state values
- [ ] Display detected elements
- [ ] Show OCR debug overlay (dev mode)

### Debug Mode
- [ ] Show OCR bounding boxes
- [ ] Show template match locations
- [ ] Display processing metrics
- [ ] Rule execution trace

---

## 7. Configuration

### Overlay Settings
```rust
pub struct OverlayConfig {
    pub enabled: bool,
    pub opacity: f32,
    pub position: Position,
    pub anchor: Anchor,
    pub scale: f32,
    pub theme: ThemeId,
    pub hotkeys: HotkeyConfig,
}
```

### Per-Widget Settings
- [ ] Position override
- [ ] Visibility toggle
- [ ] Custom styling
- [ ] Behavior options

### Persistence
- [ ] Save overlay config
- [ ] Load on startup
- [ ] Reset to defaults

---

## 8. Testing

### Visual Tests
- [ ] Render verification
- [ ] Layout correctness
- [ ] Animation smoothness

### Performance Tests
- [ ] Frame time benchmarks
- [ ] Memory usage tracking
- [ ] GPU utilization

### Integration Tests
- [ ] Message handling
- [ ] Hotkey functionality
- [ ] Multi-monitor scenarios

---

## API Design

```rust
pub struct Overlay {
    window: OverlayWindow,
    widgets: Vec<Box<dyn OverlayWidget>>,
    config: OverlayConfig,
}

impl Overlay {
    pub fn new(config: OverlayConfig) -> Result<Self>;
    pub fn show_tip(&mut self, tip: Tip);
    pub fn show_alert(&mut self, alert: Alert);
    pub fn update_state(&mut self, state: &DisplayState);
    pub fn set_visible(&mut self, visible: bool);
    pub fn render(&mut self);
}
```

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Render time | < 2ms |
| Input latency | < 5ms |
| Memory overhead | < 50MB |
| GPU usage | < 2% |
