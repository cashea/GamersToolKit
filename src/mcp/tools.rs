use crate::analysis::Tip;
use crate::capture::{CaptureTarget, capture_frame_once};
use crate::shared::SharedAppState;
use crate::storage::profiles::{
    ContentType, GameProfile, OcrRegion, ScreenDefinition, ScreenMatchMode,
};
use base64::Engine as _;
use parking_lot::RwLock;
use rust_mcp_sdk::macros::JsonSchema;
use rust_mcp_sdk::schema::{
    schema_utils::CallToolError, CallToolResult, ContentBlock, TextContent,
};
use rust_mcp_sdk::{macros::mcp_tool, tool_box};
use serde_json::json;
use std::sync::Arc;

// ============================================================================
// Helper
// ============================================================================

fn ok_json(value: serde_json::Value) -> Result<CallToolResult, CallToolError> {
    let text = serde_json::to_string_pretty(&value)
        .map_err(|e| CallToolError::from_message(format!("JSON serialization error: {e}")))?;
    Ok(CallToolResult::text_content(vec![TextContent::from(text)]))
}

// ============================================================================
// Existing Tools (preserved)
// ============================================================================

//************************//
//  GetCurrentScreenTool  //
//************************//
#[mcp_tool(
    name = "get_current_screen",
    description = "Returns the name and confidence of the current screen detected by the vision engine.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema, Default)]
pub struct GetCurrentScreenTool {}

impl GetCurrentScreenTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let state = shared_state.read();
        let result = match &state.runtime.current_screen {
            Some(screen) => json!({
                "screen_name": screen.screen_name,
                "confidence": screen.confidence,
                "screen_id": screen.screen_id
            }),
            None => json!({ "status": "No active screen detected" }),
        };
        ok_json(result)
    }
}

//************************//
//  GetActiveProfileTool  //
//************************//
#[mcp_tool(
    name = "get_active_profile",
    description = "Returns the full JSON configuration of the currently active GamersToolKit profile, including OCR zones, screens, rules, and templates.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema, Default)]
pub struct GetActiveProfileTool {}

impl GetActiveProfileTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let state = shared_state.read();
        let result = match state.active_profile() {
            Some(profile) => json!(profile),
            None => json!({ "status": "No active profile loaded" }),
        };
        ok_json(result)
    }
}

//**********************//
//  SendOverlayTipTool  //
//**********************//
#[mcp_tool(
    name = "send_overlay_tip",
    description = "Pushes a custom tip/alert to the overlay for the user to see. The tip is queued in shared state and consumed by the overlay when running.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct SendOverlayTipTool {
    /// The tip message to display
    message: String,

    /// Priority from 0-100 (100 is highest). Default 50
    priority: Option<u32>,

    /// How long to display in milliseconds. Default 5000
    duration_ms: Option<u64>,
}

impl SendOverlayTipTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let priority = self.priority.unwrap_or(50);
        let duration_ms = self.duration_ms.unwrap_or(5000);

        let tip = Tip {
            id: uuid::Uuid::new_v4().to_string(),
            message: self.message.clone(),
            priority,
            duration_ms: Some(duration_ms),
            play_sound: false,
        };

        tracing::info!("MCP send_overlay_tip: {}", tip.message);

        // Queue the tip in shared state for the overlay to consume
        shared_state.write().runtime.pending_tips.push(tip);

        ok_json(json!({
            "status": "queued",
            "message": self.message,
            "priority": priority,
            "duration_ms": duration_ms
        }))
    }
}

// ============================================================================
// New Tools
// ============================================================================

//*************************//
//  GetRuntimeStatusTool   //
//*************************//
#[mcp_tool(
    name = "get_runtime_status",
    description = "Returns the full runtime status of GamersToolKit: capture state, overlay state, FPS, current screen, pending tips, last errors, and OCR results.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema, Default)]
pub struct GetRuntimeStatusTool {}

impl GetRuntimeStatusTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let state = shared_state.read();
        let rt = &state.runtime;
        let screen = rt.current_screen.as_ref().map(|s| {
            json!({
                "screen_name": s.screen_name,
                "screen_id": s.screen_id,
                "confidence": s.confidence,
            })
        });

        ok_json(json!({
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
        }))
    }
}

