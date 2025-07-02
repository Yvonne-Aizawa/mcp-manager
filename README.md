## WARNING
this whole project was vibecoded in an hour. i am not responsible for blowing up your pc. 

Serously though make backups of your claude configruation

# MCP Manager

A desktop application built with Tauri for managing Model Context Protocol (MCP) servers in Claude Desktop. This tool provides a user-friendly interface to add, edit, delete, and configure MCP servers without manually editing JSON configuration files.

## Features

### 🔧 Core Functionality
- **Parse & Display**: Automatically loads and displays all MCP servers from your Claude Desktop configuration
- **CRUD Operations**: Add, edit, update, and delete MCP servers with a clean interface
- **Cross-Platform**: Works on Windows, macOS, and Linux with OS-specific config file detection
- **Backup Protection**: Automatically creates backups before making changes

### 🚀 Quick Install
- **Preset Servers**: One-click installation of popular MCP servers including:
  - Brave Search, OpenWeather, Time utilities
  - Development tools (Docker, Context7)
  - AI tools (Sequential Thinking, Browser MCP)
  - And more!
- **API Key Management**: Secure handling of API keys for servers that require them
- **Category Filtering**: Browse servers by category (Utilities, AI Tools, Web Tools, etc.)

### 🔒 Security & Privacy
- **Environment Variable Protection**: Password-masked environment variables with toggle visibility
- **Local Storage**: All settings and configurations stored locally
- **Backup System**: Automatic backup creation before any configuration changes

### 🎨 User Experience  
- **Dark Mode**: Full dark mode support with system preference detection
- **Modal Interface**: Clean modal-based workflow replacing browser alerts
- **Settings Management**: Customizable Claude config path and theme preferences
- **Import/Export**: JSON import functionality for bulk server configuration

## Installation

### Prerequisites
- [Rust](https://rustup.rs/) (for building from source)
- [Node.js](https://nodejs.org/) (for development)

### From Source
```bash
# Clone the repository
git clone <repository-url>
cd mcp-manager

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Usage

### Getting Started
1. Launch the MCP Manager application
2. The app will automatically detect your Claude Desktop configuration file location
3. View all currently configured MCP servers in the main interface

### Managing Servers

#### Adding a New Server
1. Click "Add New Server"
2. Fill in server details:
   - **Name**: Unique identifier for the server
   - **Command**: Executable command (e.g., `npx`, `uvx`, `docker`)
   - **Arguments**: Command-line arguments
   - **Environment Variables**: API keys and configuration (optional)
3. Click "Save"

#### Quick Install Preset Servers
1. Click "Quick Install"
2. Browse servers by category or view all
3. Click "Install" on any server
4. For servers requiring API keys, enter your credentials when prompted

#### Editing Existing Servers
1. Click the edit button (✏️) next to any server
2. Modify the configuration as needed
3. Environment variables are hidden by default - click the eye icon to reveal values
4. Click "Save" to apply changes

#### Importing from JSON
1. Click "Import from JSON"
2. Paste JSON configuration in one of two formats:
   - Individual server config with name field
   - Full `mcpServers` object with multiple servers
3. Click "Import Server(s)"

### Settings

Access settings via the gear icon in the top-right corner:

- **Claude Config Path**: Customize the location of your Claude Desktop config file (leave empty for OS default)
- **Dark Mode**: Toggle between light and dark themes

## Configuration File Locations

The app automatically detects the correct Claude Desktop configuration file location based on your operating system:

- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`  
- **Linux**: `~/.config/Claude/claude_desktop_config.json`

## Architecture

### Tech Stack
- **Frontend**: TypeScript, HTML, CSS
- **Backend**: Rust with Tauri framework
- **Build System**: Vite for frontend bundling
- **Configuration**: JSON-based settings with localStorage persistence

### Project Structure
```
src/
├── main.ts          # Main TypeScript application logic
├── styles.css       # Application styling with dark mode support
└── index.html       # Main HTML template

src-tauri/
└── src/
    └── lib.rs       # Rust backend with MCP server management logic
```

### Key Components

#### Backend (Rust)
- `parse_claude_json()`: Loads and parses Claude config file
- `add_server()`, `update_server()`, `delete_server()`: CRUD operations
- `get_default_config_path()`: OS-specific path detection
- Automatic backup creation and error handling

#### Frontend (TypeScript)
- Modal-based UI system for all user interactions
- Settings management with localStorage persistence
- Dark mode implementation with CSS custom properties
- Preset server database with categorization
- Environment variable security with masked inputs

## Development

### Development Commands
```bash
# Start development server
npm run tauri dev

# Build for production
npm run tauri build

# Run frontend only
npm run dev

# Lint and type check
npm run lint
npm run typecheck
```

### Adding New Preset Servers
Edit the `PRESET_SERVERS` array in `src/main.ts`:

```typescript
{
  name: "server-name",
  description: "Server description",
  category: "Category",
  command: "command",
  args: ["arg1", "arg2"],
  requiresApiKey: true, // if API key needed
  apiKeyName: "API_KEY_NAME",
  apiKeyDescription: "Instructions for getting API key"
}
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Commit your changes (`git commit -m 'Add amazing feature'`)
5. Push to the branch (`git push origin feature/amazing-feature`)
6. Open a Pull Request

## Security

- Environment variables are stored securely and masked by default
- Configuration backups are created before any modifications
- No sensitive data is transmitted over the network
- All operations are performed locally

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Support

For issues, feature requests, or questions:
1. Check existing GitHub issues
2. Create a new issue with detailed information
3. Include your operating system and error messages if applicable

## Acknowledgments

- Built with [Tauri](https://tauri.app/) framework
- Integrates with [Model Context Protocol](https://modelcontextprotocol.io/)
- Designed for use with [Claude Desktop](https://claude.ai/desktop)

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
