import { invoke } from "@tauri-apps/api/core";

interface McpServerInfo {
  name: string;
  command: string;
  args: string[];
  env: { [key: string]: string };
}

interface McpServerEdit {
  command: string;
  args: string[];
  env: { [key: string]: string };
}

interface SaveResult {
  success: boolean;
  message: string;
}

interface ApiKeyRequirement {
  name: string;
  description: string;
  required: boolean;
}

interface PresetServer {
  name: string;
  description: string;
  category: string;
  serverType: string;
  command: string;
  args: string[];
  env?: { [key: string]: string };
  apiKeys: ApiKeyRequirement[];
  requiresApiKey: boolean;
  // Legacy fields for backward compatibility
  apiKeyName?: string;
  apiKeyDescription?: string;
}

let mcpListEl: HTMLElement | null;
let modalEl: HTMLElement | null;
let currentEditingServer: string | null = null;

// Settings
interface AppSettings {
  claudeConfigPath: string;
  darkMode: boolean;
}

let appSettings: AppSettings = {
  claudeConfigPath: '',
  darkMode: false
};


// Modal types
type ModalType = 'notification' | 'confirmation';

interface ModalOptions {
  title: string;
  message: string;
  type: ModalType;
  onConfirm?: () => void;
  onCancel?: () => void;
  confirmText?: string;
  cancelText?: string;
}

function showModal(options: ModalOptions) {
  if (!modalEl) return;
  
  const isConfirmation = options.type === 'confirmation';
  const confirmText = options.confirmText || 'OK';
  const cancelText = options.cancelText || 'Cancel';
  
  modalEl.innerHTML = `
    <div class="modal-overlay" onclick="${isConfirmation ? '' : 'closeNotificationModal()'}">
      <div class="modal-content notification-modal" onclick="event.stopPropagation()">
        <h2>${options.title}</h2>
        <div class="modal-message">
          ${options.message.split('\n').map(line => `<p>${line}</p>`).join('')}
        </div>
        <div class="modal-actions">
          ${isConfirmation ? `<button type="button" onclick="handleModalCancel()">${cancelText}</button>` : ''}
          <button type="button" onclick="handleModalConfirm()" class="primary-btn">${confirmText}</button>
        </div>
      </div>
    </div>
  `;
  
  modalEl.style.display = 'block';
  
  // Store callbacks for the modal
  (window as any).currentModalOptions = options;
}

function showNotification(title: string, message: string) {
  showModal({
    title,
    message,
    type: 'notification'
  });
}

function showConfirmation(title: string, message: string, onConfirm: () => void, onCancel?: () => void) {
  showModal({
    title,
    message,
    type: 'confirmation',
    onConfirm,
    onCancel,
    confirmText: 'Yes',
    cancelText: 'Cancel'
  });
}

function handleModalConfirm() {
  const options = (window as any).currentModalOptions as ModalOptions;
  closeNotificationModal();
  if (options?.onConfirm) {
    options.onConfirm();
  }
}

function handleModalCancel() {
  const options = (window as any).currentModalOptions as ModalOptions;
  closeNotificationModal();
  if (options?.onCancel) {
    options.onCancel();
  }
}

function closeNotificationModal() {
  if (modalEl) {
    modalEl.style.display = 'none';
    modalEl.innerHTML = '';
  }
  (window as any).currentModalOptions = null;
}

async function loadSettings() {
  try {
    // First check for localStorage migration
    await migrateFromLocalStorage();
    
    // Load settings from file
    const savedSettings: AppSettings = await invoke("load_app_settings");
    appSettings = { ...appSettings, ...savedSettings };
    
    // Apply dark mode setting
    applyDarkMode(appSettings.darkMode);
  } catch (error) {
    console.warn('Failed to load settings:', error);
    // Fall back to default settings
    appSettings = {
      claudeConfigPath: '',
      darkMode: false
    };
  }
}

async function saveSettings() {
  try {
    const result: SaveResult = await invoke("save_app_settings", { settings: appSettings });
    if (!result.success) {
      console.error('Failed to save settings:', result.message);
    }
  } catch (error) {
    console.error('Error saving settings:', error);
  }
}

