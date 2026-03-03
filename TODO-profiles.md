# Game Profiles TODO

Detailed implementation tasks for the game profile system.

---

## 0. Zone OCR Integration with Profiles (Priority)

### Current State
- Zone OCR definitions (`OcrRegion`) are stored in `GameProfile.ocr_regions`
- Zones are auto-saved to the active profile via `auto_save_profile_zones()` in `app.rs:462-492`
- When loading a profile, zones are extracted via `load_profile_by_id()` in `app.rs:185-254`

### Issues Fixed ✓
- [x] **Profile switching doesn't load zones**: Implemented `activate_profile()` method that loads zones from new profile
- [x] **Activate/Deactivate buttons are non-functional**: Connected buttons via `ProfileAction` enum and `process_profile_commands()`
- [x] **Zones only load on app startup**: Profile zones now load when switching profiles at runtime

### Implementation Completed ✓
- [x] Implement `activate_profile()` method in `DashboardApp` (app.rs:256-302):
  1. Saves current zones to the current active profile (if any)
  2. Updates `shared_state.active_profile_id`
  3. Loads `ocr_regions` from the new profile into `dashboard_state.vision.ocr_zones`
  4. Clears any pending zone OCR results
  5. Persists the active profile ID to config
- [x] Implement `deactivate_profile()` method (app.rs:305-335):
  1. Saves current zones to the active profile
  2. Clears `shared_state.active_profile_id`
  3. Clears zones from vision state
- [x] Add `ProfileAction` enum in state.rs for UI-to-app communication
- [x] Connect the Activate/Deactivate buttons in profiles view (profiles.rs:283, 294)
- [x] Add `process_profile_commands()` in app.rs:931-947 to handle profile actions
- [ ] Consider adding a confirmation dialog when switching profiles if there are unsaved zone changes (optional enhancement)

### Testing
- [ ] Create profile A with 2 zones, profile B with 3 zones
- [ ] Switch between profiles and verify zones update correctly
- [ ] Verify zones persist after app restart
- [ ] Test creating new zones after switching profiles

---

## 0.5. Zone Auto-Configure Feature (Priority)

### Overview
Add an auto-configure button for individual zone settings that automatically adjusts OCR parameters until text is successfully detected.

### Implementation Completed ✓
- [x] Add "Auto Configure" button to zone settings dialog (`zone_ocr.rs:548`)
- [x] Implement auto-configure algorithm that iterates through settings combinations (`app.rs:1369-1615`):
  - [x] Adjust upscale factor (1x, 2x, 3x, 4x)
  - [x] Try different preprocessing modes (enabled/disabled, grayscale, invert)
  - [x] Adjust contrast values (1.0, 1.5, 2.0)
- [x] Add visual feedback during auto-configuration:
  - [x] Show spinner and status message with current settings being tested
  - [x] Show progress bar (% complete)
  - [x] Allow cancellation via Cancel button
- [x] Stop iteration once OCR returns non-empty text (filtered by content type)
- [x] Apply successful settings to the zone (sets `zone.preprocessing`)
- [x] Handle failure case:
  - [x] Show error message when no settings produce text
  - [x] Show success message with detected text when found

### Algorithm Implementation
```
1. Capture current zone region from frame
2. For each scale in [1, 2, 3, 4]:
   For each preprocessing_enabled in [false, true]:
     For each grayscale in [false, true]:
       For each invert in [false, true]:
         For each contrast in [1.0, 1.5, 2.0]:
           Apply settings temporarily
           Run OCR on region
           Filter text by content type
           Calculate average confidence score
           If filtered text is non-empty AND confidence > best_confidence:
             Update best_preprocessing, best_confidence, best_text
3. After all combinations tested:
   If best_preprocessing found:
     Apply best settings to zone
     Show success with confidence %
   Else:
     Show error to user
```

### State Management
- `AutoConfigureState` struct in `state.rs:352-385` tracks:
  - Zone index being configured
  - Current step (Starting, Testing, Completed)
  - Current settings being tested (scale, preprocessing, grayscale, invert, contrast)
  - Progress tracking (current/total combinations)
  - Result status (success/error message)
  - Best configuration found (best_preprocessing, best_confidence, best_text)

### UI Implementation
- [x] Button in zone settings dialog (under preprocessing settings)
- [x] Progress bar and status during auto-configure
- [x] Cancel button while running
- [x] Success/error message after completion

### Testing
- [ ] Test with zones that have visible text
- [ ] Test with zones that have no text (should fail gracefully)
- [ ] Test cancellation mid-process
- [ ] Verify settings persist after successful auto-configure

---

## 1. Profile Schema

### Profile Structure
```json
{
  "id": "game-name",
  "name": "Game Name",
  "version": "1.0.0",
  "game": {
    "executable": "game.exe",
    "window_title": "Game Window",
    "resolutions": ["1920x1080", "2560x1440"]
  },
  "regions": [...],
  "templates": [...],
  "rules": [...],
  "settings": {...}
}
```

### Schema Definition
- [ ] Define JSON schema for validation
- [ ] Create Rust structs with serde
- [ ] Implement schema versioning
- [ ] Add migration support for older profiles

---

## 2. Screen Regions