//*********************//
//  ListProfilesTool   //
//*********************//
#[mcp_tool(
    name = "list_profiles",
    description = "Lists all loaded game profiles with their ID, name, executable names, and number of OCR zones, screens, and rules.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema, Default)]
pub struct ListProfilesTool {}

impl ListProfilesTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let state = shared_state.read();
        let profiles: Vec<serde_json::Value> = state
            .profiles
            .iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "name": p.name,
                    "executables": p.executables,
                    "version": p.version,
                    "ocr_regions_count": p.ocr_regions.len(),
                    "screens_count": p.screens.len(),
                    "rules_count": p.rules.len(),
                    "is_active": state.active_profile_id.as_deref() == Some(&p.id),
                })
            })
            .collect();

        ok_json(json!({
            "profiles": profiles,
            "count": profiles.len(),
            "active_profile_id": state.active_profile_id,
        }))
    }
}

//*************************//
//  SetActiveProfileTool   //
//*************************//
#[mcp_tool(
    name = "set_active_profile",
    description = "Switches the active game profile by ID. Pass null/empty to deactivate.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct SetActiveProfileTool {
    /// Profile ID to activate, or empty string to deactivate
    profile_id: String,
}

impl SetActiveProfileTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let mut state = shared_state.write();

        if self.profile_id.is_empty() {
            state.set_active_profile(None);
            return ok_json(json!({ "status": "Profile deactivated" }));
        }

        // Verify profile exists
        let exists = state.profiles.iter().any(|p| p.id == self.profile_id);
        if !exists {
            return ok_json(json!({
                "error": format!("Profile '{}' not found", self.profile_id),
                "available_profiles": state.profiles.iter().map(|p| &p.id).collect::<Vec<_>>(),
            }));
        }

        state.set_active_profile(Some(self.profile_id.clone()));
        ok_json(json!({
            "status": "activated",
            "profile_id": self.profile_id,
        }))
    }
}

//***********************//
//  CreateProfileTool    //
//***********************//
#[mcp_tool(
    name = "create_profile",
    description = "Creates a new game profile with the given name and optional executable names. Returns the new profile's ID.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct CreateProfileTool {
    /// Display name for the game profile
    name: String,
    /// Game executable names for auto-detection (e.g. ["game.exe"])
    executables: Option<Vec<String>>,
}

impl CreateProfileTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let id = uuid::Uuid::new_v4().to_string();
        let profile = GameProfile {
            id: id.clone(),
            name: self.name.clone(),
            executables: self.executables.clone().unwrap_or_default(),
            version: "1.0.0".to_string(),
            ocr_regions: vec![],
            templates: vec![],
            rules: vec![],
            labeled_regions: vec![],
            screens: vec![],
            screen_recognition_enabled: false,
            screen_check_interval_ms: 500,
        };

        // Save to disk
        if let Ok(profiles_dir) = crate::storage::get_profiles_dir() {
            let path = profiles_dir.join(format!("{}.json", id));
            if let Err(e) = crate::storage::profiles::save_profile(&profile, &path) {
                tracing::warn!("Failed to save profile to disk: {}", e);
            }
        }

        shared_state.write().add_profile(profile);

        ok_json(json!({
            "status": "created",
            "profile_id": id,
            "name": self.name,
        }))
    }
}

//***********************//
//  DeleteProfileTool    //
//***********************//
#[mcp_tool(
    name = "delete_profile",
    description = "Deletes a game profile by ID. Removes it from memory and from disk.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct DeleteProfileTool {
    /// ID of the profile to delete
    profile_id: String,
}

impl DeleteProfileTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let removed = shared_state.write().remove_profile(&self.profile_id);

        if removed.is_some() {
            // Also remove from disk
            if let Ok(profiles_dir) = crate::storage::get_profiles_dir() {
                let _ = crate::storage::profiles::delete_profile(&profiles_dir, &self.profile_id);
            }
            ok_json(json!({
                "status": "deleted",
                "profile_id": self.profile_id,
            }))
        } else {
            ok_json(json!({
                "error": format!("Profile '{}' not found", self.profile_id),
            }))
        }
    }
}

//*********************//
//  ListWindowsTool    //
//*********************//
#[mcp_tool(
    name = "list_windows",
    description = "Lists all visible windows available for screen capture. Returns window titles.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema, Default)]
