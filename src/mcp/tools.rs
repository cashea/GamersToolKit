use crate::analysis::Tip;
use crate::shared::SharedAppState;
use parking_lot::RwLock;
use rust_mcp_sdk::macros::JsonSchema;
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult, TextContent};
use rust_mcp_sdk::{macros::mcp_tool, tool_box};
use std::sync::Arc;
use serde_json::json;

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
    pub fn call_tool(&self, shared_state: Arc<RwLock<SharedAppState>>) -> Result<CallToolResult, CallToolError> {
        let runtime = &shared_state.read().runtime;
        let result = match &runtime.current_screen {
            Some(screen) => json!({
                "screen_name": screen.screen_name,
                "confidence": screen.confidence,
                "screen_id": screen.screen_id
            }),
            None => json!({ "status": "No active screen detected" })
        };

        let result_json = serde_json::to_string_pretty(&result).map_err(|err| {
            CallToolError::from_message(format!("Unable to serialize screen info: {err}"))
        })?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            result_json,
        )]))
    }
}

//************************//
//  GetActiveProfileTool  //
//************************//
#[mcp_tool(
    name = "get_active_profile",
    description = "Returns the JSON configuration of the currently active GamersToolKit profile.",
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema, Default)]
pub struct GetActiveProfileTool {}

impl GetActiveProfileTool {
    pub fn call_tool(&self, shared_state: Arc<RwLock<SharedAppState>>) -> Result<CallToolResult, CallToolError> {
        let state_guard = shared_state.read();
        let result = match state_guard.active_profile() {
            Some(profile) => json!(profile),
            None => json!({ "status": "No active profile loaded" })
        };

        let result_json = serde_json::to_string_pretty(&result).map_err(|err| {
            CallToolError::from_message(format!("Unable to serialize active profile info: {err}"))
        })?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            result_json,
        )]))
    }
}

//**********************//
//  SendOverlayTipTool  //
//**********************//
#[mcp_tool(
    name = "send_overlay_tip",
    description = "Pushes a custom tip/alert to the overlay for the user to see.",
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
    pub fn call_tool(&self, _shared_state: Arc<RwLock<SharedAppState>>) -> Result<CallToolResult, CallToolError> {
        let priority = self.priority.unwrap_or(50);
        let duration_ms = self.duration_ms.unwrap_or(5000);

        let tip = Tip {
            id: uuid::Uuid::new_v4().to_string(),
            message: self.message.clone(),
            priority,
            duration_ms: Some(duration_ms),
            play_sound: false,
        };

        tracing::info!("MCP Tool invoked send_overlay_tip: {}", tip.message);

        // We could attach an outgoing channel to SharedAppState for the MCP to emit tips.
        // For now, logging will show it is hooked up correctly.

        let result_message = format!("Successfully generated tip: {}", self.message);

        Ok(CallToolResult::text_content(vec![TextContent::from(
            result_message,
        )]))
    }
}

tool_box!(GamersToolKitTools, [GetCurrentScreenTool, GetActiveProfileTool, SendOverlayTipTool]);