async function migrateFromLocalStorage() {
  // Check if localStorage has settings and no file exists yet
  const localStorageSettings = localStorage.getItem('mcp-manager-settings');
  if (!localStorageSettings) {
    return; // No migration needed
  }
  
  try {
    // Check if settings file already exists
    const settingsPath: string = await invoke("get_settings_path");
    
    // Try to load existing file settings to see if migration already happened
    try {
      await invoke("load_app_settings");
      // File exists and is readable, migration already done
      localStorage.removeItem('mcp-manager-settings');
      return;
    } catch {
      // File doesn't exist or is corrupted, proceed with migration
    }
    
    // Parse localStorage settings
    const localSettings = JSON.parse(localStorageSettings);
    const migratedSettings: AppSettings = {
      claudeConfigPath: localSettings.claudeConfigPath || '',
      darkMode: localSettings.darkMode || false
    };
    
    // Save to file
    const result: SaveResult = await invoke("save_app_settings", { settings: migratedSettings });
    if (result.success) {
      console.log('Successfully migrated settings from localStorage to file');
      localStorage.removeItem('mcp-manager-settings');
      
      // Show user notification about migration
      showNotification("Settings Migrated", "Your settings have been migrated to a more persistent storage location.");
    }
  } catch (error) {
    console.warn('Failed to migrate settings from localStorage:', error);
  }
}

function applyDarkMode(enabled: boolean) {
  if (enabled) {
    document.documentElement.setAttribute('data-theme', 'dark');
  } else {
    document.documentElement.removeAttribute('data-theme');
  }
}

async function showSettingsModal() {
  if (!modalEl) return;
  
  // Get the default config path for the current OS
  let defaultPath = "Default system path";
  try {
    defaultPath = await invoke("get_default_config_path");
  } catch (error) {
    console.warn("Could not get default config path:", error);
  }
  
  modalEl.innerHTML = `
    <div class="modal-overlay" onclick="closeModal()">
      <div class="modal-content settings-modal" onclick="event.stopPropagation()">
        <h2>Settings</h2>
        
        <form id="settings-form">
          <div class="form-group">
            <label for="claude-config-path">Claude Desktop Config Path:</label>
            <input type="text" id="claude-config-path" value="${appSettings.claudeConfigPath}" 
                   placeholder="${defaultPath}">
            <small class="help-text">Leave empty to use the default path: ${defaultPath}</small>
          </div>
          
          <div class="form-group">
            <label class="checkbox-label">
              <input type="checkbox" id="dark-mode-toggle" ${appSettings.darkMode ? 'checked' : ''}>
              <span class="checkbox-custom"></span>
              Enable Dark Mode
            </label>
          </div>
          
          <div class="modal-actions">
            <button type="button" onclick="closeModal()">Cancel</button>
            <button type="submit" class="primary-btn">Save Settings</button>
          </div>
        </form>
      </div>
    </div>
  `;
  
  modalEl.style.display = 'block';
  
  // Handle form submission
  document.querySelector("#settings-form")?.addEventListener("submit", handleSettingsSubmit);
  
  // Handle dark mode toggle for immediate preview
  document.querySelector("#dark-mode-toggle")?.addEventListener("change", (e) => {
    const target = e.target as HTMLInputElement;
    applyDarkMode(target.checked);
  });
}

async function handleSettingsSubmit(e: Event) {
  e.preventDefault();
  
  const claudeConfigPath = (document.querySelector("#claude-config-path") as HTMLInputElement).value.trim();
  const darkMode = (document.querySelector("#dark-mode-toggle") as HTMLInputElement).checked;
  
  appSettings.claudeConfigPath = claudeConfigPath;
  appSettings.darkMode = darkMode;
  
  try {
    await saveSettings();
    applyDarkMode(darkMode);
    showNotification("Success", "Settings saved successfully!");
    closeModal();
  } catch (error) {
    showNotification("Error", `Failed to save settings: ${error}`);
  }
}

async function loadMcpServers() {
  try {
    const customPath = appSettings.claudeConfigPath || null;
    const servers: McpServerInfo[] = await invoke("parse_claude_json", { customPath });
    displayMcpServers(servers);
  } catch (error) {
    if (mcpListEl) {
      mcpListEl.innerHTML = `<p class="error">Error loading MCP servers: ${error}</p>`;
    }
  }
}

