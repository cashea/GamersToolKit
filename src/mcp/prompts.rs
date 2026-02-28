use crate::shared::SharedAppState;
use parking_lot::RwLock;
use rust_mcp_schema::{
    ContentBlock, GetPromptResult, Prompt, PromptArgument, PromptMessage, Role, TextContent,
};
use serde_json::json;
use std::sync::Arc;

pub struct AnalyzeGameStatePrompt;

impl AnalyzeGameStatePrompt {
    pub fn prompt() -> Prompt {
        Prompt {
            name: "analyze_game_state".into(),
            description: Some("Requests strategic advice based on the current parsed screen.".into()),
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
        _arguments: Option<std::collections::HashMap<String, String>>,
    ) -> Result<GetPromptResult, rust_mcp_schema::RpcError> {
        let runtime = &shared_state.read().runtime;
        let screen_info = match &runtime.current_screen {
            Some(screen) => json!({
                "screen_name": screen.screen_name,
                "confidence": screen.confidence,
                "screen_id": screen.screen_id
            }),
            None => json!({ "status": "No active screen detected" }),
        };

        let prompt_text = format!(
            "The GamersToolKit vision engine just parsed the current screen state as follows:\n\n{}\n\nWhat gameplay advice should I give the player? Respond with a short, actionable tip that could be displayed in an overlay.",
            serde_json::to_string_pretty(&screen_info).unwrap_or_default()
        );

        Ok(GetPromptResult {
            description: Some("Analyzes current screen state".into()),
            messages: vec![PromptMessage {
                role: Role::User,
                content: ContentBlock::TextContent(TextContent::new(prompt_text, None, None)),
            }],
            meta: None,
        })
    }
}
