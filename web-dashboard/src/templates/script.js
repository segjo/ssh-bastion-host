/**
 * SSH Bastion Dashboard - Modern JavaScript
 * Real-time connection monitoring with smooth animations
 */

const CONFIG = {
    REFRESH_INTERVAL: 5000,
    API_ENDPOINT: '/api/connections/json',
    ANIMATION_STAGGER: 80
};

let autoRefreshEnabled = true;
let refreshInterval = null;
let previousCount = 0;
let isLoading = false;

// Initialize application
document.addEventListener('DOMContentLoaded', init);

function init() {
    console.log('%c🔐 SSH Bastion Dashboard', 'color: #22d3ee; font-size: 20px; font-weight: bold;');
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
        const response = await fetch(CONFIG.API_ENDPOINT);
        
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
    container.innerHTML = `<div class="connections-container">${cards}</div>`;
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
                    <span class="label">Bastion Host</span>
                    <span class="value">${escape(conn.bastion_host)}:${conn.bastion_port}</span>
                </div>
                <div class="connection-row">
                    <span class="label">Reverse Port</span>
                    <span class="value highlight">*:${conn.remote_port}</span>
                </div>
                <div class="connection-row">
                    <span class="label">Local Target</span>
                    <span class="value">${escape(conn.local_bind)}:${conn.local_port}</span>
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
