use crate::shared::SharedAppState;
use parking_lot::RwLock;
use rust_mcp_schema::{ReadResourceContent, Resource, TextResourceContents};
use serde_json::json;
use std::sync::Arc;

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
            contents: vec![ReadResourceContent::TextResourceContents(TextResourceContents {
                uri: "profile://active".into(),
                text: result,
                mime_type: Some("application/json".into()),
                meta: None,
            })],
            meta: None,
        })
    }
}
