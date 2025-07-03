use crate::{AppState, McpServerEdit};
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters, wrapper::Json},
    model::{ServerCapabilities, ServerInfo},
    transport::sse_server::{SseServer, SseServerConfig},
    schemars, tool, tool_handler, tool_router,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Sanitized server info for MCP protocol (shows env keys but not values)
#[derive(Debug, serde::Serialize)]
pub struct McpServerInfoSanitized {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env_keys: Vec<String>, // Environment variable keys without values
}

impl McpServerInfoSanitized {
    // Convert from McpServerInfo, showing env keys but hiding values
    fn from_server_info(server_info: &crate::McpServerInfo) -> Self {
        Self {
            name: server_info.name.clone(),
            command: server_info.command.clone(),
            args: server_info.args.clone(),
            env_keys: server_info.env.keys().cloned().collect(),
        }
    }
}

// Sanitized preset server info for MCP protocol (shows env keys but not values)
#[derive(Debug, serde::Serialize)]
pub struct PresetServerSanitized {
    pub name: String,
    pub description: String,
    pub category: String,
    #[serde(rename = "serverType")]
    pub server_type: String,
    pub command: String,
    pub args: Vec<String>,
    pub env_keys: Vec<String>, // Environment variable keys without values
    #[serde(rename = "apiKeys")]
    pub api_keys: Vec<crate::ApiKeyRequirement>,
    #[serde(rename = "requiresApiKey")]
    pub requires_api_key: bool,
}

impl PresetServerSanitized {
    // Convert from PresetServer, showing env keys but hiding values
    fn from_preset_server(preset: &crate::PresetServer) -> Self {
        Self {
            name: preset.name.clone(),
            description: preset.description.clone(),
            category: preset.category.clone(),
            server_type: preset.server_type.to_string(),
            command: preset.command.clone(),
            args: preset.args.clone(),
            env_keys: preset.env.as_ref()
                .map(|env| env.keys().cloned().collect())
                .unwrap_or_else(Vec::new),
            api_keys: preset.api_keys.clone(),
            requires_api_key: preset.requires_api_key,
        }
    }
}

