use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::env;

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
        .invoke_handler(tauri::generate_handler![greet, parse_claude_json, get_server_details, update_server, add_server, delete_server, get_default_config_path])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
