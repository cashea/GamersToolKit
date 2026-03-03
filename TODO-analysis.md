# Analysis Layer TODO

Detailed implementation tasks for the rules engine and game state management.

---

## 1. Rhai Engine Setup

### Dependencies
- [ ] Add `rhai = "1.x"` to Cargo.toml
- [ ] Configure rhai features (sync, no_std options)

### Engine Initialization
- [ ] Create sandboxed rhai::Engine instance
- [ ] Disable dangerous operations:
  - [ ] No file system access
  - [ ] No network access
  - [ ] No process spawning
- [ ] Set resource limits:
  - [ ] Maximum operations per script
  - [ ] Maximum call stack depth
  - [ ] Maximum string length

### Custom Functions
Register these functions for scripts:
- [ ] `get_text(region_id)` - Get OCR text from region
- [ ] `has_element(element_id)` - Check if template detected
- [ ] `get_value(key)` - Get numeric value from parsed text
- [ ] `emit_tip(message)` - Output a tip to overlay
- [ ] `emit_alert(message, priority)` - Trigger an alert
- [ ] `log(message)` - Debug logging
- [ ] `time_since(event_id)` - Time since last event

### Script Compilation
- [ ] Pre-compile scripts on profile load
- [ ] Cache compiled AST
- [ ] Implement hot-reload for development

---

## 2. Game State Management

### State Structure
```rust
pub struct GameState {
    // OCR results by region ID
    pub text_regions: HashMap<String, TextRegion>,

    // Template matches by element ID
    pub elements: HashMap<String, ElementState>,

    // Parsed numeric values
    pub values: HashMap<String, f64>,

    // Event history
    pub events: Vec<GameEvent>,

    // Timestamp
    pub timestamp: Instant,
}
```

### State Updates
- [ ] Define state update pipeline
- [ ] Implement diff detection (what changed)
- [ ] Add state history ring buffer
- [ ] Track value trends (increasing/decreasing)

### Event System
- [ ] Define event types:
  - [ ] `TextChanged { region, old, new }`
  - [ ] `ElementAppeared { element_id }`
  - [ ] `ElementDisappeared { element_id }`
  - [ ] `ValueThresholdCrossed { key, threshold, direction }`
- [ ] Implement event emission
- [ ] Add event listeners for rules

---

## 3. Rules Engine

### Rule Definition
```rust
pub struct RuleDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub priority: i32,
    pub cooldown_ms: u64,
    pub trigger: TriggerType,
    pub script: String,
}

pub enum TriggerType {
    OnFrame,           // Every frame
    OnTextChange(String),  // When specific region changes
    OnElement(String),     // When element appears/disappears
    OnEvent(String),       // On specific event type
}
```

### Rule Execution
- [ ] Create rule executor
- [ ] Implement trigger matching
- [ ] Execute script in sandboxed scope
- [ ] Collect script outputs

### Throttling & Cooldowns
- [ ] Track last execution time per rule
- [ ] Implement cooldown enforcement
- [ ] Add global throttling option
- [ ] Rate limit tip/alert output

### Priority & Ordering
- [ ] Sort rules by priority
- [ ] Support rule dependencies
- [ ] Handle conflicting outputs

---

## 4. Profile Integration

### Rule Loading
- [ ] Parse rules from profile JSON
- [ ] Validate rule scripts
- [ ] Report script errors clearly
- [ ] Support rule inheritance/templates

### Region Mapping
- [ ] Map OCR regions to state keys
- [ ] Map template matches to element IDs
- [ ] Support dynamic region references

### Value Parsing
- [ ] Parse numeric values from OCR text
- [ ] Support format patterns (e.g., "1,234" → 1234)
- [ ] Handle units (HP, MP, %, etc.)
- [ ] Regex-based extraction

---

## 5. Output System

### Tip Generation
- [ ] Define tip message format
- [ ] Support message templates with variables
- [ ] Implement tip deduplication
- [ ] Add tip priority levels

### Alert System
- [ ] Define alert types (info, warning, critical)
- [ ] Support alert sounds (optional)
- [ ] Implement alert queue
- [ ] Add alert acknowledgment

### Output Throttling
- [ ] Rate limit tips per second
- [ ] Deduplicate similar messages
- [ ] Implement message coalescing

---

## 6. API Design

### Public Interface
```rust
pub struct AnalysisEngine {
    rhai: Engine,
    rules: Vec<CompiledRule>,
    state: GameState,
}

impl AnalysisEngine {
    pub fn new() -> Result<Self>;
    pub fn load_profile(&mut self, profile: &Profile) -> Result<()>;
    pub fn update_state(&mut self, vision: &VisionResult);
    pub fn evaluate_rules(&mut self) -> Vec<AnalysisOutput>;
}

pub enum AnalysisOutput {
    Tip { message: String, priority: i32 },
    Alert { message: String, level: AlertLevel },
}
```

---

## 7. Testing

### Unit Tests
- [ ] Rhai engine initialization
- [ ] Custom function registration
- [ ] Script compilation
- [ ] Rule evaluation

### Integration Tests
- [ ] Full analysis pipeline
- [ ] State update flow
- [ ] Event triggering

### Script Tests
- [ ] Sample rule scripts
- [ ] Error handling
- [ ] Performance limits

---

## Example Rule Script

```rhai
// Low health warning rule
let health = get_value("health_bar");
let max_health = get_value("max_health");

if health > 0 && max_health > 0 {
    let health_percent = (health / max_health) * 100.0;

    if health_percent < 25.0 {
        emit_alert("Health critical! Use a potion!", "warning");
    } else if health_percent < 50.0 {
        emit_tip("Health below 50%");
    }
}
```