function displayMcpServers(servers: McpServerInfo[]) {
  if (!mcpListEl) return;
  
  if (servers.length === 0) {
    mcpListEl.innerHTML = '<p>No MCP servers found in .claude.json</p>';
    return;
  }
  
  const serverElements = servers.map(server => `
    <div class="mcp-server">
      <div class="server-header">
        <h3>${server.name}</h3>
        <div class="server-actions">
          <button class="edit-btn" onclick="editServer('${server.name}')" title="Edit server">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/>
            </svg>
          </button>
          <button class="delete-btn" onclick="deleteServer('${server.name}')" title="Delete server">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
            </svg>
          </button>
        </div>
      </div>
      <div class="server-details">
        <p><strong>Command:</strong> ${server.command}</p>
        <p><strong>Arguments:</strong> ${server.args.join(' ')}</p>
        ${Object.keys(server.env).length > 0 ? 
          `<p><strong>Environment Variables:</strong> ${Object.keys(server.env).join(', ')}</p>` : 
          ''
        }
      </div>
    </div>
  `).join('');
  
  mcpListEl.innerHTML = `
    <h2>MCP Servers (${servers.length})</h2>
    <div class="button-group">
      <button id="add-server-btn" class="add-btn">Add New Server</button>
      <button id="import-json-btn" class="add-btn">Import from JSON</button>
      <button id="quick-install-btn" class="quick-install-btn">Quick Install</button>
      <button id="load-mcp-btn" class="add-btn">Reload MCP Servers</button>
    </div>
    <div class="servers-grid">
      ${serverElements}
    </div>
  `;
  
  // Re-attach button event listeners
  document.querySelector("#add-server-btn")?.addEventListener("click", () => addNewServer());
  document.querySelector("#import-json-btn")?.addEventListener("click", () => showImportModal());
  document.querySelector("#quick-install-btn")?.addEventListener("click", () => showQuickInstallModal());
  document.querySelector("#load-mcp-btn")?.addEventListener("click", loadMcpServers);
}

async function editServer(name: string) {
  try {
    const server: McpServerInfo = await invoke("get_server_details", { name });
    currentEditingServer = name;
    showEditModal(server);
  } catch (error) {
    showNotification("Error", `Error loading server details: ${error}`);
  }
}

async function deleteServer(name: string) {
  showConfirmation(
    "Delete Server",
    `Are you sure you want to delete the server "${name}"?`,
    async () => {
      try {
        const result: SaveResult = await invoke("delete_server", { name });
        if (result.success) {
          showNotification("Success", result.message);
          loadMcpServers();
        } else {
          showNotification("Error", result.message);
        }
      } catch (error) {
        showNotification("Error", `Error deleting server: ${error}`);
      }
    }
  );
}

function addNewServer() {
  currentEditingServer = null;
  const emptyServer: McpServerInfo = {
    name: "",
    command: "",
    args: [],
    env: {}
  };
  showEditModal(emptyServer);
}

