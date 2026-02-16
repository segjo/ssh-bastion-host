#!/bin/bash

###############################################################################
# SSH Bastion AutoSSH Client Setup Script
# 
# Sets up automatic SSH reverse tunnel to bastion host with autostart
# Generates keys, installs autossh, and configures autostart
###############################################################################

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
AUTOSTART_METHOD="crontab"
KEY_NAME="autossh_bastion_key"
SSH_USER="autossh"
BASTION_USER="bastion"
KEY_TYPE="ed25519"

###############################################################################
# Usage function
###############################################################################
usage() {
    cat << EOF
${BLUE}SSH Bastion AutoSSH Client Setup${NC}

Usage: $0 [OPTIONS]

${YELLOW}Required Options:${NC}
  -H, --host              Bastion host (IP or hostname)
  -P, --port              Bastion SSH port (default: 22)
  -R, --reverse-port      Reverse tunnel port on bastion (e.g., 8080)

${YELLOW}Optional Options:${NC}
  -u, --bastion-user      Username on bastion (default: bastion)
  -s, --autostart         Autostart method: systemd, crontab, manual (default: crontab)
  -k, --key-name          SSH key name (default: autossh_bastion_key)
  --ssh-user              Local user for autossh service (default: autossh)
  --local-port            Local port to forward (default: 22)
  --root                  Run as root (for crontab on root user)
  -h, --help              Show this help message

${YELLOW}Examples:${NC}
  # Setup with crontab (default)
  $0 -H bastion.example.com -P 2222 -R 8080

  # Setup with systemd
  $0 -H 192.168.1.10 -P 22 -R 9090 -s systemd

  # Setup as root with custom key name
  $0 -H bastion.local -R 8888 -k my_bastion_key --root

EOF
    exit 1
}

###############################################################################
# Helper functions
###############################################################################
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

prompt_yes_no() {
    local prompt="$1"
    local response
    
    while true; do
        read -p "$(echo -e ${BLUE}$prompt${NC}) (y/n): " response
        case "$response" in
            [yY][eE][sS]|[yY])
                return 0
                ;;
            [nN][oO]|[nN])
                return 1
                ;;
            *)
                echo "Please answer y or n."
                ;;
        esac
    done
}

###############################################################################
# Check if running as root
###############################################################################
check_root() {
    if [ "$EUID" -ne 0 ]; then 
        log_error "This script must be run as root"
        exit 1
    fi
}

###############################################################################
# Install dependencies
###############################################################################
install_dependencies() {
    log_info "Installing required packages (autossh, openssh-client)..."
    
    if command -v apt-get &> /dev/null; then
        apt-get update
        apt-get install -y autossh openssh-client
    elif command -v yum &> /dev/null; then
        yum install -y autossh openssh-clients
    elif command -v brew &> /dev/null; then
        brew install autossh
    else
        log_error "No supported package manager found (apt, yum, or brew)"
        exit 1
    fi
    
    log_success "Dependencies installed"
}

###############################################################################
# Create or verify autossh user
###############################################################################
setup_autossh_user() {
    if [ "$RUN_AS_ROOT" = true ]; then
        log_info "Running as root user"
        SSH_HOME="/root"
        return
    fi
    
    log_info "Setting up autossh service user..."
    
    if ! id "$SSH_USER" &>/dev/null; then
        log_info "Creating user: $SSH_USER"
        useradd -m -s /bin/false "$SSH_USER" || useradd -m -s /bin/sh "$SSH_USER"
        log_success "User $SSH_USER created"
    else
        log_warning "User $SSH_USER already exists"
    fi
    
    SSH_HOME="/home/$SSH_USER"
}

###############################################################################
# Generate SSH key
###############################################################################
generate_ssh_key() {
    local key_path="$SSH_HOME/.ssh/$KEY_NAME"
    
    mkdir -p "$SSH_HOME/.ssh"
    
    if [ -f "$key_path" ]; then
        log_warning "Key already exists at $key_path"
        if ! prompt_yes_no "Overwrite existing key?"; then
            log_info "Using existing key"
            return
        fi
    fi
    
    log_info "Generating $KEY_TYPE SSH key for autossh tunnel..."
    
    if [ "$RUN_AS_ROOT" = true ]; then
        ssh-keygen -t "$KEY_TYPE" -f "$key_path" -C "autossh@$(hostname)" -N ""
    else
        sudo -u "$SSH_USER" ssh-keygen -t "$KEY_TYPE" -f "$key_path" -C "autossh@$(hostname)" -N ""
    fi
    
    # Set correct permissions
    if [ "$RUN_AS_ROOT" = false ]; then
        chown "$SSH_USER:$SSH_USER" "$key_path"
        chown "$SSH_USER:$SSH_USER" "$key_path.pub"
    fi
    chmod 600 "$key_path"
    chmod 644 "$key_path.pub"
    
    log_success "SSH key generated at $key_path"
}