### Region Definition
```json
{
  "id": "health_bar",
  "name": "Health Bar",
  "type": "ocr",
  "bounds": {
    "x": 100,
    "y": 50,
    "width": 200,
    "height": 30
  },
  "scaling": "proportional",
  "ocr_config": {
    "preprocessing": "threshold",
    "charset": "0123456789/",
    "pattern": "(\\d+)/(\\d+)"
  }
}
```

### Region Types
- [ ] OCR region (text extraction)
- [ ] Template region (icon detection)
- [ ] Composite region (multiple checks)
- [ ] Dynamic region (position from template)

### Scaling Modes
- [ ] Fixed position (absolute pixels)
- [ ] Proportional (percentage of screen)
- [ ] Anchored (relative to corner)
- [ ] Template-relative (offset from detected element)

---

## 3. Template Assets

### Template Definition
```json
{
  "id": "potion_icon",
  "name": "Health Potion",
  "file": "assets/potion.png",
  "scales": [1.0, 0.75, 0.5],
  "threshold": 0.85,
  "search_region": "inventory_area"
}
```

### Asset Management
- [ ] Template image loading
- [ ] Multi-scale preprocessing
- [ ] Asset validation
- [ ] Missing asset handling

### Search Optimization
- [ ] Region-limited search
- [ ] Priority ordering
- [ ] Early termination on match

---

## 4. Rule Definitions

### Rule Structure
```json
{
  "id": "low_health_warning",
  "name": "Low Health Warning",
  "enabled": true,
  "priority": 10,
  "trigger": {
    "type": "value_change",
    "region": "health_bar"
  },
  "cooldown_ms": 5000,
  "script": "..."
}
```

### Trigger Types
- [ ] `on_frame` - Every processed frame
- [ ] `on_change` - When region value changes
- [ ] `on_appear` - When template detected
- [ ] `on_disappear` - When template lost
- [ ] `on_threshold` - When value crosses threshold

### Script Loading
- [ ] Inline scripts in JSON
- [ ] External script file references
- [ ] Script includes/imports

---

## 5. Profile Loading

### Loader Implementation
- [ ] Load profile from JSON file
- [ ] Validate against schema
- [ ] Load referenced assets
- [ ] Compile rule scripts
- [ ] Report errors clearly

### Profile Discovery
- [ ] Scan profiles directory
- [ ] Auto-detect game by window title
- [ ] Manual profile selection

### Hot Reload
- [ ] Watch profile files for changes
- [ ] Reload without restart
- [ ] Preserve runtime state

---

## 6. Profile Settings

### User Overrides
```json
{
  "profile_id": "game-name",
  "overrides": {
    "rules": {
      "low_health_warning": {
        "enabled": false
      }
    },
    "regions": {
      "health_bar": {
        "bounds": { "x": 110, "y": 55 }
      }
    }
  }
}
```

### Settings Features
- [ ] Enable/disable individual rules
- [ ] Adjust region positions
- [ ] Modify thresholds
- [ ] Custom keybindings per profile

---

## 7. Profile Editor (Future)

### UI Features
- [ ] Visual region editor
- [ ] Template capture tool
- [ ] Rule testing sandbox
- [ ] Live preview

### Workflow
- [ ] Capture screenshot
- [ ] Draw regions on screenshot
- [ ] Test OCR on captured regions
- [ ] Export profile JSON

---

## 8. Example Profiles

### MVP Profile Tasks
- [ ] Select target game
- [ ] Screenshot HUD elements
- [ ] Define key regions:
  - [ ] Health/HP
  - [ ] Mana/MP/Energy
  - [ ] Experience/Level
  - [ ] Minimap indicators
- [ ] Create template assets:
  - [ ] Important icons
  - [ ] Status indicators
- [ ] Write rules:
  - [ ] Low health warning
  - [ ] Buff/debuff tracking
  - [ ] Resource management tips

---

## 9. Testing

### Validation Tests
- [ ] Schema validation
- [ ] Required field checks
- [ ] Asset existence checks
- [ ] Script compilation tests

### Loading Tests
- [ ] Valid profile loading
- [ ] Invalid profile error handling
- [ ] Missing asset handling
- [ ] Partial profile support

---

## Directory Structure

```
profiles/
├── built-in/
│   └── example-game/
│       ├── profile.json
│       ├── assets/
│       │   ├── icon1.png
│       │   └── icon2.png
│       └── scripts/
│           └── rules.rhai
└── user/
    └── my-custom-profile/
        ├── profile.json
        └── assets/
```

---

## Profile JSON Schema (Draft)

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["id", "name", "version"],
  "properties": {
    "id": { "type": "string", "pattern": "^[a-z0-9-]+$" },
    "name": { "type": "string" },
    "version": { "type": "string", "pattern": "^\\d+\\.\\d+\\.\\d+$" },
    "game": {
      "type": "object",
      "properties": {
        "executable": { "type": "string" },
        "window_title": { "type": "string" }
      }
    },
    "regions": {
      "type": "array",
      "items": { "$ref": "#/definitions/region" }
    },
    "templates": {
      "type": "array",
      "items": { "$ref": "#/definitions/template" }
    },
    "rules": {
      "type": "array",
      "items": { "$ref": "#/definitions/rule" }
    }
  }
}
```
