use crate::shared::SharedAppState;
use parking_lot::RwLock;
use rust_mcp_schema::{
    ContentBlock, GetPromptResult, Prompt, PromptArgument, PromptMessage, Role, TextContent,
};
use serde_json::json;
use std::sync::Arc;

// ============================================================================
// analyze_game_state
// ============================================================================

pub struct AnalyzeGameStatePrompt;

impl AnalyzeGameStatePrompt {
    pub fn prompt() -> Prompt {
        Prompt {
            name: "analyze_game_state".into(),
            description: Some(
                "Requests strategic advice based on the current parsed screen.".into(),
            ),
            arguments: vec![PromptArgument {
                name: "context".into(),
                description: Some("Optional extra context to include in the analysis".into()),
                required: Some(false),
                title: None,
            }],
            icons: vec![],
            meta: None,
            title: None,
        }
    }

    pub async fn get_prompt(
        shared_state: Arc<RwLock<SharedAppState>>,
        arguments: Option<std::collections::HashMap<String, String>>,
    ) -> Result<GetPromptResult, rust_mcp_schema::RpcError> {
        let state = shared_state.read();
        let screen_info = match &state.runtime.current_screen {
            Some(screen) => json!({
                "screen_name": screen.screen_name,
                "confidence": screen.confidence,
                "screen_id": screen.screen_id
            }),
            None => json!({ "status": "No active screen detected" }),
        };

        let ocr_results: Vec<serde_json::Value> = state
            .runtime
            .last_ocr_results
            .iter()
            .map(|(zone, text)| json!({ "zone": zone, "text": text }))
            .collect();

        let extra_context = arguments
            .as_ref()
            .and_then(|a| a.get("context"))
            .cloned()
            .unwrap_or_default();

        let prompt_text = format!(
            "The GamersToolKit vision engine just parsed the current game state:\n\n\
             **Screen:** {}\n\n\
             **OCR Results:** {}\n\n\
             {}\
             What gameplay advice should I give the player? Respond with a short, actionable tip \
             that could be displayed in an overlay.",
            serde_json::to_string_pretty(&screen_info).unwrap_or_default(),
            serde_json::to_string_pretty(&ocr_results).unwrap_or_default(),
            if extra_context.is_empty() {
                String::new()
            } else {
                format!("**Extra context:** {}\n\n", extra_context)
            },
        );

        Ok(GetPromptResult {
            description: Some("Analyzes current screen state and OCR data".into()),
            messages: vec![PromptMessage {
                role: Role::User,
                content: ContentBlock::TextContent(TextContent::new(prompt_text, None, None)),
            }],
            meta: None,
        })
    }
}

// ============================================================================
// create_game_profile
// ============================================================================

pub struct CreateGameProfilePrompt;

impl CreateGameProfilePrompt {
    pub fn prompt() -> Prompt {
        Prompt {
            name: "create_game_profile".into(),
            description: Some(
                "Guides you through creating a new game profile with OCR zones and screen definitions."
                    .into(),
            ),
            arguments: vec![
                PromptArgument {
                    name: "game_name".into(),
                    description: Some("Name of the game to create a profile for".into()),
                    required: Some(true),
                    title: None,
                },
                PromptArgument {
                    name: "executable".into(),
                    description: Some(
                        "Game executable filename (e.g., game.exe) for auto-detection".into(),
                    ),
                    required: Some(false),
                    title: None,
                },
            ],
            icons: vec![],
            meta: None,
            title: None,
        }
    }

    pub async fn get_prompt(
        shared_state: Arc<RwLock<SharedAppState>>,
        arguments: Option<std::collections::HashMap<String, String>>,
    ) -> Result<GetPromptResult, rust_mcp_schema::RpcError> {
        let state = shared_state.read();
        let existing_profiles: Vec<&str> = state.profiles.iter().map(|p| p.name.as_str()).collect();

        let game_name = arguments
            .as_ref()
            .and_then(|a| a.get("game_name"))
            .cloned()
            .unwrap_or_else(|| "Unknown Game".into());

        let executable = arguments
            .as_ref()
            .and_then(|a| a.get("executable"))
            .cloned()
            .unwrap_or_default();

        let prompt_text = format!(
            "I want to create a GamersToolKit profile for **{game_name}**.\n\n\
             {exe_info}\
             **Existing profiles:** {existing}\n\n\
             A game profile defines:\n\
             1. **OCR Zones** — screen regions to read text from (health bars, resource counts, timers, etc.)\n\
             2. **Screen Definitions** — named screens the game can be on (Main Menu, Inventory, Battle, etc.) with text/visual anchors to identify them\n\
             3. **Rules** — Rhai scripts that generate tips based on parsed data\n\n\
             Please help me design this profile by:\n\
             1. Suggesting the most important OCR zones for this game (with approximate screen positions as percentages)\n\
             2. Listing key screens to recognize and what text anchors could identify them\n\
             3. Proposing 2-3 useful rules/tips\n\n\
             Then use the `create_profile`, `add_ocr_region`, and `add_screen` tools to build it.\n\
             Bounds use percentages (0.0-1.0) where (0,0) is top-left of the screen.",
            exe_info = if executable.is_empty() {
                String::new()
            } else {
                format!("**Executable:** `{executable}`\n")
            },
            existing = if existing_profiles.is_empty() {
                "None".to_string()
            } else {
                existing_profiles.join(", ")
            },
        );

        Ok(GetPromptResult {
            description: Some(format!("Create a game profile for {game_name}")),
            messages: vec![PromptMessage {
                role: Role::User,
                content: ContentBlock::TextContent(TextContent::new(prompt_text, None, None)),
            }],
            meta: None,
        })
    }
}