pub struct ListWindowsTool {}

impl ListWindowsTool {
    pub fn call_tool(
        &self,
        _shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        match crate::capture::ScreenCapture::list_windows() {
            Ok(windows) => ok_json(json!({
                "windows": windows,
                "count": windows.len(),
            })),
            Err(e) => ok_json(json!({
                "error": format!("Failed to enumerate windows: {}", e),
            })),
        }
    }
}

//***********************//
//  ListMonitorsTool     //
//***********************//
#[mcp_tool(
    name = "list_monitors",
    description = "Lists all available monitors for screen capture.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema, Default)]
pub struct ListMonitorsTool {}

impl ListMonitorsTool {
    pub fn call_tool(
        &self,
        _shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        match crate::capture::ScreenCapture::list_monitors() {
            Ok(monitors) => ok_json(json!({
                "monitors": monitors,
                "count": monitors.len(),
            })),
            Err(e) => ok_json(json!({
                "error": format!("Failed to enumerate monitors: {}", e),
            })),
        }
    }
}

//*******************//
//  GetConfigTool    //
//*******************//
#[mcp_tool(
    name = "get_config",
    description = "Returns the current application configuration including capture, overlay, vision, and performance settings.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema, Default)]
pub struct GetConfigTool {}

impl GetConfigTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let state = shared_state.read();
        ok_json(json!({
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
                "anchor": format!("{:?}", state.config.overlay.anchor),
                "max_tips": state.config.overlay.max_tips,
                "default_duration_ms": state.config.overlay.default_duration_ms,
                "max_width": state.config.overlay.max_width,
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
        }))
    }
}

//******************************//
//  UpdateOverlayConfigTool     //
//******************************//
#[mcp_tool(
    name = "update_overlay_config",
    description = "Updates overlay settings. Only provided fields are changed; omitted fields keep their current value.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct UpdateOverlayConfigTool {
    /// Overlay opacity (0.0 - 1.0)
    opacity: Option<f32>,
    /// Whether the overlay is enabled
    enabled: Option<bool>,
    /// Maximum number of tips displayed at once
    max_tips: Option<u32>,
    /// Default tip duration in milliseconds
    default_duration_ms: Option<u64>,
    /// Whether clicks pass through the overlay
    click_through: Option<bool>,
}

impl UpdateOverlayConfigTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let mut state = shared_state.write();
        let mut changed = vec![];

        if let Some(opacity) = self.opacity {
            state.config.overlay.opacity = opacity.clamp(0.0, 1.0);
            state.overlay_config.opacity = state.config.overlay.opacity;
            changed.push("opacity");
        }
        if let Some(enabled) = self.enabled {
            state.config.overlay.enabled = enabled;
            state.overlay_config.enabled = enabled;
            changed.push("enabled");
        }
        if let Some(max_tips) = self.max_tips {
            state.config.overlay.max_tips = max_tips as usize;
            state.overlay_config.max_tips = max_tips as usize;
            changed.push("max_tips");
        }
        if let Some(duration) = self.default_duration_ms {
            state.config.overlay.default_duration_ms = duration;
            state.overlay_config.default_duration_ms = duration;
            changed.push("default_duration_ms");
        }
        if let Some(click_through) = self.click_through {
            state.config.overlay.click_through = click_through;
            state.overlay_config.click_through = click_through;
            changed.push("click_through");
        }

        ok_json(json!({
            "status": "updated",
            "changed_fields": changed,
        }))
    }
}

//***********************//
//  GetOcrRegionsTool    //
//***********************//
#[mcp_tool(
    name = "get_ocr_regions",
    description = "Lists all OCR zones defined in the active profile with their bounds, content type, and enabled status.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema, Default)]
pub struct GetOcrRegionsTool {}

impl GetOcrRegionsTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let state = shared_state.read();
        match state.active_profile() {
            Some(profile) => {
                let regions: Vec<serde_json::Value> = profile
                    .ocr_regions
                    .iter()
                    .map(|r| {
                        json!({
                            "id": r.id,
                            "name": r.name,
                            "bounds": { "x": r.bounds.0, "y": r.bounds.1, "w": r.bounds.2, "h": r.bounds.3 },
                            "content_type": format!("{:?}", r.content_type),
                            "enabled": r.enabled,
                        })
                    })
                    .collect();

                ok_json(json!({
                    "profile_id": profile.id,
                    "profile_name": profile.name,
                    "ocr_regions": regions,
                    "count": regions.len(),
                }))
            }
            None => ok_json(json!({ "error": "No active profile" })),
        }
    }
}