async function showQuickInstallModal() {
  if (!modalEl) return;
  
  try {
    // Get preset servers from Rust backend
    const presetServers: PresetServer[] = await invoke("get_preset_servers");
    
    // Get current servers to filter out already installed ones
    const currentServers = getCurrentServerNames();
    const availableServers = presetServers.filter(server => !currentServers.includes(server.name));
    
    // Get categories and server types from backend
    const categories: string[] = await invoke("get_preset_server_categories");
    const serverTypes: string[] = await invoke("get_server_types");
    
    const categoryTabs = categories.map(category => `
      <button class="category-tab" data-category="${category}">${category}</button>
    `).join('');
    
    const typeTabs = serverTypes.map(type => `
      <button class="type-tab" data-type="${type}">${type.toUpperCase()}</button>
    `).join('');
    
    const serverCards = availableServers.map(server => {
      // Check API key requirements (new format or legacy)
      const apiKeys = server.apiKeys && server.apiKeys.length > 0 ? server.apiKeys : 
        (server.apiKeyName ? [{name: server.apiKeyName, description: '', required: true}] : []);
      
      const requiredKeys = apiKeys.filter(key => key.required);
      const optionalKeys = apiKeys.filter(key => !key.required);
      
      let apiKeyBadge = '';
      if (requiredKeys.length > 0 && optionalKeys.length > 0) {
        apiKeyBadge = `<span class="api-key-badge">API Keys: ${requiredKeys.length} required, ${optionalKeys.length} optional</span>`;
      } else if (requiredKeys.length > 0) {
        apiKeyBadge = `<span class="api-key-badge">${requiredKeys.length > 1 ? `${requiredKeys.length} API Keys Required` : 'API Key Required'}</span>`;
      } else if (optionalKeys.length > 0) {
        apiKeyBadge = `<span class="api-key-badge optional">Optional API Key</span>`;
      }
      
      return `
        <div class="preset-server-card" data-category="${server.category}" data-type="${server.serverType}">
          <div class="server-info">
            <h4>${server.name}</h4>
            <p class="server-description">${server.description}</p>
            <div class="server-meta">
              <span class="category-badge">${server.category}</span>
              <span class="server-type-badge server-type-${server.serverType}">${server.serverType.toUpperCase()}</span>
              ${apiKeyBadge}
            </div>
          </div>
          <button class="install-preset-btn" onclick="installPresetServer('${server.name}')">Install</button>
        </div>
      `;
    }).join('');
    
    modalEl.innerHTML = `
      <div class="modal-overlay" onclick="closeModal()">
        <div class="modal-content quick-install-modal" onclick="event.stopPropagation()">
          <h2>Quick Install MCP Servers</h2>
          
          ${availableServers.length === 0 ? 
            '<div class="no-servers-message"><p>All preset servers are already installed!</p></div>' :
            `<div class="filter-tabs">
              <div class="category-tabs">
                <label>By Category:</label>
                <button class="category-tab active" data-category="all">All</button>
                ${categoryTabs}
              </div>
              <div class="type-tabs">
                <label>By Type:</label>
                <button class="type-tab active" data-type="all">All</button>
                ${typeTabs}
              </div>
            </div>
            
            <div class="preset-servers-grid">
              ${serverCards}
            </div>`
          }
          
          <div class="modal-actions">
            <button type="button" onclick="closeModal()">Close</button>
          </div>
        </div>
      </div>
    `;
    
    modalEl.style.display = 'block';
    
    // Add filtering functionality
    let selectedCategory = 'all';
    let selectedType = 'all';
    
    function filterCards() {
      document.querySelectorAll('.preset-server-card').forEach(card => {
        const cardElement = card as HTMLElement;
        const cardCategory = cardElement.dataset.category;
        const cardType = cardElement.dataset.type;
        
        const categoryMatch = selectedCategory === 'all' || cardCategory === selectedCategory;
        const typeMatch = selectedType === 'all' || cardType === selectedType;
        
        if (categoryMatch && typeMatch) {
          cardElement.style.display = 'block';
        } else {
          cardElement.style.display = 'none';
        }
      });
    }
    
    // Category filter functionality
    document.querySelectorAll('.category-tab').forEach(tab => {
      tab.addEventListener('click', (e) => {
        const target = e.target as HTMLElement;
        selectedCategory = target.dataset.category || 'all';
        
        // Update active tab
        document.querySelectorAll('.category-tab').forEach(t => t.classList.remove('active'));
        target.classList.add('active');
        
        filterCards();
      });
    });
    
    // Type filter functionality
    document.querySelectorAll('.type-tab').forEach(tab => {
      tab.addEventListener('click', (e) => {
        const target = e.target as HTMLElement;
        selectedType = target.dataset.type || 'all';
        
        // Update active tab
        document.querySelectorAll('.type-tab').forEach(t => t.classList.remove('active'));
        target.classList.add('active');
        
        filterCards();
      });
    });
  } catch (error) {
    showNotification("Error", `Failed to load preset servers: ${error}`);
  }
}

function getCurrentServerNames(): string[] {
  // This would normally get current servers from the state
  // For now, we'll make an API call or use a stored list
  // TODO: Optimize this by storing current server list in memory
  return [];
}

