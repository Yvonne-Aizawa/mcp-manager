# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

### tool usage
before writing code try to look at your tools.
with these you can get documentation for the rust library you want to use.
for rmcp its rust-sdk
                                                                                  
## Project Overview

This is a Tauri desktop application called "mcp-manager" for managing Model Context Protocol (MCP) servers in Claude Desktop. The project follows Tauri's recommended architecture with:

- Frontend: TypeScript + Vite (port 1420)
- Backend: Rust with Tauri framework
- Communication: Frontend invokes Rust commands via Tauri's IPC system

## Development Commands

### Development
- `npm run dev` - Start Vite development server (frontend only)
- `npm run tauri dev` - Start Tauri development mode (frontend + backend hot reload)
- `npm run tauri:dev` - Alternative command for Tauri development

### Building
- `npm run build` - Build TypeScript and create production frontend
- `npm run tauri build` - Build complete desktop application for distribution
- `npm run tauri:build` - Alternative command for production build
- `npm run tauri:build:debug` - Build debug version with debug symbols

### Testing & Quality
- `npm run typecheck` - Run TypeScript type checking without emitting files
- `npm run lint` - Currently shows "No linter configured" message
- `npm run test` - Currently shows "No tests configured" message
- `tsc` - Type check TypeScript code directly

### Utilities
- `npm run preview` - Preview the built frontend
- `npm run clean` - Remove dist and src-tauri/target directories

## Architecture

### Frontend Structure (`src/`)
- `main.ts` - Main TypeScript entry point with comprehensive MCP server management logic
- `index.html` - HTML template with modal containers and basic UI structure
- `styles.css` - Application styles with dark mode support using CSS custom properties
- `vite.config.ts` - Vite configuration optimized for Tauri development

### Backend Structure (`src-tauri/`)
- `src/lib.rs` - Main Rust library with all Tauri commands for MCP server management
- `src/main.rs` - Rust application entry point
- `Cargo.toml` - Rust dependencies (includes serde for JSON handling)
- `tauri.conf.json` - Tauri app configuration (window: 1000x700, bundle settings)

### Key Patterns & Data Flow

#### Tauri Commands (Backend)
All Tauri commands are defined in `src-tauri/src/lib.rs` and must be registered in the `invoke_handler`:
- `parse_claude_json()` - Load and parse Claude Desktop config file
- `add_server()`, `update_server()`, `delete_server()` - CRUD operations for MCP servers
- `get_server_details()` - Get individual server configuration
- `get_default_config_path()` - Get OS-specific Claude config path
- `load_app_settings()`, `save_app_settings()` - App settings persistence
- `get_preset_servers()` - Get built-in preset server database
- Preset server filtering commands by category/type

#### Frontend Communication
- Frontend calls backend via `invoke()` from `@tauri-apps/api/core`
- All data structures are TypeScript interfaces matching Rust structs
- Modal-based UI system replacing browser alerts
- Settings stored in OS-specific locations (not localStorage)

#### Data Structures
Key interfaces used throughout the application:
- `McpServerInfo` - Complete server info with name, command, args, env
- `McpServerEdit` - Server data for create/update operations
- `PresetServer` - Preset server definitions with API key requirements
- `AppSettings` - Application settings (Claude config path, dark mode)
- `SaveResult` - Standard response format for save operations

## Configuration & Settings

### Claude Config File Locations
The app automatically detects OS-specific Claude Desktop config paths:
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`

### App Settings Storage
Settings are stored in OS-specific locations (not localStorage):
- Windows: `%APPDATA%\mcp-manager\settings.json`
- macOS: `~/Library/Application Support/mcp-manager/settings.json`
- Linux: `~/.config/mcp-manager/settings.json`

### Preset Servers Database
The built-in preset server database includes categorized servers:
- **Utilities**: dice, time
- **AI Tools**: sequential-thinking
- **Web Tools**: browsermcp
- **Search**: brave-search (requires API key)
- **Weather**: openweather (requires API key)
- **Development**: context7, docker
- **System**: desktop-commander

## Important Implementation Details

### Security & Environment Variables
- Environment variables are password-masked by default with toggle visibility
- API keys are handled securely during preset server installation
- Automatic backup creation before any configuration changes

### Settings Migration
- The app includes localStorage-to-file migration for settings
- Migration happens automatically on first load if localStorage settings exist

### Error Handling
- All Rust commands return `Result<T, String>` for proper error handling
- Frontend uses `SaveResult` struct for consistent success/error reporting
- Automatic backup and rollback capabilities

### Cross-Platform Considerations
- The Rust backend library is named `mcp_manager_lib` to avoid naming conflicts on Windows
- OS-specific path handling for config files and settings
- Window configuration: 1000x700 pixels, minimum 800x600, centered

### Development Environment
- Development server runs on fixed port 1420 (configured in vite.config.ts and tauri.conf.json)
- Vite is configured to ignore watching `src-tauri` directory to prevent conflicts
- TypeScript is configured with strict mode and unused parameter checking