//***********************//
//  AddOcrRegionTool     //
//***********************//
#[mcp_tool(
    name = "add_ocr_region",
    description = "Adds an OCR zone to the active profile. Bounds are percentages of screen (0.0-1.0). Content type: Text, Number, Percentage, or Time.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct AddOcrRegionTool {
    /// User-friendly name for this zone
    name: String,
    /// Left edge as percentage of screen width (0.0 - 1.0)
    x: f32,
    /// Top edge as percentage of screen height (0.0 - 1.0)
    y: f32,
    /// Width as percentage of screen width (0.0 - 1.0)
    w: f32,
    /// Height as percentage of screen height (0.0 - 1.0)
    h: f32,
    /// Content type: "Text", "Number", "Percentage", or "Time"
    content_type: Option<String>,
}

impl AddOcrRegionTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let mut state = shared_state.write();
        let profile_id = state.active_profile_id.clone();

        let profile_id = match profile_id {
            Some(id) => id,
            None => return ok_json(json!({ "error": "No active profile" })),
        };

        let ct = match self.content_type.as_deref() {
            Some("Number") => ContentType::Number,
            Some("Percentage") => ContentType::Percentage,
            Some("Time") => ContentType::Time,
            _ => ContentType::Text,
        };

        let zone_id = uuid::Uuid::new_v4().to_string();
        let region = OcrRegion {
            id: zone_id.clone(),
            name: self.name.clone(),
            bounds: (self.x, self.y, self.w, self.h),
            content_type: ct,
            enabled: true,
            preprocessing: None,
        };

        if let Some(profile) = state.profiles.iter_mut().find(|p| p.id == profile_id) {
            profile.ocr_regions.push(region);

            // Persist to disk
            if let Ok(dir) = crate::storage::get_profiles_dir() {
                let _ =
                    crate::storage::profiles::save_profile(profile, &dir.join(format!("{}.json", profile_id)));
            }

            ok_json(json!({
                "status": "added",
                "zone_id": zone_id,
                "name": self.name,
            }))
        } else {
            ok_json(json!({ "error": "Active profile not found in memory" }))
        }
    }
}

//**************************//
//  RemoveOcrRegionTool     //
//**************************//
#[mcp_tool(
    name = "remove_ocr_region",
    description = "Removes an OCR zone from the active profile by zone ID.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct RemoveOcrRegionTool {
    /// ID of the OCR zone to remove
    zone_id: String,
}

impl RemoveOcrRegionTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let mut state = shared_state.write();
        let profile_id = state.active_profile_id.clone();

        let profile_id = match profile_id {
            Some(id) => id,
            None => return ok_json(json!({ "error": "No active profile" })),
        };

        if let Some(profile) = state.profiles.iter_mut().find(|p| p.id == profile_id) {
            let before = profile.ocr_regions.len();
            profile.ocr_regions.retain(|r| r.id != self.zone_id);
            let removed = before - profile.ocr_regions.len();

            if removed > 0 {
                // Persist
                if let Ok(dir) = crate::storage::get_profiles_dir() {
                    let _ = crate::storage::profiles::save_profile(
                        profile,
                        &dir.join(format!("{}.json", profile_id)),
                    );
                }

                ok_json(json!({
                    "status": "removed",
                    "zone_id": self.zone_id,
                }))
            } else {
                ok_json(json!({
                    "error": format!("Zone '{}' not found in profile", self.zone_id),
                }))
            }
        } else {
            ok_json(json!({ "error": "Active profile not found in memory" }))
        }
    }
}

//*********************//
//  ListScreensTool    //
//*********************//
#[mcp_tool(
    name = "list_screens",
    description = "Lists all screen definitions in the active profile with their ID, name, match mode, anchor count, and hierarchy info.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema, Default)]
pub struct ListScreensTool {}

