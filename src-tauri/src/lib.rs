use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager as _;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

pub mod mcp_server;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServer {
    command: String,
    args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClaudeConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServer>,
}

#[derive(Debug, Serialize)]
struct McpServerInfo {
    name: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct McpServerEdit {
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
struct SaveResult {
    success: bool,
    message: String,
}

#[derive(Debug, Serialize)]
struct JsonErrorInfo {
    error_type: String,
    message: String,
    line: Option<usize>,
    column: Option<usize>,
    suggestion: Option<String>,
    has_backup: bool,
}

#[derive(Debug, Serialize)]
struct BackupInfo {
    path: String,
    created: String,
    size: u64,
    is_valid: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    #[serde(rename = "claudeConfigPath")]
    pub claude_config_path: String,
    #[serde(rename = "darkMode")]
    pub dark_mode: bool,
    #[serde(rename = "mcpServerEnabled")]
    pub mcp_server_enabled: bool,
    #[serde(rename = "mcpServerPort")]
    pub mcp_server_port: u16,
    #[serde(rename = "mcpSsePath")]
    pub mcp_sse_path: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            claude_config_path: String::new(),
            dark_mode: false,
            mcp_server_enabled: false,
            mcp_server_port: 8000,
            mcp_sse_path: "/sse".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ServerType {
    Docker,
    Npx,
    Uvx,
    Uv,
    Other(String),
}

impl ServerType {
    fn from_command(command: &str) -> Self {
        match command.to_lowercase().as_str() {
            "docker" => ServerType::Docker,
            "npx" => ServerType::Npx,
            "uvx" => ServerType::Uvx,
            "uv" => ServerType::Uv,
            _ => ServerType::Other(command.to_string()),
        }
    }

    fn to_string(&self) -> String {
        match self {
            ServerType::Docker => "docker".to_string(),
            ServerType::Npx => "npx".to_string(),
            ServerType::Uvx => "uvx".to_string(),
            ServerType::Uv => "uv".to_string(),
            ServerType::Other(s) => s.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiKeyRequirement {
    name: String,
    description: String,
    required: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PresetServer {
    name: String,
    description: String,
    category: String,
    #[serde(rename = "serverType")]
    server_type: ServerType,
    command: String,
    args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<HashMap<String, String>>,
    #[serde(rename = "apiKeys", skip_serializing_if = "Vec::is_empty")]
    api_keys: Vec<ApiKeyRequirement>,
    #[serde(rename = "requiresApiKey")]
    requires_api_key: bool,
    // Legacy fields for backward compatibility
    #[serde(rename = "apiKeyName", skip_serializing_if = "Option::is_none")]
    api_key_name: Option<String>,
    #[serde(rename = "apiKeyDescription", skip_serializing_if = "Option::is_none")]
    api_key_description: Option<String>,
}

impl PresetServer {
    fn validate_command_matches_type(&self) -> bool {
        let expected_type = ServerType::from_command(&self.command);
        self.server_type == expected_type || matches!(self.server_type, ServerType::Other(_))
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn parse_claude_json(
    state: tauri::State<'_, AppState>,
    custom_path: Option<String>,
) -> Result<Vec<McpServerInfo>, String> {
    internal_parse_claude_json(&state, custom_path).await
}

#[tauri::command]
fn get_server_details(name: String, custom_path: Option<String>) -> Result<McpServerInfo, String> {
    let config_path = resolve_config_path(custom_path)?;
    let file_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read Claude Desktop config: {}", e))?;

    let config: ClaudeConfig =
        serde_json::from_str(&file_content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let server = config
        .mcp_servers
        .get(&name)
        .ok_or_else(|| format!("Server '{}' not found", name))?;

    Ok(McpServerInfo {
        name,
        command: server.command.clone(),
        args: server.args.clone(),
        env: server.env.clone().unwrap_or_default(),
    })
}

#[tauri::command]
fn update_server(
    name: String,
    server_data: McpServerEdit,
    custom_path: Option<String>,
) -> Result<SaveResult, String> {
    save_server_config(name.clone(), Some(server_data), false, custom_path)
}

#[tauri::command]
async fn add_server(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    name: String,
    server_data: McpServerEdit,
) -> Result<SaveResult, String> {
    internal_add_server(&state, name, server_data, Some(&app_handle)).await
}

#[tauri::command]
async fn delete_server(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    name: String,
) -> Result<SaveResult, String> {
    internal_delete_server(&state, name, Some(&app_handle)).await
}

#[tauri::command]
fn get_default_config_path() -> Result<String, String> {
    get_claude_config_path()
}

#[tauri::command]
async fn load_app_settings(state: tauri::State<'_, AppState>) -> Result<AppSettings, String> {
    let settings_path = get_settings_path()?;

    let settings = if !Path::new(&settings_path).exists() {
        // Return default settings if file doesn't exist
        AppSettings::default()
    } else {
        let file_content = fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read settings file: {}", e))?;

        serde_json::from_str(&file_content)
            .map_err(|e| format!("Failed to parse settings: {}", e))?
    };

    // Update the settings cache
    {
        let mut settings_cache = state.settings_cache.write().await;
        *settings_cache = settings.clone();
    }

    Ok(settings)
}

#[tauri::command]
async fn save_app_settings(
    state: tauri::State<'_, AppState>,
    settings: AppSettings,
) -> Result<SaveResult, String> {
    let settings_path = get_settings_path()?;
    let settings_dir = Path::new(&settings_path)
        .parent()
        .ok_or("Could not determine settings directory")?;

    // Create settings directory if it doesn't exist
    if !settings_dir.exists() {
        fs::create_dir_all(settings_dir)
            .map_err(|e| format!("Failed to create settings directory: {}", e))?;
    }

    let settings_json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(&settings_path, settings_json)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

    // Update the settings cache
    {
        let mut settings_cache = state.settings_cache.write().await;
        *settings_cache = settings;
    }

    Ok(SaveResult {
        success: true,
        message: "Settings saved successfully".to_string(),
    })
}

#[tauri::command]
fn get_preset_servers() -> Vec<PresetServer> {
    get_preset_servers_database()
}

#[tauri::command]
fn get_preset_servers_by_category(category: String) -> Vec<PresetServer> {
    get_preset_servers_database()
        .into_iter()
        .filter(|server| server.category == category)
        .collect()
}

#[tauri::command]
fn get_preset_server_categories() -> Vec<String> {
    let mut categories: Vec<String> = get_preset_servers_database()
        .into_iter()
        .map(|server| server.category)
        .collect();

    // Remove duplicates and sort
    categories.sort();
    categories.dedup();
    categories
}

#[tauri::command]
fn get_preset_server_by_name(name: String) -> Option<PresetServer> {
    get_preset_servers_database()
        .into_iter()
        .find(|server| server.name == name)
}

#[tauri::command]
fn get_preset_servers_by_type(server_type: String) -> Vec<PresetServer> {
    let target_type = match server_type.to_lowercase().as_str() {
        "docker" => ServerType::Docker,
        "npx" => ServerType::Npx,
        "uvx" => ServerType::Uvx,
        "uv" => ServerType::Uv,
        _ => ServerType::Other(server_type),
    };

    get_preset_servers_database()
        .into_iter()
        .filter(|server| server.server_type == target_type)
        .collect()
}

#[tauri::command]
fn get_server_types() -> Vec<String> {
    let mut types: Vec<String> = get_preset_servers_database()
        .into_iter()
        .map(|server| server.server_type.to_string())
        .collect();

    // Remove duplicates and sort
    types.sort();
    types.dedup();
    types
}

#[tauri::command]
fn validate_server_config(server: PresetServer) -> bool {
    server.validate_command_matches_type()
}

// Internal function for starting MCP server (used by both Tauri command and auto-start)
async fn internal_start_mcp_server(state: &AppState) -> Result<SaveResult, String> {
    let settings = {
        let settings_guard = state.settings_cache.read().await;
        settings_guard.clone()
    };

    if !settings.mcp_server_enabled {
        return Ok(SaveResult {
            success: false,
            message: "MCP server is disabled in settings".to_string(),
        });
    }

    // Check if server is already running
    {
        let status_guard = state.mcp_server_status.read().await;
        if status_guard.running {
            return Ok(SaveResult {
                success: false,
                message: "MCP server is already running".to_string(),
            });
        }
    }

    // Validate port availability (basic check)
    if let Err(_) = std::net::TcpListener::bind(format!("127.0.0.1:{}", settings.mcp_server_port)) {
        return Ok(SaveResult {
            success: false,
            message: format!("Port {} is already in use", settings.mcp_server_port),
        });
    }

    // Create cancellation token
    let cancellation_token = CancellationToken::new();
    
    // Store cancellation token
    {
        let mut token_guard = state.mcp_server_cancellation.write().await;
        *token_guard = Some(cancellation_token.clone());
    }

    // Update server status
    {
        let mut status_guard = state.mcp_server_status.write().await;
        status_guard.running = true;
        status_guard.port = Some(settings.mcp_server_port);
        status_guard.sse_path = Some(settings.mcp_sse_path.clone());
        status_guard.url = Some(format!("http://127.0.0.1:{}{}", settings.mcp_server_port, settings.mcp_sse_path));
    }

    // Start MCP server in background
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(e) = mcp_server::start_mcp_server(state_clone.clone()).await {
            eprintln!("MCP server error: {}", e);
            // Reset status on error
            let mut status_guard = state_clone.mcp_server_status.write().await;
            status_guard.running = false;
            status_guard.port = None;
            status_guard.sse_path = None;
            status_guard.url = None;
        }
    });

    Ok(SaveResult {
        success: true,
        message: format!("MCP server started on port {}", settings.mcp_server_port),
    })
}

#[tauri::command]
async fn start_mcp_server(state: tauri::State<'_, AppState>) -> Result<SaveResult, String> {
    let settings = {
        let settings_guard = state.settings_cache.read().await;
        settings_guard.clone()
    };

    println!("Debug: MCP server enabled: {}", settings.mcp_server_enabled);
    println!("Debug: MCP server port: {}", settings.mcp_server_port);
    println!("Debug: MCP SSE path: {}", settings.mcp_sse_path);

    internal_start_mcp_server(state.inner()).await
}

#[tauri::command]
async fn stop_mcp_server(state: tauri::State<'_, AppState>) -> Result<SaveResult, String> {
    // Get and cancel the token
    let token = {
        let mut token_guard = state.mcp_server_cancellation.write().await;
        token_guard.take()
    };

    if let Some(token) = token {
        token.cancel();
    }

    // Update server status
    {
        let mut status_guard = state.mcp_server_status.write().await;
        status_guard.running = false;
        status_guard.port = None;
        status_guard.sse_path = None;
        status_guard.url = None;
    }

    Ok(SaveResult {
        success: true,
        message: "MCP server stopped".to_string(),
    })
}

#[tauri::command]
async fn get_mcp_server_status(state: tauri::State<'_, AppState>) -> Result<McpServerStatus, String> {
    let status_guard = state.mcp_server_status.read().await;
    Ok(status_guard.clone())
}

#[tauri::command]
fn validate_mcp_port(port: u16) -> Result<SaveResult, String> {
    if port < 1024 {
        return Ok(SaveResult {
            success: false,
            message: "Port must be between 1024 and 65535".to_string(),
        });
    }

    // Try to bind to the port to check availability
    match std::net::TcpListener::bind(format!("127.0.0.1:{}", port)) {
        Ok(_) => Ok(SaveResult {
            success: true,
            message: format!("Port {} is available", port),
        }),
        Err(_) => Ok(SaveResult {
            success: false,
            message: format!("Port {} is already in use", port),
        }),
    }
}

#[tauri::command]
fn get_backup_info(custom_path: Option<String>) -> Result<Option<BackupInfo>, String> {
    let config_path = resolve_config_path(custom_path)?;
    let backup_path = format!("{}.backup", config_path);

    if !Path::new(&backup_path).exists() {
        return Ok(None);
    }

    let metadata =
        fs::metadata(&backup_path).map_err(|e| format!("Failed to get backup metadata: {}", e))?;

    let size = metadata.len();
    let created = metadata
        .modified()
        .map(|time| {
            let duration = time
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let datetime = std::time::SystemTime::UNIX_EPOCH + duration;
            format!("{:?}", datetime) // Simple formatting for now
        })
        .unwrap_or_else(|_| "Unknown".to_string());

    // Validate backup by trying to parse it
    let is_valid = match fs::read_to_string(&backup_path) {
        Ok(content) => serde_json::from_str::<ClaudeConfig>(&content).is_ok(),
        Err(_) => false,
    };

    Ok(Some(BackupInfo {
        path: backup_path,
        created,
        size,
        is_valid,
    }))
}

#[tauri::command]
fn restore_from_backup(custom_path: Option<String>) -> Result<SaveResult, String> {
    let config_path = resolve_config_path(custom_path)?;
    let backup_path = format!("{}.backup", config_path);

    if !Path::new(&backup_path).exists() {
        return Ok(SaveResult {
            success: false,
            message: "No backup file found".to_string(),
        });
    }

    // Validate backup before restoring
    let backup_content = fs::read_to_string(&backup_path)
        .map_err(|e| format!("Failed to read backup file: {}", e))?;

    let _config: ClaudeConfig =
        serde_json::from_str(&backup_content).map_err(|_| "Backup file is corrupted or invalid")?;

    // Create a backup of the current (potentially broken) file
    let broken_backup_path = format!("{}.broken", config_path);
    if Path::new(&config_path).exists() {
        fs::copy(&config_path, &broken_backup_path)
            .map_err(|e| format!("Failed to backup current file: {}", e))?;
    }

    // Restore from backup
    fs::copy(&backup_path, &config_path)
        .map_err(|e| format!("Failed to restore from backup: {}", e))?;

    Ok(SaveResult {
        success: true,
        message: "Configuration restored from backup successfully".to_string(),
    })
}

#[tauri::command]
fn open_file_location(path: String) -> Result<(), String> {
    use std::process::Command;

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| format!("Failed to open file location: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| format!("Failed to open file location: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try different file managers for Linux
        let file_managers = ["nautilus", "dolphin", "thunar", "pcmanfm", "nemo"];
        let mut success = false;

        for manager in &file_managers {
            if let Ok(_) = Command::new(manager).arg(&path).spawn() {
                success = true;
                break;
            }
        }

        if !success {
            // Fallback to opening the parent directory with xdg-open
            let parent_path = std::path::Path::new(&path)
                .parent()
                .unwrap_or_else(|| std::path::Path::new("/"))
                .to_string_lossy();

            Command::new("xdg-open")
                .arg(parent_path.as_ref())
                .spawn()
                .map_err(|e| format!("Failed to open file location: {}", e))?;
        }
    }

    Ok(())
}

#[tauri::command]
fn create_manual_backup(custom_path: Option<String>) -> Result<SaveResult, String> {
    let config_path = resolve_config_path(custom_path)?;

    if !Path::new(&config_path).exists() {
        return Ok(SaveResult {
            success: false,
            message: "Configuration file does not exist".to_string(),
        });
    }

    // Create timestamped backup
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let manual_backup_path = format!("{}.manual_backup_{}", config_path, timestamp);

    fs::copy(&config_path, &manual_backup_path)
        .map_err(|e| format!("Failed to create manual backup: {}", e))?;

    Ok(SaveResult {
        success: true,
        message: format!("Manual backup created: {}", manual_backup_path),
    })
}

#[tauri::command]
fn get_settings_path() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let appdata =
            env::var("APPDATA").map_err(|_| "Could not determine APPDATA directory".to_string())?;
        Ok(format!("{}\\mcp-manager\\settings.json", appdata))
    }

    #[cfg(target_os = "macos")]
    {
        let home_dir =
            env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;
        Ok(format!(
            "{}/Library/Application Support/mcp-manager/settings.json",
            home_dir
        ))
    }

    #[cfg(target_os = "linux")]
    {
        let home_dir =
            env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;
        Ok(format!("{}/.config/mcp-manager/settings.json", home_dir))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Unsupported operating system".to_string())
    }
}

// MCP Server Status
#[derive(Debug, Serialize, Clone)]
pub struct McpServerStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub sse_path: Option<String>,
    pub url: Option<String>,
}

// Shared state for real-time sync between GUI and MCP server
#[derive(Debug, Clone)]
pub struct AppState {
    pub config_cache: Arc<RwLock<Option<ClaudeConfig>>>,
    pub settings_cache: Arc<RwLock<AppSettings>>,
    pub config_path: Arc<RwLock<String>>,
    pub mcp_server_status: Arc<RwLock<McpServerStatus>>,
    pub mcp_server_cancellation: Arc<RwLock<Option<CancellationToken>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config_cache: Arc::new(RwLock::new(None)),
            settings_cache: Arc::new(RwLock::new(AppSettings::default())),
            config_path: Arc::new(RwLock::new(String::new())),
            mcp_server_status: Arc::new(RwLock::new(McpServerStatus {
                running: false,
                port: None,
                sse_path: None,
                url: None,
            })),
            mcp_server_cancellation: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn load_config(&self, custom_path: Option<String>) -> Result<ClaudeConfig, String> {
        let config_path = resolve_config_path(custom_path)?;
        *self.config_path.write().await = config_path.clone();

        let file_content = fs::read_to_string(&config_path).map_err(|e| {
            format!(
                "Failed to read Claude Desktop config at {}: {}",
                config_path, e
            )
        })?;

        let config: ClaudeConfig = match serde_json::from_str(&file_content) {
            Ok(config) => config,
            Err(e) => {
                let mut error_info = analyze_json_error(&file_content, &e);
                let backup_path = format!("{}.backup", config_path);
                error_info.has_backup = Path::new(&backup_path).exists();
                let error_json = serde_json::to_string(&error_info).unwrap_or_else(|_| {
                    format!(
                        "{{\"error_type\":\"unknown\",\"message\":\"Failed to parse JSON: {}\"}}",
                        e
                    )
                });
                return Err(format!("JSON_ERROR:{}", error_json));
            }
        };

        if let Err(validation_error) = validate_claude_config_structure(&config) {
            return Err(format!(
                "Configuration validation failed: {}",
                validation_error
            ));
        }

        *self.config_cache.write().await = Some(config.clone());
        Ok(config)
    }

    pub async fn save_config(&self, config: &ClaudeConfig) -> Result<(), String> {
        let config_path = self.config_path.read().await.clone();
        if config_path.is_empty() {
            return Err("Config path not set".to_string());
        }

        // Create backup
        let backup_path = format!("{}.backup", config_path);
        fs::copy(&config_path, &backup_path)
            .map_err(|e| format!("Failed to create backup: {}", e))?;

        // Write updated config
        let updated_content = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(&config_path, updated_content)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        // Update cache
        *self.config_cache.write().await = Some(config.clone());
        Ok(())
    }

    pub async fn emit_event(
        &self,
        app_handle: &tauri::AppHandle,
        event: &str,
        payload: serde_json::Value,
    ) {
        if let Err(e) = app_handle.emit(event, payload) {
            eprintln!("Failed to emit event {}: {}", event, e);
        }
    }
}

// Internal shared functions that both Tauri commands and MCP tools can use
async fn internal_parse_claude_json(
    state: &AppState,
    custom_path: Option<String>,
) -> Result<Vec<McpServerInfo>, String> {
    let config = state.load_config(custom_path).await?;

    let mut servers = Vec::new();
    for (name, server) in config.mcp_servers {
        let env = server.env.unwrap_or_default();
        servers.push(McpServerInfo {
            name,
            command: server.command,
            args: server.args,
            env,
        });
    }

    // Sort servers alphabetically by name
    servers.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(servers)
}

async fn internal_add_server(
    state: &AppState,
    name: String,
    server_data: McpServerEdit,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<SaveResult, String> {
    let mut config = state.load_config(None).await?;

    if config.mcp_servers.contains_key(&name) {
        return Ok(SaveResult {
            success: false,
            message: format!("Server '{}' already exists", name),
        });
    }

    let env = if server_data.env.is_empty() {
        None
    } else {
        Some(server_data.env)
    };

    config.mcp_servers.insert(
        name.clone(),
        McpServer {
            command: server_data.command,
            args: server_data.args,
            env,
        },
    );

    state.save_config(&config).await?;

    // Emit event for GUI updates
    if let Some(handle) = app_handle {
        state
            .emit_event(handle, "server-added", serde_json::json!({ "name": name }))
            .await;
        state
            .emit_event(handle, "config-changed", serde_json::json!({}))
            .await;
    }

    Ok(SaveResult {
        success: true,
        message: format!("Server '{}' added successfully", name),
    })
}

async fn internal_delete_server(
    state: &AppState,
    name: String,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<SaveResult, String> {
    let mut config = state.load_config(None).await?;

    if config.mcp_servers.remove(&name).is_none() {
        return Ok(SaveResult {
            success: false,
            message: format!("Server '{}' not found", name),
        });
    }

    state.save_config(&config).await?;

    // Emit event for GUI updates
    if let Some(handle) = app_handle {
        state
            .emit_event(
                handle,
                "server-deleted",
                serde_json::json!({ "name": name }),
            )
            .await;
        state
            .emit_event(handle, "config-changed", serde_json::json!({}))
            .await;
    }

    Ok(SaveResult {
        success: true,
        message: format!("Server '{}' deleted successfully", name),
    })
}

fn get_preset_servers_database() -> Vec<PresetServer> {
    vec![
        PresetServer {
            name: "dice".to_string(),
            description: "Random dice rolling utility for games and decision making".to_string(),
            category: "Utilities".to_string(),
            server_type: ServerType::Uvx,
            command: "uvx".to_string(),
            args: vec!["mcp-dice".to_string()],
            env: None,
            api_keys: vec![],
            requires_api_key: false,
            api_key_name: None,
            api_key_description: None,
        },
        PresetServer {
            name: "time".to_string(),
            description: "Time and timezone utilities for scheduling and time management"
                .to_string(),
            category: "Utilities".to_string(),
            server_type: ServerType::Uvx,
            command: "uvx".to_string(),
            args: vec![
                "mcp-server-time".to_string(),
                "--local-timezone=UTC".to_string(),
            ],
            env: None,
            api_keys: vec![],
            requires_api_key: false,
            api_key_name: None,
            api_key_description: None,
        },
        PresetServer {
            name: "sequential-thinking".to_string(),
            description: "Enhanced reasoning capabilities for complex problem solving".to_string(),
            category: "AI Tools".to_string(),
            server_type: ServerType::Docker,
            command: "docker".to_string(),
            args: vec![
                "run".to_string(),
                "--rm".to_string(),
                "-i".to_string(),
                "mcp/sequentialthinking".to_string(),
            ],
            env: None,
            api_keys: vec![],
            requires_api_key: false,
            api_key_name: None,
            api_key_description: None,
        },
        PresetServer {
            name: "browsermcp".to_string(),
            description: "Web browsing capabilities for accessing and interacting with websites"
                .to_string(),
            category: "Web Tools".to_string(),
            server_type: ServerType::Npx,
            command: "npx".to_string(),
            args: vec!["@browsermcp/mcp@latest".to_string()],
            env: None,
            api_keys: vec![],
            requires_api_key: false,
            api_key_name: None,
            api_key_description: None,
        },
        PresetServer {
            name: "brave-search".to_string(),
            description: "Web search functionality using Brave Search API".to_string(),
            category: "Search".to_string(),
            server_type: ServerType::Npx,
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-brave-search".to_string(),
            ],
            env: None,
            api_keys: vec![ApiKeyRequirement {
                name: "BRAVE_API_KEY".to_string(),
                description: "Get your API key from https://brave.com/search/api/".to_string(),
                required: true,
            }],
            requires_api_key: true,
            api_key_name: Some("BRAVE_API_KEY".to_string()),
            api_key_description: Some(
                "Get your API key from https://brave.com/search/api/".to_string(),
            ),
        },
        PresetServer {
            name: "openweather".to_string(),
            description: "Weather information and forecasts using OpenWeatherMap API".to_string(),
            category: "Weather".to_string(),
            server_type: ServerType::Docker,
            command: "docker".to_string(),
            args: vec![
                "run".to_string(),
                "-i".to_string(),
                "--rm".to_string(),
                "-e".to_string(),
                "OWM_API_KEY".to_string(),
                "mcp/openweather".to_string(),
            ],
            env: None,
            api_keys: vec![ApiKeyRequirement {
                name: "OWM_API_KEY".to_string(),
                description: "Get your API key from https://openweathermap.org/api".to_string(),
                required: true,
            }],
            requires_api_key: true,
            api_key_name: Some("OWM_API_KEY".to_string()),
            api_key_description: Some(
                "Get your API key from https://openweathermap.org/api".to_string(),
            ),
        },
        PresetServer {
            name: "context7".to_string(),
            description: "Documentation search and code context analysis".to_string(),
            category: "Development".to_string(),
            server_type: ServerType::Npx,
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@upstash/context7-mcp@latest".to_string()],
            env: None,
            api_keys: vec![],
            requires_api_key: false,
            api_key_name: None,
            api_key_description: None,
        },
        PresetServer {
            name: "docker".to_string(),
            description: "Docker container management and operations".to_string(),
            category: "Development".to_string(),
            server_type: ServerType::Uvx,
            command: "uvx".to_string(),
            args: vec![
                "--from".to_string(),
                "git+https://github.com/ckreiling/mcp-server-docker".to_string(),
                "mcp-server-docker".to_string(),
            ],
            env: None,
            api_keys: vec![],
            requires_api_key: false,
            api_key_name: None,
            api_key_description: None,
        },
        PresetServer {
            name: "desktop-commander".to_string(),
            description: "Desktop automation and system control capabilities".to_string(),
            category: "System".to_string(),
            server_type: ServerType::Docker,
            command: "docker".to_string(),
            args: vec![
                "run".to_string(),
                "-i".to_string(),
                "--rm".to_string(),
                "mcp/desktop-commander".to_string(),
            ],
            env: None,
            api_keys: vec![],
            requires_api_key: false,
            api_key_name: None,
            api_key_description: None,
        },
        PresetServer {
            name: "mcp-manager".to_string(),
            description: "control mcp manager using ai".to_string(),
            category: "Development".to_string(),
            server_type: ServerType::Npx,
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "supergateway".to_string(),
                "--sse".to_string(),
                "http://localhost:8000/sse".to_string(),
            ],
            env: None,
            api_keys: vec![],
            requires_api_key: false,
            api_key_name: None,
            api_key_description: None,
        },
    ]
}

fn analyze_json_error(_json_content: &str, error: &serde_json::Error) -> JsonErrorInfo {
    let error_msg = error.to_string();
    let line = error.line();
    let column = error.column();

    let (error_type, user_message, suggestion) = if error_msg.contains("EOF while parsing") {
        (
            "incomplete",
            "The JSON file appears to be incomplete or truncated",
            Some("Check if the file ends properly with closing braces }".to_string()),
        )
    } else if error_msg.contains("expected") && error_msg.contains("found") {
        (
            "syntax",
            "Invalid JSON syntax found",
            Some(
                "Check for missing commas, quotes, or brackets around the error location"
                    .to_string(),
            ),
        )
    } else if error_msg.contains("trailing comma") {
        (
            "trailing_comma",
            "Found an extra comma at the end of a list or object",
            Some("Remove the trailing comma before the closing bracket".to_string()),
        )
    } else if error_msg.contains("duplicate key") {
        (
            "duplicate_key",
            "Found duplicate server names in the configuration",
            Some("Each server must have a unique name".to_string()),
        )
    } else {
        (
            "unknown",
            "JSON parsing error occurred",
            Some("Please check your JSON syntax or restore from backup".to_string()),
        )
    };

    JsonErrorInfo {
        error_type: error_type.to_string(),
        message: user_message.to_string(),
        line: Some(line),
        column: Some(column),
        suggestion,
        has_backup: false, // Will be updated by caller
    }
}

fn validate_claude_config_structure(config: &ClaudeConfig) -> Result<(), String> {
    // Check if mcpServers exists and is valid
    if config.mcp_servers.is_empty() {
        return Ok(()); // Empty config is valid
    }

    // Validate each server configuration
    for (name, server) in &config.mcp_servers {
        if name.trim().is_empty() {
            return Err("Server name cannot be empty".to_string());
        }

        if server.command.trim().is_empty() {
            return Err(format!("Server '{}' has an empty command", name));
        }

        // Check for common command issues
        if server.command.contains(" ") && !server.command.starts_with("\"") {
            return Err(format!("Server '{}' command contains spaces but is not quoted. Consider moving arguments to the 'args' array", name));
        }
    }

    Ok(())
}

fn save_server_config(
    name: String,
    server_data: Option<McpServerEdit>,
    is_new: bool,
    custom_path: Option<String>,
) -> Result<SaveResult, String> {
    let config_path = resolve_config_path(custom_path)?;

    // Create backup
    let backup_path = format!("{}.backup", config_path);
    fs::copy(&config_path, &backup_path).map_err(|e| format!("Failed to create backup: {}", e))?;

    // Read current config
    let file_content =
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    let mut config: ClaudeConfig =
        serde_json::from_str(&file_content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let is_add_or_update = server_data.is_some();

    match server_data {
        Some(data) => {
            // Add or update server
            if is_new && config.mcp_servers.contains_key(&name) {
                return Ok(SaveResult {
                    success: false,
                    message: format!("Server '{}' already exists", name),
                });
            }

            let env = if data.env.is_empty() {
                None
            } else {
                Some(data.env)
            };

            config.mcp_servers.insert(
                name.clone(),
                McpServer {
                    command: data.command,
                    args: data.args,
                    env,
                },
            );
        }
        None => {
            // Delete server
            if config.mcp_servers.remove(&name).is_none() {
                return Ok(SaveResult {
                    success: false,
                    message: format!("Server '{}' not found", name),
                });
            }
        }
    }

    // Write updated config
    let updated_content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, updated_content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    let action = if is_add_or_update {
        if is_new {
            "added"
        } else {
            "updated"
        }
    } else {
        "deleted"
    };

    Ok(SaveResult {
        success: true,
        message: format!("Server '{}' {} successfully", name, action),
    })
}

fn get_claude_config_path() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let appdata =
            env::var("APPDATA").map_err(|_| "Could not determine APPDATA directory".to_string())?;
        Ok(format!("{}\\Claude\\claude_desktop_config.json", appdata))
    }

    #[cfg(target_os = "macos")]
    {
        let home_dir =
            env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;
        Ok(format!(
            "{}/Library/Application Support/Claude/claude_desktop_config.json",
            home_dir
        ))
    }

    #[cfg(target_os = "linux")]
    {
        let home_dir =
            env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;
        Ok(format!(
            "{}/.config/Claude/claude_desktop_config.json",
            home_dir
        ))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Unsupported operating system".to_string())
    }
}

fn resolve_config_path(custom_path: Option<String>) -> Result<String, String> {
    if let Some(path) = custom_path {
        if path.trim().is_empty() {
            get_claude_config_path()
        } else {
            Ok(path)
        }
    } else {
        get_claude_config_path()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create the shared state for the application
    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            parse_claude_json,
            get_server_details,
            update_server,
            add_server,
            delete_server,
            get_default_config_path,
            load_app_settings,
            save_app_settings,
            get_settings_path,
            get_preset_servers,
            get_preset_servers_by_category,
            get_preset_server_categories,
            get_preset_server_by_name,
            get_preset_servers_by_type,
            get_server_types,
            validate_server_config,
            get_backup_info,
            restore_from_backup,
            create_manual_backup,
            open_file_location,
            start_mcp_server,
            stop_mcp_server,
            get_mcp_server_status,
            validate_mcp_port
        ])
        .setup(|_app| {
            println!("🚀 MCP Manager started with integrated MCP server support");
            println!("📱 Desktop GUI: Available");
            println!("🔗 MCP Server: Ready for Claude Desktop integration");
            println!("⚙️ Auto-start: Will check settings and start if enabled");
            // Load settings into cache on startup
            let app_state = _app.state::<AppState>();
            let state_clone = app_state.inner().clone();
            tauri::async_runtime::spawn(async move {
                // Load settings on startup to populate cache
                let settings_path = match get_settings_path() {
                    Ok(path) => path,
                    Err(e) => {
                        eprintln!("⚠️ Failed to get settings path: {}", e);
                        return;
                    }
                };

                let settings = if !Path::new(&settings_path).exists() {
                    AppSettings::default()
                } else {
                    match fs::read_to_string(&settings_path) {
                        Ok(content) => {
                            match serde_json::from_str(&content) {
                                Ok(settings) => settings,
                                Err(e) => {
                                    eprintln!("⚠️ Failed to parse settings: {}", e);
                                    AppSettings::default()
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("⚠️ Failed to read settings file: {}", e);
                            AppSettings::default()
                        }
                    }
                };

                // Update the settings cache
                {
                    let mut settings_cache = state_clone.settings_cache.write().await;
                    *settings_cache = settings.clone();
                }

                println!("✅ Settings loaded on startup: MCP server enabled = {}", settings.mcp_server_enabled);

                // Auto-start MCP server if enabled in settings
                if settings.mcp_server_enabled {
                    println!("🚀 Auto-starting MCP server...");
                    match internal_start_mcp_server(&state_clone).await {
                        Ok(result) => {
                            if result.success {
                                println!("✅ MCP server auto-started successfully: {}", result.message);
                            } else {
                                println!("⚠️ MCP server auto-start failed: {}", result.message);
                            }
                        }
                        Err(e) => {
                            println!("❌ MCP server auto-start error: {}", e);
                        }
                    }
                } else {
                    println!("ℹ️ MCP server auto-start skipped (disabled in settings)");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
