use crate::shared::SharedAppState;
use parking_lot::RwLock;
use rust_mcp_schema::{ReadResourceContent, Resource, TextResourceContents};
use serde_json::json;
use std::sync::Arc;

// ============================================================================
// profile://active
// ============================================================================

pub struct ProfileActiveResource;

impl ProfileActiveResource {
    pub fn resource() -> Resource {
        Resource {
            uri: "profile://active".into(),
            name: "Active Game Profile".into(),
            description: Some("The currently active game profile configuration".into()),
            mime_type: Some("application/json".into()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
            title: None,
        }
    }

    pub async fn get_resource(
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<rust_mcp_schema::ReadResourceResult, rust_mcp_schema::RpcError> {
        let state_guard = shared_state.read();
        let result = match state_guard.active_profile() {
            Some(profile) => serde_json::to_string_pretty(&profile).unwrap_or_default(),
            None => json!({ "status": "No active profile loaded" }).to_string(),
        };

        Ok(rust_mcp_schema::ReadResourceResult {
            contents: vec![ReadResourceContent::TextResourceContents(
                TextResourceContents {
                    uri: "profile://active".into(),
                    text: result,
                    mime_type: Some("application/json".into()),
                    meta: None,
                },
            )],
            meta: None,
        })
    }
}

// ============================================================================
// config://current
// ============================================================================

pub struct ConfigCurrentResource;

impl ConfigCurrentResource {
    pub fn resource() -> Resource {
        Resource {
            uri: "config://current".into(),
            name: "Current Configuration".into(),
            description: Some(
                "Current application configuration including capture, overlay, vision, and performance settings"
                    .into(),
            ),
            mime_type: Some("application/json".into()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
            title: None,
        }
    }

    pub async fn get_resource(
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<rust_mcp_schema::ReadResourceResult, rust_mcp_schema::RpcError> {
        let state = shared_state.read();
        let config_json = json!({
            "general": {
                "start_minimized": state.config.general.start_minimized,
                "auto_start": state.config.general.auto_start,
                "check_updates": state.config.general.check_updates,
            },
            "capture": {
                "target_window": state.config.capture.target_window,
                "max_fps": state.config.capture.max_fps,
                "capture_cursor": state.config.capture.capture_cursor,
                "draw_border": state.config.capture.draw_border,
            },
            "overlay": {
                "enabled": state.config.overlay.enabled,
                "opacity": state.config.overlay.opacity,
                "sound_enabled": state.config.overlay.sound_enabled,
                "toggle_hotkey": state.config.overlay.toggle_hotkey,
                "max_tips": state.config.overlay.max_tips,
                "default_duration_ms": state.config.overlay.default_duration_ms,
                "click_through": state.config.overlay.click_through,
            },
            "performance": {
                "max_cpu_percent": state.config.performance.max_cpu_percent,
                "max_memory_mb": state.config.performance.max_memory_mb,
                "idle_optimization": state.config.performance.idle_optimization,
            },
            "vision": {
                "backend": format!("{:?}", state.config.vision.backend),
                "granularity": format!("{:?}", state.config.vision.granularity),
                "match_threshold": state.config.vision.match_threshold,
                "auto_run_ocr": state.config.vision.auto_run_ocr,
            },
        });

        let text = serde_json::to_string_pretty(&config_json).unwrap_or_default();

        Ok(rust_mcp_schema::ReadResourceResult {
            contents: vec![ReadResourceContent::TextResourceContents(
                TextResourceContents {
                    uri: "config://current".into(),
                    text,
                    mime_type: Some("application/json".into()),
                    meta: None,
                },
            )],
            meta: None,
        })
    }
}

// ============================================================================
// runtime://status
// ============================================================================

pub struct RuntimeStatusResource;

impl RuntimeStatusResource {
    pub fn resource() -> Resource {
        Resource {
            uri: "runtime://status".into(),
            name: "Runtime Status".into(),
            description: Some(
                "Current runtime status: capture state, overlay state, screen recognition, FPS, and errors"
                    .into(),
            ),
            mime_type: Some("application/json".into()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
            title: None,
        }
    }

    pub async fn get_resource(
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<rust_mcp_schema::ReadResourceResult, rust_mcp_schema::RpcError> {
        let state = shared_state.read();
        let rt = &state.runtime;

        let screen = rt.current_screen.as_ref().map(|s| {
            json!({
                "screen_name": s.screen_name,
                "screen_id": s.screen_id,
                "confidence": s.confidence,
            })
        });

        let status = json!({
            "is_capturing": rt.is_capturing,
            "capture_fps": rt.capture_fps,
            "capture_target": rt.current_capture_target,
            "is_overlay_running": rt.is_overlay_running,
            "overlay_visible": rt.overlay_visible,
            "tips_displayed": rt.tips_displayed,
            "pending_tips_count": rt.pending_tips.len(),
            "current_screen": screen,
            "screen_just_changed": rt.screen_just_changed,
            "last_error": rt.last_error,
            "last_ocr_results_count": rt.last_ocr_results.len(),
            "active_profile_id": state.active_profile_id,
            "profiles_loaded": state.profiles.len(),
        });

        let text = serde_json::to_string_pretty(&status).unwrap_or_default();

        Ok(rust_mcp_schema::ReadResourceResult {
            contents: vec![ReadResourceContent::TextResourceContents(
                TextResourceContents {
                    uri: "runtime://status".into(),
                    text,
                    mime_type: Some("application/json".into()),
                    meta: None,
                },
            )],
            meta: None,
        })
    }
}