impl ListScreensTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let state = shared_state.read();
        match state.active_profile() {
            Some(profile) => {
                let screens: Vec<serde_json::Value> = profile
                    .screens
                    .iter()
                    .map(|s| {
                        json!({
                            "id": s.id,
                            "name": s.name,
                            "parent_id": s.parent_id,
                            "match_mode": format!("{:?}", s.match_mode),
                            "anchors_count": s.anchors.len(),
                            "match_threshold": s.match_threshold,
                            "enabled": s.enabled,
                            "priority": s.priority,
                            "ocr_zone_overrides_count": s.ocr_zone_overrides.len(),
                            "rules_to_trigger": s.rules_to_trigger,
                        })
                    })
                    .collect();

                ok_json(json!({
                    "profile_id": profile.id,
                    "screens": screens,
                    "count": screens.len(),
                    "screen_recognition_enabled": profile.screen_recognition_enabled,
                }))
            }
            None => ok_json(json!({ "error": "No active profile" })),
        }
    }
}

//*********************//
//  AddScreenTool      //
//*********************//
#[mcp_tool(
    name = "add_screen",
    description = "Adds a screen definition to the active profile for screen recognition. Uses anchor-based matching by default.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct AddScreenTool {
    /// Display name (e.g., "Main Menu", "Inventory")
    name: String,
    /// Parent screen ID for hierarchy (optional)
    parent_id: Option<String>,
    /// Minimum confidence threshold (0.0-1.0). Default 0.8
    match_threshold: Option<f32>,
    /// Priority for matching order (higher = checked first). Default 0
    priority: Option<u32>,
    /// Whether to show overlay notification on detection. Default true
    show_notification: Option<bool>,
}

impl AddScreenTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let mut state = shared_state.write();
        let profile_id = state.active_profile_id.clone();

        let profile_id = match profile_id {
            Some(id) => id,
            None => return ok_json(json!({ "error": "No active profile" })),
        };

        let screen_id = uuid::Uuid::new_v4().to_string();
        let screen = ScreenDefinition {
            id: screen_id.clone(),
            name: self.name.clone(),
            parent_id: self.parent_id.clone(),
            match_mode: ScreenMatchMode::Anchors,
            anchors: vec![],
            full_template: None,
            match_threshold: self.match_threshold.unwrap_or(0.8),
            enabled: true,
            priority: self.priority.unwrap_or(0),
            ocr_zone_overrides: vec![],
            rules_to_trigger: vec![],
            show_notification: self.show_notification.unwrap_or(true),
        };

        if let Some(profile) = state.profiles.iter_mut().find(|p| p.id == profile_id) {
            profile.screens.push(screen);

            // Persist
            if let Ok(dir) = crate::storage::get_profiles_dir() {
                let _ = crate::storage::profiles::save_profile(
                    profile,
                    &dir.join(format!("{}.json", profile_id)),
                );
            }

            ok_json(json!({
                "status": "added",
                "screen_id": screen_id,
                "name": self.name,
            }))
        } else {
            ok_json(json!({ "error": "Active profile not found in memory" }))
        }
    }
}

//****************************//
//  GetLastOcrResultsTool     //
//****************************//
#[mcp_tool(
    name = "get_last_ocr_results",
    description = "Returns the last OCR text results from the vision pipeline, organized by zone ID.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema, Default)]
pub struct GetLastOcrResultsTool {}

impl GetLastOcrResultsTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let state = shared_state.read();
        let results: Vec<serde_json::Value> = state
            .runtime
            .last_ocr_results
            .iter()
            .map(|(zone_id, text)| {
                json!({
                    "zone_id": zone_id,
                    "text": text,
                })
            })
            .collect();

        ok_json(json!({
            "results": results,
            "count": results.len(),
        }))
    }
}

//******************************//
//  CaptureScreenshotTool      //
//******************************//
#[mcp_tool(
    name = "capture_screenshot",
    description = "Captures a screenshot of the user's screen and returns it as an image. Can capture the active capture target, a specific window by title, or a monitor. Returns JPEG image data that can be analyzed for visual context."
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct CaptureScreenshotTool {
    /// Capture source: "primary_monitor", "monitor:0", "monitor:1", or a window title.
    /// If omitted, uses the current capture target when capture is running, otherwise primary monitor.
    source: Option<String>,

    /// Maximum image width in pixels. Images wider than this are downscaled proportionally.
    /// Default: 1920
    max_width: Option<u32>,

    /// JPEG compression quality (1-100). Lower = smaller file, less detail. Default: 75
    quality: Option<u32>,
}

