# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Tauri desktop application called "mcp-manager" with a TypeScript/Vite frontend and Rust backend. The project follows Tauri's recommended architecture with:

- Frontend: TypeScript + Vite (port 1420)
- Backend: Rust with Tauri framework
- Communication: Frontend invokes Rust commands via Tauri's IPC system

## Development Commands

### Development
- `npm run dev` - Start development server (frontend + backend hot reload)
- `npm run tauri dev` - Alternative way to start Tauri development mode

### Building
- `npm run build` - Build TypeScript and create production frontend
- `npm run tauri build` - Build complete desktop application for distribution

### Other
- `npm run preview` - Preview the built frontend
- `tsc` - Type check TypeScript code

## Architecture

### Frontend Structure
- `src/main.ts` - Main TypeScript entry point with DOM manipulation and Tauri API calls
- `index.html` - HTML template with basic UI structure
- `src/styles.css` - Application styles
- `vite.config.ts` - Vite configuration optimized for Tauri development

### Backend Structure
- `src-tauri/src/lib.rs` - Main Rust library with Tauri commands (e.g., `greet` function)
- `src-tauri/src/main.rs` - Rust application entry point
- `src-tauri/Cargo.toml` - Rust dependencies and configuration
- `src-tauri/tauri.conf.json` - Tauri app configuration (window settings, build commands, bundle config)

### Key Patterns
- Tauri commands are defined in Rust using `#[tauri::command]` attribute
- Frontend calls backend via `invoke()` from `@tauri-apps/api/core`
- All Tauri commands must be registered in the `invoke_handler` in `lib.rs`
- Development server runs on fixed port 1420 (configured in vite.config.ts and tauri.conf.json)

## Important Notes

- The Rust backend library is named `mcp_manager_lib` to avoid naming conflicts on Windows
- TypeScript is configured with strict mode and unused parameter checking
- Vite is configured to ignore watching `src-tauri` directory to prevent conflicts
- The app window is configured as 800x600 pixels by default