async function installPresetServer(serverName: string) {
  try {
    const server: PresetServer | null = await invoke("get_preset_server_by_name", { name: serverName });
    if (!server) {
      showNotification("Error", `Server "${serverName}" not found in presets`);
      return;
    }
    
    // Check if server requires API keys (either new format or legacy)
    const hasApiKeys = (server.apiKeys && server.apiKeys.length > 0) || 
                      (server.requiresApiKey && server.apiKeyName);
    const hasRequiredApiKeys = server.apiKeys?.some(key => key.required) || 
                              (server.requiresApiKey && server.apiKeyName);
    
    if (hasApiKeys) {
      // Show API key input modal
      showApiKeyModal(server);
    } else {
      // Install directly (no API keys required)
      await performServerInstallation(server, {});
    }
  } catch (error) {
    showNotification("Error", `Failed to get server details: ${error}`);
  }
}

async function performServerInstallation(server: PresetServer, env: { [key: string]: string }) {
  const serverData: McpServerEdit = {
    command: server.command,
    args: server.args,
    env: { ...server.env, ...env }
  };
  
  try {
    const result: SaveResult = await invoke("add_server", { name: server.name, serverData });
    
    if (result.success) {
      showNotification("Success", `${server.name} installed successfully!`);
      closeModal();
      loadMcpServers();
    } else {
      showNotification("Installation Failed", result.message);
    }
  } catch (error) {
    showNotification("Installation Error", `Failed to install ${server.name}: ${error}`);
  }
}

function showApiKeyModal(server: PresetServer) {
  if (!modalEl) return;
  
  // Use new apiKeys structure if available, fallback to legacy fields
  const apiKeys = server.apiKeys && server.apiKeys.length > 0 ? server.apiKeys : 
    (server.apiKeyName ? [{
      name: server.apiKeyName,
      description: server.apiKeyDescription || "API key required",
      required: true
    }] : []);
  
  if (apiKeys.length === 0) {
    showNotification("Error", "No API key configuration found for this server");
    return;
  }
  
  const requiredKeys = apiKeys.filter(key => key.required);
  const optionalKeys = apiKeys.filter(key => !key.required);
  
  const keyInputs = apiKeys.map(apiKey => `
    <div class="form-group">
      <label for="api-key-${apiKey.name}">
        ${apiKey.name}${apiKey.required ? ' *' : ' (optional)'}:
      </label>
      <input type="password" 
             id="api-key-${apiKey.name}" 
             placeholder="Enter your ${apiKey.name}" 
             ${apiKey.required ? 'required' : ''}>
      <small class="api-key-help">${apiKey.description}</small>
    </div>
  `).join('');
  
  modalEl.innerHTML = `
    <div class="modal-overlay" onclick="closeModal()">
      <div class="modal-content api-key-modal" onclick="event.stopPropagation()">
        <h2>API Keys Required</h2>
        <p>The <strong>${server.name}</strong> server requires API keys to function.</p>
        ${requiredKeys.length > 0 ? `<p><strong>Required:</strong> ${requiredKeys.length} API key(s)</p>` : ''}
        ${optionalKeys.length > 0 ? `<p><strong>Optional:</strong> ${optionalKeys.length} API key(s) for enhanced features</p>` : ''}
        
        <form id="api-key-form">
          ${keyInputs}
          
          <div class="modal-actions">
            <button type="button" onclick="closeModal()">Cancel</button>
            <button type="submit" class="primary-btn">Install with API Keys</button>
          </div>
        </form>
      </div>
    </div>
  `;
  
  modalEl.style.display = 'block';
  
  // Handle form submission
  document.querySelector("#api-key-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    
    const env: { [key: string]: string } = {};
    let hasRequiredKeys = true;
    
    // Collect all API keys
    for (const apiKey of apiKeys) {
      const input = document.querySelector(`#api-key-${apiKey.name}`) as HTMLInputElement;
      const value = input?.value.trim() || '';
      
      if (apiKey.required && !value) {
        showNotification("Validation Error", `Please enter ${apiKey.name}`);
        hasRequiredKeys = false;
        break;
      }
      
      if (value) {
        env[apiKey.name] = value;
      }
    }
    
    if (hasRequiredKeys) {
      await performServerInstallation(server, env);
    }
  });
}