###############################################################################
# Create tunnel script
###############################################################################
create_tunnel_script() {
    local script_path="$SSH_HOME/tunnel.sh"
    
    log_info "Creating tunnel script at $script_path..."
    
    cat > "$script_path" << 'TUNNEL_SCRIPT'
#!/bin/bash
# AutoSSH Tunnel Script
# Maintains reverse SSH tunnel to bastion host

TUNNEL_ENDPOINT="BASTION_USER@BASTION_HOST"
TUNNEL_PORT="BASTION_PORT"
REVERSE_PORT="REVERSE_PORT"
LOCAL_PORT="LOCAL_PORT"
KEY_PATH="KEY_PATH"
MONITORE_PORT="MONITOR_PORT"

# Check if tunnel is already running
if pgrep -f "autossh.*TUNNEL_ENDPOINT" > /dev/null; then
    echo "Tunnel already running"
    exit 0
fi

# Start autossh tunnel
exec autossh -M "$MONITORE_PORT" \
    -f \
    -N \
    -i "$KEY_PATH" \
    -R "*:$REVERSE_PORT:localhost:$LOCAL_PORT" \
    -p "$TUNNEL_PORT" \
    -o "ServerAliveInterval=30" \
    -o "ServerAliveCountMax=3" \
    -o "StrictHostKeyChecking=accept-new" \
    -o "ExitOnForwardFailure=yes" \
    "$TUNNEL_ENDPOINT"
TUNNEL_SCRIPT

    # Replace placeholders
    sed -i "s|TUNNEL_ENDPOINT|$BASTION_USER@$BASTION_HOST|g" "$script_path"
    sed -i "s|BASTION_PORT|$BASTION_PORT|g" "$script_path"
    sed -i "s|REVERSE_PORT|$REVERSE_PORT|g" "$script_path"
    sed -i "s|LOCAL_PORT|$LOCAL_PORT|g" "$script_path"
    sed -i "s|KEY_PATH|$SSH_HOME/.ssh/$KEY_NAME|g" "$script_path"
    sed -i "s|MONITOR_PORT|$((REVERSE_PORT + 1000))|g" "$script_path"
    
    chmod 755 "$script_path"
    
    if [ "$RUN_AS_ROOT" = false ]; then
        chown "$SSH_USER:$SSH_USER" "$script_path"
    fi
    
    log_success "Tunnel script created"
}

###############################################################################
# Setup systemd service
###############################################################################
setup_systemd() {
    log_info "Setting up systemd service for autossh..."
    
    local service_file="/etc/systemd/system/autossh-bastion.service"
    local service_user="root"
    
    if [ "$RUN_AS_ROOT" = false ]; then
        service_user="$SSH_USER"
    fi
    
    cat > "$service_file" << SERVICE_FILE
[Unit]
Description=AutoSSH Reverse Tunnel to Bastion Host
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$service_user
ExecStart=$SSH_HOME/tunnel.sh
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
SERVICE_FILE

    systemctl daemon-reload
    systemctl enable autossh-bastion
    
    log_success "Systemd service created and enabled"
    
    log_info "Starting autossh service..."
    systemctl start autossh-bastion
    
    log_success "Autossh service started"
    echo ""
    echo -e "${BLUE}Service commands:${NC}"
    echo "  Check status:  systemctl status autossh-bastion"
    echo "  View logs:     journalctl -u autossh-bastion -f"
    echo "  Stop service:  systemctl stop autossh-bastion"
    echo "  Start service: systemctl start autossh-bastion"
}

###############################################################################
# Setup crontab
###############################################################################
setup_crontab() {
    local cron_cmd="@reboot sleep 10 && $SSH_HOME/tunnel.sh"
    local cron_check="*/5 * * * * $SSH_HOME/tunnel.sh > /tmp/autossh.log 2>&1"
    
    if [ "$RUN_AS_ROOT" = true ]; then
        log_info "Setting up crontab for root..."
        
        # Create temporary crontab file
        crontab -l 2>/dev/null || true > /tmp/crontab_temp
        
        # Add lines if they don't exist
        if ! grep -q "tunnel.sh" /tmp/crontab_temp; then
            echo "$cron_cmd" >> /tmp/crontab_temp
            echo "$cron_check" >> /tmp/crontab_temp
        fi
        
        crontab /tmp/crontab_temp
        rm -f /tmp/crontab_temp
    else
        log_info "Setting up crontab for user $SSH_USER..."
        
        # Create temporary crontab file
        sudo -u "$SSH_USER" crontab -l 2>/dev/null > /tmp/crontab_temp || true
        
        # Add lines if they don't exist
        if ! grep -q "tunnel.sh" /tmp/crontab_temp; then
            echo "$cron_cmd" >> /tmp/crontab_temp
            echo "$cron_check" >> /tmp/crontab_temp
        fi
        
        sudo -u "$SSH_USER" crontab /tmp/crontab_temp
        rm -f /tmp/crontab_temp
    fi
    
    log_success "Crontab configured"
    echo ""
    echo -e "${BLUE}Crontab entries:${NC}"
    echo "  @reboot sleep 10 && $SSH_HOME/tunnel.sh"
    echo "  */5 * * * * $SSH_HOME/tunnel.sh > /tmp/autossh.log 2>&1"
    echo ""
    echo -e "${BLUE}Check crontab:${NC}"
    if [ "$RUN_AS_ROOT" = true ]; then
        echo "  crontab -l"
    else
        echo "  sudo -u $SSH_USER crontab -l"
    fi
}

