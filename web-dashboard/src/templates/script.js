/**
 * SSH Bastion Dashboard - Modern JavaScript
 * Real-time connection monitoring with smooth animations
 * 
 * Demo Mode: Add ?demo=1 to URL to load demo-data.json for local testing
 * Example: http://localhost:3000/?demo=1
 */

const CONFIG = {
    REFRESH_INTERVAL: 5000,
    API_ENDPOINT: '/api/connections/json',
    DEMO_ENDPOINT: '/web-dashboard/src/templates/demo-data.json',
    ANIMATION_STAGGER: 80
};

let autoRefreshEnabled = true;
let refreshInterval = null;
let previousCount = 0;
let isLoading = false;
let demoMode = new URLSearchParams(window.location.search).has('demo');
let lastConnectionsJSON = null;

// Initialize application
document.addEventListener('DOMContentLoaded', init);

function init() {
    console.log('%c🔐 SSH Bastion Dashboard', 'color: #22d3ee; font-size: 20px; font-weight: bold;');
    if (demoMode) {
        console.log('%c📋 DEMO MODE ENABLED', 'color: #f59e0b; font-weight: bold;');
        console.log('%cUsing demo-data.json for testing', 'color: #f59e0b;');
    }
    console.log('%cInitializing...', 'color: #94a3b8;');
    
    setupEventListeners();
    loadConnections();
    
    if (autoRefreshEnabled) {
        startAutoRefresh();
    }
}

function setupEventListeners() {
    // Refresh button
    const refreshBtn = document.getElementById('refresh-btn');
    if (refreshBtn) {
        refreshBtn.addEventListener('click', handleRefreshClick);
    }
    
    // Auto-refresh toggle
    const autoRefreshToggle = document.getElementById('auto-refresh');
    if (autoRefreshToggle) {
        autoRefreshToggle.addEventListener('change', handleAutoRefreshToggle);
    }
    
    // Keyboard shortcuts
    document.addEventListener('keydown', handleKeydown);
    
    // Visibility change - pause when tab is hidden
    document.addEventListener('visibilitychange', handleVisibilityChange);
}

function handleKeydown(e) {
    // R key to refresh (without modifiers)
    if (e.key.toLowerCase() === 'r' && !e.ctrlKey && !e.metaKey && !e.altKey) {
        const activeElement = document.activeElement;
        if (activeElement.tagName !== 'INPUT' && activeElement.tagName !== 'TEXTAREA') {
            e.preventDefault();
            loadConnections();
        }
    }
}

function handleVisibilityChange() {
    if (document.hidden) {
        stopAutoRefresh();
    } else if (autoRefreshEnabled) {
        loadConnections();
        startAutoRefresh();
    }
}

async function handleRefreshClick() {
    const btn = document.getElementById('refresh-btn');
    if (!btn || isLoading) return;
    
    // Button animation
    btn.style.transform = 'scale(0.95)';
    btn.style.opacity = '0.8';
    
    await loadConnections();
    
    // Reset button
    setTimeout(() => {
        btn.style.transform = '';
        btn.style.opacity = '';
    }, 150);
}

function handleAutoRefreshToggle(e) {
    autoRefreshEnabled = e.target.checked;
    
    if (autoRefreshEnabled) {
        startAutoRefresh();
        console.log('%c⏱️  Auto-refresh enabled', 'color: #10b981;');
    } else {
        stopAutoRefresh();
        console.log('%c⏸️  Auto-refresh paused', 'color: #f59e0b;');
    }
}

async function loadConnections() {
    if (isLoading) return;
    isLoading = true;
    
    try {
        const endpoint = demoMode ? CONFIG.DEMO_ENDPOINT : CONFIG.API_ENDPOINT;
        const response = await fetch(endpoint);
        
        if (!response.ok) {
            throw new Error(`Server error: ${response.status}`);
        }
        
        const data = await response.json();
        
        updateHeader(data);
        renderConnections(data.connections);
        updateSystemStatus(true);
        
        console.log(`%c✓ ${data.total_connections} connection(s)`, 'color: #10b981;');
        
    } catch (error) {
        console.error('%c✗ Failed to load', 'color: #ef4444;', error.message);
        updateSystemStatus(false);
        renderError(error.message);
    } finally {
        isLoading = false;
    }
}

function updateHeader(data) {
    // Connection count with animation
    const countEl = document.getElementById('connection-count');
    if (countEl) {
        const newCount = data.total_connections;
        
        if (newCount !== previousCount) {
            countEl.style.transform = 'scale(1.3)';
            countEl.style.color = '#22d3ee';
            countEl.textContent = newCount;
            
            setTimeout(() => {
                countEl.style.transform = 'scale(1)';
            }, 200);
            
            previousCount = newCount;
        }
    }
    
    // Last update time
    const timeEl = document.getElementById('last-update');
    if (timeEl && data.timestamp) {
        const time = new Date(data.timestamp * 1000); // Convert Unix timestamp to milliseconds
        timeEl.textContent = time.toLocaleTimeString('en-US', {
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
            hour12: false
        });
    }
}