// MCP Tool Request Types
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddMcpServerRequest {
    #[schemars(description = "Name of the MCP server")]
    pub name: String,
    #[schemars(description = "Command to execute the server")]
    pub command: String,
    #[schemars(description = "Arguments to pass to the command")]
    pub args: Vec<String>,
    #[schemars(description = "Environment variables for the server")]
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetPresetServersRequest {
    #[schemars(description = "Filter out already installed servers (default: false)")]
    #[serde(default)]
    pub exclude_installed: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateMcpServerRequest {
    #[schemars(description = "Name of the MCP server to update")]
    pub name: String,
    #[schemars(description = "Command to execute the server")]
    pub command: String,
    #[schemars(description = "Arguments to pass to the command")]
    pub args: Vec<String>,
    #[schemars(description = "Environment variables for the server")]
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeleteMcpServerRequest {
    #[schemars(description = "Name of the MCP server to delete")]
    pub name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetMcpServerDetailsRequest {
    #[schemars(description = "Name of the MCP server to get details for")]
    pub name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InstallPresetServerRequest {
    #[schemars(description = "Name of the preset server to install")]
    pub preset_name: String,
    #[schemars(description = "API keys required for the preset server")]
    pub api_keys: Option<HashMap<String, String>>,
}

// MCP Server with tool router
#[derive(Debug, Clone)]
pub struct McpManagerServer {
    state: AppState,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl McpManagerServer {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List all configured MCP servers in Claude Desktop")]
    async fn list_mcp_servers(&self) -> Json<Value> {
        match crate::internal_parse_claude_json(&self.state, None).await {
            Ok(servers) => {
                // Convert to sanitized version (without environment variables)
                let sanitized_servers: Vec<McpServerInfoSanitized> = servers
                    .iter()
                    .map(|server| McpServerInfoSanitized::from_server_info(server))
                    .collect();
                
                Json(json!({
                    "servers": sanitized_servers,
                    "total_count": sanitized_servers.len()
                }))
            },
            Err(e) => Json(json!({
                "error": format!("Failed to list MCP servers: {}", e)
            })),
        }
    }

    #[tool(description = "Add a new MCP server to Claude Desktop configuration")]
    async fn add_mcp_server(
        &self,
        Parameters(AddMcpServerRequest { name, command, args, env }): Parameters<AddMcpServerRequest>,
    ) -> Json<Value> {
        let server_data = McpServerEdit {
            command,
            args,
            env: env.unwrap_or_default(),
        };

        match crate::internal_add_server(&self.state, name.clone(), server_data, None).await {
            Ok(result) => {
                if result.success {
                    Json(json!({
                        "success": true,
                        "message": result.message,
                        "server_name": name
                    }))
                } else {
                    Json(json!({
                        "success": false,
                        "error": result.message
                    }))
                }
            }
            Err(e) => Json(json!({
                "success": false,
                "error": format!("Failed to add MCP server: {}", e)
            })),
        }
    }

    #[tool(description = "Update an existing MCP server configuration")]
    async fn update_mcp_server(
        &self,
        Parameters(UpdateMcpServerRequest { name, command, args, env }): Parameters<UpdateMcpServerRequest>,
    ) -> Json<Value> {
        let server_data = McpServerEdit {
            command,
            args,
            env: env.unwrap_or_default(),
        };

        match crate::update_server(name.clone(), server_data, None) {
            Ok(result) => {
                if result.success {
                    Json(json!({
                        "success": true,
                        "message": result.message,
                        "server_name": name
                    }))
                } else {
                    Json(json!({
                        "success": false,
                        "error": result.message
                    }))
                }
            }
            Err(e) => Json(json!({
                "success": false,
                "error": format!("Failed to update MCP server: {}", e)
            })),
        }
    }

    #[tool(description = "Delete an MCP server from Claude Desktop configuration")]
    async fn delete_mcp_server(
        &self,
        Parameters(DeleteMcpServerRequest { name }): Parameters<DeleteMcpServerRequest>,
    ) -> Json<Value> {
        match crate::internal_delete_server(&self.state, name.clone(), None).await {
            Ok(result) => {
                if result.success {
                    Json(json!({
                        "success": true,
                        "message": result.message,
                        "server_name": name
                    }))
                } else {
                    Json(json!({
                        "success": false,
                        "error": result.message
                    }))
                }
            }
            Err(e) => Json(json!({
                "success": false,
                "error": format!("Failed to delete MCP server: {}", e)
            })),
        }
    }

    #[tool(description = "Get detailed information about a specific MCP server")]
    async fn get_mcp_server_details(
        &self,
        Parameters(GetMcpServerDetailsRequest { name }): Parameters<GetMcpServerDetailsRequest>,
    ) -> Json<Value> {
        let config_path = {
            let path_guard = self.state.config_path.read().await;
            if path_guard.is_empty() {
                None
            } else {
                Some(path_guard.clone())
            }
        };

        match crate::get_server_details(name.clone(), config_path) {
            Ok(server_info) => {
                // Convert to sanitized version (without environment variables)
                let sanitized_server = McpServerInfoSanitized::from_server_info(&server_info);
                
                Json(json!({
                    "server": sanitized_server,
                    "name": name
                }))
            },
            Err(e) => Json(json!({
                "error": format!("Failed to get server details: {}", e)
            })),
        }
    }

    #[tool(description = "Get a list of all available preset MCP servers that can be installed")]
    async fn get_preset_servers(&self) -> Json<Value> {
        let presets = crate::get_preset_servers();
        
        // Convert to sanitized version (without environment variables)
        let sanitized_presets: Vec<PresetServerSanitized> = presets
            .iter()
            .map(|preset| PresetServerSanitized::from_preset_server(preset))
            .collect();
        
        Json(json!({
            "preset_servers": sanitized_presets,
            "total_count": sanitized_presets.len()
        }))
    }

    #[tool(description = "Get available preset MCP servers with option to exclude already installed ones")]
    async fn get_preset_servers_filtered(
        &self,
        Parameters(GetPresetServersRequest { exclude_installed }): Parameters<GetPresetServersRequest>,
    ) -> Json<Value> {
        let presets = crate::get_preset_servers();
        
        let filtered_presets = if exclude_installed {
            // Get currently installed servers to filter out
            match crate::internal_parse_claude_json(&self.state, None).await {
                Ok(installed_servers) => {
                    let installed_names: std::collections::HashSet<String> = installed_servers
                        .iter()
                        .map(|server| server.name.clone())
                        .collect();
                    
                    presets
                        .into_iter()
                        .filter(|preset| !installed_names.contains(&preset.name))
                        .collect()
                }
                Err(_) => {
                    // If we can't load installed servers, return all presets
                    presets
                }
            }
        } else {
            presets
        };
        
        // Convert to sanitized version (without environment variables)
        let sanitized_presets: Vec<PresetServerSanitized> = filtered_presets
            .iter()
            .map(|preset| PresetServerSanitized::from_preset_server(preset))
            .collect();
        
        Json(json!({
            "preset_servers": sanitized_presets,
            "total_count": sanitized_presets.len(),
            "excluded_installed": exclude_installed,
            "total_available": crate::get_preset_servers().len()
        }))
    }

    #[tool(description = "Install a preset MCP server with optional API keys")]
    async fn install_preset_server(
        &self,
        Parameters(InstallPresetServerRequest { preset_name, api_keys }): Parameters<InstallPresetServerRequest>,
    ) -> Json<Value> {
        // Get preset server details
        let preset = match crate::get_preset_server_by_name(preset_name.clone()) {
            Some(preset) => preset,
            None => {
                return Json(json!({
                    "success": false,
                    "error": format!("Preset server '{}' not found", preset_name)
                }))
            }
        };

        // Prepare server data with API keys if provided
        let mut env = preset.env.unwrap_or_default();
        if let Some(keys) = api_keys {
            env.extend(keys);
        }

        let server_data = McpServerEdit {
            command: preset.command,
            args: preset.args,
            env,
        };

        match crate::internal_add_server(&self.state, preset.name.clone(), server_data, None).await
        {
            Ok(result) => {
                if result.success {
                    Json(json!({
                        "success": true,
                        "message": result.message,
                        "server_name": preset.name,
                        "preset_name": preset_name
                    }))
                } else {
                    Json(json!({
                        "success": false,
                        "error": result.message
                    }))
                }
            }
            Err(e) => Json(json!({
                "success": false,
                "error": format!("Failed to install preset server: {}", e)
            })),
        }
    }
}

// ServerHandler implementation with tool capabilities
#[tool_handler]
impl ServerHandler for McpManagerServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("MCP Manager Server for managing Claude Desktop MCP servers. Use the available tools to list, add, update, delete, and manage MCP server configurations.".to_string()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// Start MCP server with SSE transport
pub async fn start_mcp_server(
    state: AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Get settings to determine port and path
    let settings = {
        let settings_guard = state.settings_cache.read().await;
        settings_guard.clone()
    };
    
    if !settings.mcp_server_enabled {
        println!("ℹ️ MCP server is disabled in settings");
        return Ok(());
    }
    
    let bind_address: SocketAddr = format!("127.0.0.1:{}", settings.mcp_server_port).parse()?;
    
    println!("🔗 Starting MCP Manager Server...");
    println!("📋 Available MCP Tools:");
    println!("  • list_mcp_servers - List all configured MCP servers");
    println!("  • add_mcp_server - Add a new MCP server");
    println!("  • update_mcp_server - Update an existing MCP server");
    println!("  • delete_mcp_server - Delete an MCP server");
    println!("  • get_mcp_server_details - Get details of a specific server");
    println!("  • get_preset_servers - Get available preset servers");
    println!("  • get_preset_servers_filtered - Get preset servers with filtering options");
    println!("  • install_preset_server - Install a preset server");
    
    // Get cancellation token from AppState
    let cancellation_token = {
        let token_guard = state.mcp_server_cancellation.read().await;
        token_guard.clone().unwrap_or_else(|| CancellationToken::new())
    };

    // Create SSE server configuration
    let config = SseServerConfig {
        bind: bind_address,
        sse_path: settings.mcp_sse_path.clone(),
        post_path: "/message".to_string(), // Required by SseServerConfig but not used for MCP
        ct: cancellation_token.clone(),
        sse_keep_alive: None,
    };
    
    let (sse_server, router) = SseServer::new(config);
    
    // Create TCP listener
    let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;
    
    let ct = sse_server.config.ct.child_token();
    
    // Start the axum server with graceful shutdown
    let _server_task = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router.into_make_service())
            .with_graceful_shutdown(async move {
                ct.cancelled().await;
                tracing::info!("SSE server cancelled");
            })
            .await 
        {
            tracing::error!(error = %e, "SSE server shutdown with error");
        }
    });
    
    println!("✅ MCP Manager Server initialized with tool support");
    println!("🔄 Real-time GUI synchronization active");
    println!("📡 Listening on SSE at http://{}{}...", bind_address, settings.mcp_sse_path);
    
    // Start the MCP server with the service
    let mcp_server = McpManagerServer::new(state);
    let _ct = sse_server.with_service(move || mcp_server.clone());
    
    // Wait for cancellation instead of ctrl_c since this is controlled by GUI
    cancellation_token.cancelled().await;
    
    println!("🔚 MCP server stopped");
    Ok(())
}