###############################################################################
# Display public key
###############################################################################
display_public_key() {
    local key_path="$SSH_HOME/.ssh/$KEY_NAME.pub"
    
    echo ""
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}PUBLIC KEY FOR BASTION AUTHORIZED_KEYS${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo ""
    cat "$key_path"
    echo ""
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo -e "${YELLOW}Add this key to the bastion's authorized_keys:${NC}"
    echo ""
    echo "On bastion host:"
    echo "  cat ~/.ssh/authorized_keys"
    echo ""
    echo "Or add it directly if autossh is running:"
    echo "  docker exec ssh-bastion bash -c 'echo \"$(cat $key_path)\" >> /home/bastion/.ssh/authorized_keys'"
    echo "  docker exec ssh-bastion chmod 600 /home/bastion/.ssh/authorized_keys"
    echo ""
}

###############################################################################
# Verify tunnel connectivity
###############################################################################
verify_tunnel() {
    echo ""
    echo -e "${BLUE}Verifying tunnel connectivity...${NC}"
    
    sleep 2
    
    if pgrep -f "autossh.*$BASTION_HOST" > /dev/null; then
        log_success "Tunnel is running"
    else
        log_warning "Tunnel process not found immediately. This is normal if using crontab."
        log_info "Tunnel will start at next reboot or on next cron check (5 minute interval)"
    fi
    
    echo ""
    echo -e "${BLUE}Test tunnel access from bastion:${NC}"
    echo "  ssh -p $REVERSE_PORT bastion@$BASTION_HOST"
    echo ""
}

###############################################################################
# Main execution
###############################################################################
main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -H|--host)
                BASTION_HOST="$2"
                shift 2
                ;;
            -P|--port)
                BASTION_PORT="$2"
                shift 2
                ;;
            -R|--reverse-port)
                REVERSE_PORT="$2"
                shift 2
                ;;
            -u|--bastion-user)
                BASTION_USER="$2"
                shift 2
                ;;
            -s|--autostart)
                AUTOSTART_METHOD="$2"
                shift 2
                ;;
            -k|--key-name)
                KEY_NAME="$2"
                shift 2
                ;;
            --ssh-user)
                SSH_USER="$2"
                shift 2
                ;;
            --local-port)
                LOCAL_PORT="$2"
                shift 2
                ;;
            --root)
                RUN_AS_ROOT=true
                shift
                ;;
            -h|--help)
                usage
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                ;;
        esac
    done
    
    # Validate required arguments
    if [ -z "$BASTION_HOST" ] || [ -z "$REVERSE_PORT" ]; then
        log_error "Missing required arguments"
        usage
    fi
    
    # Set defaults
    BASTION_PORT="${BASTION_PORT:-22}"
    LOCAL_PORT="${LOCAL_PORT:-22}"
    RUN_AS_ROOT="${RUN_AS_ROOT:-false}"
    
    # Check root
    check_root
    
    echo ""
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}SSH BASTION AUTOSSH CLIENT SETUP${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo ""
    
    log_info "Configuration:"
    echo "  Bastion Host:      $BASTION_HOST"
    echo "  Bastion Port:      $BASTION_PORT"
    echo "  Bastion User:      $BASTION_USER"
    echo "  Reverse Port:      $REVERSE_PORT"
    echo "  Local Port:        $LOCAL_PORT"
    echo "  Autostart Method:  $AUTOSTART_METHOD"
    echo "  SSH Key Name:      $KEY_NAME"
    if [ "$RUN_AS_ROOT" = false ]; then
        echo "  Service User:      $SSH_USER"
    else
        echo "  Service User:      root"
    fi
    echo ""
    
    if ! prompt_yes_no "Continue with setup?"; then
        log_info "Setup cancelled"
        exit 0
    fi
    
    echo ""
    
    # Execute setup steps
    install_dependencies
    setup_autossh_user
    generate_ssh_key
    create_tunnel_script
    
    case "$AUTOSTART_METHOD" in
        systemd)
            setup_systemd
            ;;
        crontab)
            setup_crontab
            ;;
        manual)
            log_info "Manual mode selected. Run $SSH_HOME/tunnel.sh manually when needed"
            ;;
        *)
            log_error "Unknown autostart method: $AUTOSTART_METHOD"
            exit 1
            ;;
    esac
    
    display_public_key
    verify_tunnel
    
    log_success "Setup complete!"
}

# Run main function
main "$@"