function updateSystemStatus(isOnline) {
    const statusEl = document.getElementById('system-status');
    if (!statusEl) return;
    
    if (isOnline) {
        statusEl.textContent = 'Online';
        statusEl.className = 'value status-online';
        statusEl.style.color = '#10b981';
    } else {
        statusEl.textContent = 'Offline';
        statusEl.className = 'value';
        statusEl.style.color = '#ef4444';
    }
}

function renderConnections(connections) {
    const container = document.getElementById('connections-container');
    if (!container) return;
    
    // Check if connections data has changed
    const connectionsJSON = JSON.stringify(connections);
    if (lastConnectionsJSON === connectionsJSON) {
        // No changes, skip re-render to prevent flickering
        return;
    }
    lastConnectionsJSON = connectionsJSON;
    
    if (!connections || connections.length === 0) {
        container.innerHTML = `
            <div class="no-connections">
                No active tunnels
                <small>Waiting for reverse SSH connections...</small>
            </div>
        `;
        return;
    }
    
    const cards = connections.map((conn, index) => createConnectionCard(conn, index)).join('');
    container.innerHTML = cards;
    
    // Attach copy handlers
    attachCopyHandlers();
}

function createConnectionCard(conn, index) {
    const statusClass = conn.status === 'Connected' ? '' : 'disconnected';
    const delay = index * CONFIG.ANIMATION_STAGGER;
    
    return `
        <div class="connection-card" style="animation-delay: ${delay}ms">
            <div class="connection-header">
                <span class="status-badge ${statusClass}">${escape(conn.status)}</span>
                <span class="pid">PID ${conn.pid}</span>
            </div>
            <div class="connection-body">
                <div class="connection-row">
                    <span class="label">User</span>
                    <span class="value">${escape(conn.user)}</span>
                </div>
                <div class="connection-row">
                    <span class="label">Reverse Port</span>
                    <span class="value highlight">*:${conn.remote_port}</span>
                </div>
                <div class="connection-row command-row">
                    <div class="command-content">
                        <span class="label">Connect Command</span>
                        <span class="value command-text" data-command="${escape(conn.command)}" style="font-size: 0.85rem;">${escape(conn.command)}</span>
                    </div>
                    <button class="btn-copy" data-command="${escape(conn.command)}" title="Copy command">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path>
                            <rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect>
                        </svg>
                    </button>
                </div>
            </div>
        </div>
    `;
}

function renderError(message) {
    const container = document.getElementById('connections-container');
    if (!container) return;
    
    container.innerHTML = `
        <div class="no-connections" style="border-color: rgba(239, 68, 68, 0.3);">
            Connection Error
            <small>${escape(message)}</small>
        </div>
    `;
}

function attachCopyHandlers() {
    const copyButtons = document.querySelectorAll('.btn-copy');
    copyButtons.forEach(button => {
        button.addEventListener('click', function(e) {
            e.preventDefault();
            const command = this.getAttribute('data-command');
            copyToClipboard(command, this);
        });
    });
}

function copyToClipboard(text, button) {
    navigator.clipboard.writeText(text).then(() => {
        // Visual feedback
        const originalHTML = button.innerHTML;
        const originalTitle = button.title;
        
        button.innerHTML = `
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="20 6 9 17 4 12"></polyline>
            </svg>
        `;
        button.title = 'Copied!';
        button.classList.add('copied');
        
        setTimeout(() => {
            button.innerHTML = originalHTML;
            button.title = originalTitle;
            button.classList.remove('copied');
        }, 2000);
        
        console.log('%c✓ Command copied to clipboard', 'color: #10b981;');
    }).catch(err => {
        console.error('%c✗ Failed to copy', 'color: #ef4444;', err);
    });
}

function startAutoRefresh() {
    stopAutoRefresh();
    refreshInterval = setInterval(loadConnections, CONFIG.REFRESH_INTERVAL);
}

function stopAutoRefresh() {
    if (refreshInterval) {
        clearInterval(refreshInterval);
        refreshInterval = null;
    }
}

// Utility: Escape HTML
function escape(text) {
    if (!text) return '';
    const el = document.createElement('div');
    el.textContent = text;
    return el.innerHTML;
}

// Performance: Log page load time
window.addEventListener('load', () => {
    if (window.performance) {
        const timing = performance.timing;
        const loadTime = timing.loadEventEnd - timing.navigationStart;
        console.log(`%c⚡ Loaded in ${loadTime}ms`, 'color: #06b6d4;');
    }
});