impl CaptureScreenshotTool {
    pub fn call_tool(
        &self,
        shared_state: Arc<RwLock<SharedAppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let quality = self.quality.unwrap_or(75).clamp(1, 100);
        let max_width = self.max_width.unwrap_or(1920).max(100);

        // Determine capture target
        let target = match &self.source {
            Some(src) => Self::parse_source(src),
            None => {
                // Use current capture target if capturing, otherwise primary monitor
                let state = shared_state.read();
                if state.runtime.is_capturing {
                    state.capture_config.target.clone()
                } else {
                    CaptureTarget::PrimaryMonitor
                }
            }
        };

        let source_desc = format!("{:?}", target);

        // Try to get a frame: use cached frame if capture is active, otherwise one-shot
        let frame = {
            let state = shared_state.read();
            if state.runtime.is_capturing {
                state.runtime.last_captured_frame.as_ref().map(|f| f.as_ref().clone())
            } else {
                None
            }
        };

        let frame = match frame {
            Some(f) => f,
            None => {
                // One-shot capture
                capture_frame_once(&target).map_err(|e| {
                    CallToolError::from_message(format!("Screenshot capture failed: {}", e))
                })?
            }
        };

        // Convert to image
        let rgba_img = frame.to_rgba_image().ok_or_else(|| {
            CallToolError::from_message("Failed to convert captured frame to image".to_string())
        })?;

        // Resize if needed
        let (orig_w, orig_h) = (rgba_img.width(), rgba_img.height());
        let final_img = if orig_w > max_width {
            let scale = max_width as f32 / orig_w as f32;
            let new_h = (orig_h as f32 * scale) as u32;
            image::imageops::resize(
                &rgba_img,
                max_width,
                new_h,
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            rgba_img
        };

        let (final_w, final_h) = (final_img.width(), final_img.height());

        // Encode to JPEG
        let mut jpeg_buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut jpeg_buffer);
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality as u8);
        image::DynamicImage::ImageRgba8(final_img)
            .write_with_encoder(encoder)
            .map_err(|e| {
                CallToolError::from_message(format!("JPEG encoding failed: {}", e))
            })?;

        // Base64 encode
        let b64_data = base64::engine::general_purpose::STANDARD.encode(&jpeg_buffer);

        // Build metadata text
        let metadata = serde_json::to_string_pretty(&json!({
            "width": final_w,
            "height": final_h,
            "original_width": orig_w,
            "original_height": orig_h,
            "format": "jpeg",
            "quality": quality,
            "size_bytes": jpeg_buffer.len(),
            "source": source_desc,
        }))
        .unwrap_or_default();

        // Return mixed content: text metadata + image
        Ok(CallToolResult {
            content: vec![
                ContentBlock::text_content(metadata),
                ContentBlock::image_content(b64_data, "image/jpeg".to_string()),
            ],
            is_error: None,
            meta: None,
            structured_content: None,
        })
    }

    fn parse_source(source: &str) -> CaptureTarget {
        match source.to_lowercase().as_str() {
            "primary_monitor" | "primary" => CaptureTarget::PrimaryMonitor,
            s if s.starts_with("monitor:") => {
                if let Ok(idx) = s.trim_start_matches("monitor:").parse::<usize>() {
                    CaptureTarget::MonitorIndex(idx)
                } else {
                    CaptureTarget::PrimaryMonitor
                }
            }
            title => CaptureTarget::Window(title.to_string()),
        }
    }
}

// ============================================================================
// Tool Box Registration
// ============================================================================

tool_box!(
    GamersToolKitTools,
    [
        GetCurrentScreenTool,
        GetActiveProfileTool,
        SendOverlayTipTool,
        GetRuntimeStatusTool,
        ListProfilesTool,
        SetActiveProfileTool,
        CreateProfileTool,
        DeleteProfileTool,
        ListWindowsTool,
        ListMonitorsTool,
        GetConfigTool,
        UpdateOverlayConfigTool,
        GetOcrRegionsTool,
        AddOcrRegionTool,
        RemoveOcrRegionTool,
        ListScreensTool,
        AddScreenTool,
        GetLastOcrResultsTool,
        CaptureScreenshotTool
    ]
);
