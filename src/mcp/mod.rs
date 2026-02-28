use async_trait::async_trait;
use rust_mcp_schema::{
    CallToolRequestParams, CallToolResult, CompleteRequestParams, CompleteResult,
    ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResult,
};
use rust_mcp_sdk::{
    mcp_server::{server_runtime, McpServerOptions, ServerHandler},
    schema::schema_utils::CallToolError,
    schema::{RpcError, Implementation, InitializeResult, ProtocolVersion, ServerCapabilities, ServerCapabilitiesTools},
    McpServer as McpServerTrait, StdioTransport, ToMcpServerHandler, TransportOptions,
};
use crate::shared::SharedAppState;
use parking_lot::RwLock;
use std::sync::Arc;

pub mod tools;
pub mod prompts;
pub mod resources;

pub struct McpServer {
    shared_state: Arc<RwLock<SharedAppState>>,
}

impl McpServer {
    pub fn new(shared_state: Arc<RwLock<SharedAppState>>) -> Self {
        Self { shared_state }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let server_details = InitializeResult {
            server_info: Implementation {
                name: "GamersToolKit MCP Server".into(),
                version: "0.1.0".into(),
                title: Some("GamersToolKit MCP Server".into()),
                description: Some("Real-time game analysis and assistance overlay".into()),
                icons: vec![],
                website_url: Some("https://github.com/cashea/GamersToolKit".into()),
            },
            capabilities: ServerCapabilities {
                tools: Some(ServerCapabilitiesTools { list_changed: None }),
                resources: None,
                completions: None,
                tasks: None,
                ..Default::default()
            },
            meta: None,
            instructions: Some("This server provides tools to access the read-only game state parsed by GamersToolKit vision.".into()),
            protocol_version: ProtocolVersion::V2024_11_05.into(),
        };

        let transport = StdioTransport::new(TransportOptions::default())
            .map_err(|e| anyhow::anyhow!("Failed to initialize stdio transport: {}", e))?;

        let handler = GamersToolKitHandler {
            shared_state: Arc::clone(&self.shared_state),
        };

        let server = server_runtime::create_server(McpServerOptions {
            server_details,
            transport,
            handler: handler.to_mcp_server_handler(),
            task_store: None,
            client_task_store: None,
        });

        if let Err(e) = server.start().await {
            tracing::error!("Failed to start MCP Server: {}", e);
        }

        Ok(())
    }
}

pub struct GamersToolKitHandler {
    shared_state: Arc<RwLock<SharedAppState>>,
}

#[async_trait]
impl ServerHandler for GamersToolKitHandler {
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServerTrait>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools: tools::GamersToolKitTools::tools(),
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServerTrait>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool_params: tools::GamersToolKitTools =
            tools::GamersToolKitTools::try_from(params).map_err(CallToolError::new)?;

        match tool_params {
            tools::GamersToolKitTools::GetCurrentScreenTool(t) => t.call_tool(Arc::clone(&self.shared_state)),
            tools::GamersToolKitTools::GetActiveProfileTool(t) => t.call_tool(Arc::clone(&self.shared_state)),
            tools::GamersToolKitTools::SendOverlayTipTool(t) => t.call_tool(Arc::clone(&self.shared_state)),
        }
    }

    async fn handle_list_resources_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServerTrait>,
    ) -> std::result::Result<ListResourcesResult, RpcError> {
        Ok(ListResourcesResult {
            meta: None,
            next_cursor: None,
            resources: vec![resources::ProfileActiveResource::resource()],
        })
    }

    async fn handle_list_resource_templates_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServerTrait>,
    ) -> std::result::Result<ListResourceTemplatesResult, RpcError> {
        Ok(ListResourceTemplatesResult {
            meta: None,
            next_cursor: None,
            resource_templates: vec![],
        })
    }

    async fn handle_read_resource_request(
        &self,
        params: ReadResourceRequestParams,
        _runtime: Arc<dyn McpServerTrait>,
    ) -> std::result::Result<ReadResourceResult, RpcError> {
        if params.uri == "profile://active" {
            return resources::ProfileActiveResource::get_resource(Arc::clone(&self.shared_state)).await;
        }

        Err(RpcError::invalid_request()
            .with_message(format!("No resource was found for '{}'.", params.uri)))
    }

    async fn handle_list_prompts_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServerTrait>,
    ) -> std::result::Result<rust_mcp_schema::ListPromptsResult, RpcError> {
        Ok(rust_mcp_schema::ListPromptsResult {
            meta: None,
            next_cursor: None,
            prompts: vec![prompts::AnalyzeGameStatePrompt::prompt()],
        })
    }

    async fn handle_get_prompt_request(
        &self,
        params: rust_mcp_schema::GetPromptRequestParams,
        _runtime: Arc<dyn McpServerTrait>,
    ) -> std::result::Result<rust_mcp_schema::GetPromptResult, RpcError> {
        if params.name == "analyze_game_state" {
            return prompts::AnalyzeGameStatePrompt::get_prompt(
                Arc::clone(&self.shared_state),
                params.arguments,
            )
            .await;
        }

        Err(RpcError::invalid_request()
            .with_message(format!("No prompt was found for '{}'.", params.name)))
    }

    async fn handle_complete_request(
        &self,
        params: CompleteRequestParams,
        _runtime: Arc<dyn McpServerTrait>,
    ) -> std::result::Result<CompleteResult, RpcError> {
        Err(RpcError::method_not_found().with_message(format!(
            "No completion is implemented for '{}'.",
            params.argument.name,
        )))
    }
}
