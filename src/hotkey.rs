//! Global hotkey handling for overlay visibility toggle

use anyhow::{anyhow, Result};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{info, warn};

/// Parses a hotkey string like "F9", "Ctrl+Shift+O", "Alt+F1" into a HotKey
pub fn parse_hotkey(hotkey_str: &str) -> Result<HotKey> {
    let parts: Vec<&str> = hotkey_str.split('+').map(|s| s.trim()).collect();

    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in parts {
        let upper = part.to_uppercase();
        match upper.as_str() {
            "CTRL" | "CONTROL" => modifiers |= Modifiers::CONTROL,
            "SHIFT" => modifiers |= Modifiers::SHIFT,
            "ALT" => modifiers |= Modifiers::ALT,
            "WIN" | "SUPER" | "META" => modifiers |= Modifiers::SUPER,
            _ => {
                // This should be the key code
                key_code = Some(parse_key_code(&upper)?);
            }
        }
    }

    let code = key_code.ok_or_else(|| anyhow!("No key code found in hotkey string"))?;
    Ok(HotKey::new(Some(modifiers), code))
}

/// Parse a key code string into a Code enum
fn parse_key_code(key: &str) -> Result<Code> {
    let code = match key {
        // Function keys
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,

        // Letters
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,

        // Numbers
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,

        // Special keys
        "SPACE" => Code::Space,
        "ENTER" | "RETURN" => Code::Enter,
        "TAB" => Code::Tab,
        "ESCAPE" | "ESC" => Code::Escape,
        "BACKSPACE" => Code::Backspace,
        "DELETE" | "DEL" => Code::Delete,
        "INSERT" | "INS" => Code::Insert,
        "HOME" => Code::Home,
        "END" => Code::End,
        "PAGEUP" | "PGUP" => Code::PageUp,
        "PAGEDOWN" | "PGDN" => Code::PageDown,
        "UP" => Code::ArrowUp,
        "DOWN" => Code::ArrowDown,
        "LEFT" => Code::ArrowLeft,
        "RIGHT" => Code::ArrowRight,

        // Numpad
        "NUMPAD0" | "NUM0" => Code::Numpad0,
        "NUMPAD1" | "NUM1" => Code::Numpad1,
        "NUMPAD2" | "NUM2" => Code::Numpad2,
        "NUMPAD3" | "NUM3" => Code::Numpad3,
        "NUMPAD4" | "NUM4" => Code::Numpad4,
        "NUMPAD5" | "NUM5" => Code::Numpad5,
        "NUMPAD6" | "NUM6" => Code::Numpad6,
        "NUMPAD7" | "NUM7" => Code::Numpad7,
        "NUMPAD8" | "NUM8" => Code::Numpad8,
        "NUMPAD9" | "NUM9" => Code::Numpad9,

        _ => return Err(anyhow!("Unknown key code: {}", key)),
    };

    Ok(code)
}

/// Manages global hotkeys for the application
pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    toggle_hotkey_id: Option<u32>,
    shared_state: Arc<RwLock<crate::shared::SharedAppState>>,
}

impl HotkeyManager {
    /// Create a new hotkey manager
    pub fn new(shared_state: Arc<RwLock<crate::shared::SharedAppState>>) -> Result<Self> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| anyhow!("Failed to create hotkey manager: {:?}", e))?;

        Ok(Self {
            manager,
            toggle_hotkey_id: None,
            shared_state,
        })
    }

    /// Register the overlay toggle hotkey from config
    pub fn register_toggle_hotkey(&mut self) -> Result<()> {
        // Unregister existing hotkey if any
        self.unregister_toggle_hotkey();

        let hotkey_str = {
            let state = self.shared_state.read();
            state.config.overlay.toggle_hotkey.clone()
        };

        if let Some(ref hotkey_str) = hotkey_str {
            match parse_hotkey(hotkey_str) {
                Ok(hotkey) => {
                    self.manager
                        .register(hotkey)
                        .map_err(|e| anyhow!("Failed to register hotkey: {:?}", e))?;

                    self.toggle_hotkey_id = Some(hotkey.id());
                    info!("Registered overlay toggle hotkey: {}", hotkey_str);
                }
                Err(e) => {
                    warn!("Failed to parse hotkey '{}': {}", hotkey_str, e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Unregister the toggle hotkey
    pub fn unregister_toggle_hotkey(&mut self) {
        if let Some(id) = self.toggle_hotkey_id.take() {
            // Get the hotkey string to reconstruct the HotKey for unregistration
            let hotkey_str = {
                let state = self.shared_state.read();
                state.config.overlay.toggle_hotkey.clone()
            };

            if let Some(ref hotkey_str) = hotkey_str {
                if let Ok(hotkey) = parse_hotkey(hotkey_str) {
                    let _ = self.manager.unregister(hotkey);
                }
            }
        }
    }

    /// Process pending hotkey events
    /// Returns true if a toggle event occurred
    pub fn poll_events(&self) -> bool {
        let mut toggled = false;

        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if Some(event.id) == self.toggle_hotkey_id {
                // Toggle overlay visibility
                let mut state = self.shared_state.write();
                state.overlay_config.visible = !state.overlay_config.visible;
                state.runtime.overlay_visible = state.overlay_config.visible;

                info!(
                    "Hotkey pressed: overlay visibility toggled to {}",
                    if state.overlay_config.visible {
                        "visible"
                    } else {
                        "hidden"
                    }
                );
                toggled = true;
            }
        }

        toggled
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.unregister_toggle_hotkey();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_key() {
        let hotkey = parse_hotkey("F9").unwrap();
        assert!(hotkey.id() > 0);
    }

    #[test]
    fn test_parse_with_modifiers() {
        let hotkey = parse_hotkey("Ctrl+Shift+O").unwrap();
        assert!(hotkey.id() > 0);
    }

    #[test]
    fn test_parse_alt_key() {
        let hotkey = parse_hotkey("Alt+F1").unwrap();
        assert!(hotkey.id() > 0);
    }

    #[test]
    fn test_parse_invalid_key() {
        let result = parse_hotkey("InvalidKey");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty() {
        let result = parse_hotkey("");
        assert!(result.is_err());
    }
}