function showImportModal() {
  if (!modalEl) return;
  
  modalEl.innerHTML = `
    <div class="modal-overlay" onclick="closeModal()">
      <div class="modal-content" onclick="event.stopPropagation()">
        <h2>Import Server(s) from JSON</h2>
        <form id="import-form">
          <div class="form-group">
            <label for="server-name-import">Server Name (required only for individual server config):</label>
            <input type="text" id="server-name-import" placeholder="Enter server name">
          </div>
          
          <div class="form-group">
            <label for="json-input">JSON Configuration:</label>
            <textarea id="json-input" placeholder='Paste JSON here. Supports two formats:

1. Individual server:
{
  "command": "npx",
  "args": ["-y", "@modelcontextprotocol/server-example"],
  "env": {
    "API_KEY": "your-api-key"
  }
}

2. Multiple servers (mcpServers format):
{
  "mcpServers": {
    "server-name": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-example"]
    }
  }
}' rows="12" required></textarea>
          </div>
          
          <div class="modal-actions">
            <button type="button" onclick="closeModal()">Cancel</button>
            <button type="submit">Import Server(s)</button>
          </div>
        </form>
      </div>
    </div>
  `;
  
  modalEl.style.display = 'block';
  
  // Attach form submit handler
  document.querySelector("#import-form")?.addEventListener("submit", handleImportSubmit);
}

function showEditModal(server: McpServerInfo) {
  if (!modalEl) return;
  
  const envEntries = Object.entries(server.env);
  const envHtml = envEntries.length > 0 ? 
    envEntries.map(([key, value], index) => `
      <div class="env-var">
        <input type="text" class="env-key" value="${key}" placeholder="Variable name">
        <div class="env-value-container">
          <input type="password" class="env-value" value="${value}" placeholder="Variable value">
          <button type="button" class="toggle-visibility-btn" onclick="toggleEnvVisibility(this)" title="Show/Hide value">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" class="eye-icon">
              <path d="M12 4.5C7 4.5 2.73 7.61 1 12c1.73 4.39 6 7.5 11 7.5s9.27-3.11 11-7.5c-1.73-4.39-6-7.5-11-7.5zM12 17c-2.76 0-5-2.24-5-5s2.24-5 5-5 5 2.24 5 5-2.24 5-5 5zm0-8c-1.66 0-3 1.34-3 3s1.34 3 3 3 3-1.34 3-3-1.34-3-3-3z"/>
            </svg>
          </button>
        </div>
        <button type="button" onclick="removeEnvVar(this)">Remove</button>
      </div>
    `).join('') : '';
  
  modalEl.innerHTML = `
    <div class="modal-overlay" onclick="closeModal()">
      <div class="modal-content" onclick="event.stopPropagation()">
        <h2>${currentEditingServer ? 'Edit Server' : 'Add New Server'}</h2>
        <form id="server-form">
          <div class="form-group">
            <label for="server-name">Server Name:</label>
            <input type="text" id="server-name" value="${server.name}" ${currentEditingServer ? 'readonly' : ''} required>
          </div>
          
          <div class="form-group">
            <label for="server-command">Command:</label>
            <input type="text" id="server-command" value="${server.command}" required>
          </div>
          
          <div class="form-group">
            <label for="server-args">Arguments (space-separated):</label>
            <input type="text" id="server-args" value="${server.args.join(' ')}">
          </div>
          
          <div class="form-group">
            <label>Environment Variables:</label>
            <div id="env-vars">
              ${envHtml}
            </div>
            <button type="button" onclick="addEnvVar()">Add Environment Variable</button>
          </div>
          
          <div class="modal-actions">
            <button type="button" onclick="closeModal()">Cancel</button>
            <button type="submit">Save</button>
          </div>
        </form>
      </div>
    </div>
  `;
  
  modalEl.style.display = 'block';
  
  // Attach form submit handler
  document.querySelector("#server-form")?.addEventListener("submit", handleFormSubmit);
}

