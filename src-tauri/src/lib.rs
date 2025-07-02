use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::env;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct McpServer {
    command: String,
    args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeConfig {
    #[serde(rename = "mcpServers")]
    mcp_servers: HashMap<String, McpServer>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppSettings {
    #[serde(rename = "claudeConfigPath")]
    claude_config_path: String,
    #[serde(rename = "darkMode")]
    dark_mode: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            claude_config_path: String::new(),
            dark_mode: false,
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
struct ApiKeyRequirement {
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
fn parse_claude_json(custom_path: Option<String>) -> Result<Vec<McpServerInfo>, String> {
    let config_path = resolve_config_path(custom_path)?;
    
    let file_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read Claude Desktop config at {}: {}", config_path, e))?;
    
    let config: ClaudeConfig = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
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

#[tauri::command]
fn get_server_details(name: String, custom_path: Option<String>) -> Result<McpServerInfo, String> {
    let config_path = resolve_config_path(custom_path)?;
    let file_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read Claude Desktop config: {}", e))?;
    
    let config: ClaudeConfig = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    let server = config.mcp_servers.get(&name)
        .ok_or_else(|| format!("Server '{}' not found", name))?;
    
    Ok(McpServerInfo {
        name,
        command: server.command.clone(),
        args: server.args.clone(),
        env: server.env.clone().unwrap_or_default(),
    })
}

#[tauri::command]
fn update_server(name: String, server_data: McpServerEdit, custom_path: Option<String>) -> Result<SaveResult, String> {
    save_server_config(name.clone(), Some(server_data), false, custom_path)
}

#[tauri::command]
fn add_server(name: String, server_data: McpServerEdit, custom_path: Option<String>) -> Result<SaveResult, String> {
    save_server_config(name, Some(server_data), true, custom_path)
}

#[tauri::command]
fn delete_server(name: String, custom_path: Option<String>) -> Result<SaveResult, String> {
    save_server_config(name, None, false, custom_path)
}

#[tauri::command]
fn get_default_config_path() -> Result<String, String> {
    get_claude_config_path()
}

#[tauri::command]
fn load_app_settings() -> Result<AppSettings, String> {
    let settings_path = get_settings_path()?;
    
    if !Path::new(&settings_path).exists() {
        // Return default settings if file doesn't exist
        return Ok(AppSettings::default());
    }
    
    let file_content = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;
    
    let settings: AppSettings = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to parse settings: {}", e))?;
    
    Ok(settings)
}

#[tauri::command]
fn save_app_settings(settings: AppSettings) -> Result<SaveResult, String> {
    let settings_path = get_settings_path()?;
    let settings_dir = Path::new(&settings_path).parent()
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

#[tauri::command]
fn get_settings_path() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let appdata = env::var("APPDATA")
            .map_err(|_| "Could not determine APPDATA directory".to_string())?;
        Ok(format!("{}\\mcp-manager\\settings.json", appdata))
    }
    
    #[cfg(target_os = "macos")]
    {
        let home_dir = env::var("HOME")
            .map_err(|_| "Could not determine home directory".to_string())?;
        Ok(format!("{}/Library/Application Support/mcp-manager/settings.json", home_dir))
    }
    
    #[cfg(target_os = "linux")]
    {
        let home_dir = env::var("HOME")
            .map_err(|_| "Could not determine home directory".to_string())?;
        Ok(format!("{}/.config/mcp-manager/settings.json", home_dir))
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Unsupported operating system".to_string())
    }
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
            description: "Time and timezone utilities for scheduling and time management".to_string(),
            category: "Utilities".to_string(),
            server_type: ServerType::Uvx,
            command: "uvx".to_string(),
            args: vec!["mcp-server-time".to_string(), "--local-timezone=UTC".to_string()],
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
            args: vec!["run".to_string(), "--rm".to_string(), "-i".to_string(), "mcp/sequentialthinking".to_string()],
            env: None,
            api_keys: vec![],
            requires_api_key: false,
            api_key_name: None,
            api_key_description: None,
        },
        PresetServer {
            name: "browsermcp".to_string(),
            description: "Web browsing capabilities for accessing and interacting with websites".to_string(),
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
            args: vec!["-y".to_string(), "@modelcontextprotocol/server-brave-search".to_string()],
            env: None,
            api_keys: vec![
                ApiKeyRequirement {
                    name: "BRAVE_API_KEY".to_string(),
                    description: "Get your API key from https://brave.com/search/api/".to_string(),
                    required: true,
                }
            ],
            requires_api_key: true,
            api_key_name: Some("BRAVE_API_KEY".to_string()),
            api_key_description: Some("Get your API key from https://brave.com/search/api/".to_string()),
        },
        PresetServer {
            name: "openweather".to_string(),
            description: "Weather information and forecasts using OpenWeatherMap API".to_string(),
            category: "Weather".to_string(),
            server_type: ServerType::Docker,
            command: "docker".to_string(),
            args: vec!["run".to_string(), "-i".to_string(), "--rm".to_string(), "-e".to_string(), "OWM_API_KEY".to_string(), "mcp/openweather".to_string()],
            env: None,
            api_keys: vec![
                ApiKeyRequirement {
                    name: "OWM_API_KEY".to_string(),
                    description: "Get your API key from https://openweathermap.org/api".to_string(),
                    required: true,
                }
            ],
            requires_api_key: true,
            api_key_name: Some("OWM_API_KEY".to_string()),
            api_key_description: Some("Get your API key from https://openweathermap.org/api".to_string()),
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
            args: vec!["--from".to_string(), "git+https://github.com/ckreiling/mcp-server-docker".to_string(), "mcp-server-docker".to_string()],
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
            args: vec!["run".to_string(), "-i".to_string(), "--rm".to_string(), "mcp/desktop-commander".to_string()],
            env: None,
            api_keys: vec![],
            requires_api_key: false,
            api_key_name: None,
            api_key_description: None,
        }
    ]
}

fn save_server_config(name: String, server_data: Option<McpServerEdit>, is_new: bool, custom_path: Option<String>) -> Result<SaveResult, String> {
    let config_path = resolve_config_path(custom_path)?;
    
    // Create backup
    let backup_path = format!("{}.backup", config_path);
    fs::copy(&config_path, &backup_path)
        .map_err(|e| format!("Failed to create backup: {}", e))?;
    
    // Read current config
    let file_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let mut config: ClaudeConfig = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
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
            
            let env = if data.env.is_empty() { None } else { Some(data.env) };
            
            config.mcp_servers.insert(name.clone(), McpServer {
                command: data.command,
                args: data.args,
                env,
            });
        },
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
        if is_new { "added" } else { "updated" }
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
        let appdata = env::var("APPDATA")
            .map_err(|_| "Could not determine APPDATA directory".to_string())?;
        Ok(format!("{}\\Claude\\claude_desktop_config.json", appdata))
    }
    
    #[cfg(target_os = "macos")]
    {
        let home_dir = env::var("HOME")
            .map_err(|_| "Could not determine home directory".to_string())?;
        Ok(format!("{}/Library/Application Support/Claude/claude_desktop_config.json", home_dir))
    }
    
    #[cfg(target_os = "linux")]
    {
        let home_dir = env::var("HOME")
            .map_err(|_| "Could not determine home directory".to_string())?;
        Ok(format!("{}/.config/Claude/claude_desktop_config.json", home_dir))
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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, parse_claude_json, get_server_details, update_server, add_server, delete_server, get_default_config_path, load_app_settings, save_app_settings, get_settings_path, get_preset_servers, get_preset_servers_by_category, get_preset_server_categories, get_preset_server_by_name, get_preset_servers_by_type, get_server_types, validate_server_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