function closeModal() {
  if (modalEl) {
    modalEl.style.display = 'none';
    modalEl.innerHTML = '';
  }
  currentEditingServer = null;
}

function addEnvVar() {
  const envVarsContainer = document.querySelector("#env-vars");
  if (envVarsContainer) {
    const envVar = document.createElement('div');
    envVar.className = 'env-var';
    envVar.innerHTML = `
      <input type="text" class="env-key" placeholder="Variable name">
      <div class="env-value-container">
        <input type="password" class="env-value" placeholder="Variable value">
        <button type="button" class="toggle-visibility-btn" onclick="toggleEnvVisibility(this)" title="Show/Hide value">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" class="eye-icon">
            <path d="M12 4.5C7 4.5 2.73 7.61 1 12c1.73 4.39 6 7.5 11 7.5s9.27-3.11 11-7.5c-1.73-4.39-6-7.5-11-7.5zM12 17c-2.76 0-5-2.24-5-5s2.24-5 5-5 5 2.24 5 5-2.24 5-5 5zm0-8c-1.66 0-3 1.34-3 3s1.34 3 3 3 3-1.34 3-3-1.34-3-3-3z"/>
          </svg>
        </button>
      </div>
      <button type="button" onclick="removeEnvVar(this)">Remove</button>
    `;
    envVarsContainer.appendChild(envVar);
  }
}

function removeEnvVar(button: HTMLElement) {
  button.parentElement?.remove();
}

function toggleEnvVisibility(button: HTMLElement) {
  const container = button.closest('.env-value-container');
  const input = container?.querySelector('.env-value') as HTMLInputElement;
  const eyeIcon = button.querySelector('.eye-icon path');
  
  if (!input || !eyeIcon) return;
  
  if (input.type === 'password') {
    input.type = 'text';
    // Change to "eye-off" icon
    eyeIcon.setAttribute('d', 'M9.88 9.88a3 3 0 1 0 4.24 4.24m-6.06-6.06L12 4.5c5 0 9.27 3.11 11 7.5a11.79 11.79 0 0 1-4 5.83m-15.44-8.84C2.73 8.61 7 5.5 12 5.5a9.77 9.77 0 0 1 3.14.51M3 3l18 18');
  } else {
    input.type = 'password';
    // Change back to "eye" icon
    eyeIcon.setAttribute('d', 'M12 4.5C7 4.5 2.73 7.61 1 12c1.73 4.39 6 7.5 11 7.5s9.27-3.11 11-7.5c-1.73-4.39-6-7.5-11-7.5zM12 17c-2.76 0-5-2.24-5-5s2.24-5 5-5 5 2.24 5 5-2.24 5-5 5zm0-8c-1.66 0-3 1.34-3 3s1.34 3 3 3 3-1.34 3-3-1.34-3-3-3z');
  }
}

async function handleFormSubmit(e: Event) {
  e.preventDefault();
  
  const form = e.target as HTMLFormElement;
  const formData = new FormData(form);
  
  const name = (document.querySelector("#server-name") as HTMLInputElement).value.trim();
  const command = (document.querySelector("#server-command") as HTMLInputElement).value.trim();
  const argsString = (document.querySelector("#server-args") as HTMLInputElement).value.trim();
  const args = argsString ? argsString.split(' ').filter(arg => arg.length > 0) : [];
  
  // Collect environment variables
  const env: { [key: string]: string } = {};
  const envVars = document.querySelectorAll("#env-vars .env-var");
  envVars.forEach(envVar => {
    const key = (envVar.querySelector(".env-key") as HTMLInputElement).value.trim();
    const value = (envVar.querySelector(".env-value") as HTMLInputElement).value.trim();
    if (key && value) {
      env[key] = value;
    }
  });
  
  if (!name || !command) {
    showNotification("Validation Error", "Please fill in all required fields");
    return;
  }
  
  const serverData: McpServerEdit = {
    command,
    args,
    env
  };
  
  try {
    let result: SaveResult;
    if (currentEditingServer) {
      result = await invoke("update_server", { name: currentEditingServer, serverData });
    } else {
      result = await invoke("add_server", { name, serverData });
    }
    
    if (result.success) {
      showNotification("Success", result.message);
      closeModal();
      loadMcpServers();
    } else {
      showNotification("Error", result.message);
    }
  } catch (error) {
    showNotification("Error", `Error saving server: ${error}`);
  }
}

async function handleImportSubmit(e: Event) {
  e.preventDefault();
  
  const name = (document.querySelector("#server-name-import") as HTMLInputElement).value.trim();
  const jsonInput = (document.querySelector("#json-input") as HTMLTextAreaElement).value.trim();
  
  if (!jsonInput) {
    showNotification("Validation Error", "Please provide JSON configuration");
    return;
  }
  
  try {
    // Parse the JSON input
    const jsonData = JSON.parse(jsonInput);
    
    let serversToImport: { [key: string]: any } = {};
    
    // Check if this is wrapped in mcpServers format
    if (jsonData.mcpServers && typeof jsonData.mcpServers === 'object') {
      serversToImport = jsonData.mcpServers;
    } else if (jsonData.command) {
      // This is a single server config
      if (!name) {
        showNotification("Validation Error", "Please provide a server name for the individual server configuration");
        return;
      }
      serversToImport[name] = jsonData;
    } else {
      showNotification("Validation Error", "JSON must contain either 'mcpServers' object or a server configuration with 'command' field");
      return;
    }
    
    // Import all servers
    let successCount = 0;
    let errorMessages: string[] = [];
    
    for (const [serverName, serverConfig] of Object.entries(serversToImport)) {
      try {
        // Validate required fields
        if (!serverConfig.command) {
          errorMessages.push(`Server '${serverName}': missing 'command' field`);
          continue;
        }
        
        // Extract data with defaults
        const command = serverConfig.command;
        const args = Array.isArray(serverConfig.args) ? serverConfig.args : [];
        const env = (typeof serverConfig.env === 'object' && serverConfig.env !== null) ? serverConfig.env : {};
        
        const serverData: McpServerEdit = {
          command,
          args,
          env
        };
        
        // Add the server
        const result: SaveResult = await invoke("add_server", { name: serverName, serverData });
        
        if (result.success) {
          successCount++;
        } else {
          errorMessages.push(`Server '${serverName}': ${result.message}`);
        }
      } catch (error) {
        errorMessages.push(`Server '${serverName}': ${error}`);
      }
    }
    
    // Show results
    let message = "";
    if (successCount > 0) {
      message += `Successfully imported ${successCount} server(s).`;
    }
    if (errorMessages.length > 0) {
      message += `\n\nErrors:\n${errorMessages.join('\n')}`;
    }
    
    const title = successCount > 0 ? "Import Results" : "Import Failed";
    showNotification(title, message);
    
    if (successCount > 0) {
      closeModal();
      loadMcpServers();
    }
    
  } catch (error) {
    if (error instanceof SyntaxError) {
      showNotification("JSON Error", "Invalid JSON format. Please check your JSON syntax.");
    } else {
      showNotification("Import Error", `Error importing server: ${error}`);
    }
  }
}

// Make functions globally available
(window as any).editServer = editServer;
(window as any).deleteServer = deleteServer;
(window as any).addEnvVar = addEnvVar;
(window as any).removeEnvVar = removeEnvVar;
(window as any).toggleEnvVisibility = toggleEnvVisibility;
(window as any).closeModal = closeModal;
(window as any).showImportModal = showImportModal;
(window as any).showQuickInstallModal = showQuickInstallModal;
(window as any).showSettingsModal = showSettingsModal;
(window as any).installPresetServer = installPresetServer;
(window as any).handleModalConfirm = handleModalConfirm;
(window as any).handleModalCancel = handleModalCancel;
(window as any).closeNotificationModal = closeNotificationModal;

window.addEventListener("DOMContentLoaded", async () => {
  mcpListEl = document.querySelector("#mcp-list");
  modalEl = document.querySelector("#modal");
  
  // Load settings first
  await loadSettings();
  
  // Attach settings button event listener
  document.querySelector("#settings-btn")?.addEventListener("click", showSettingsModal);
  
  // Load MCP servers on startup
  loadMcpServers();
});
