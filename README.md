# SSH Bastion Host

A secure SSH bastion host Docker container with public key authentication only.

## Features

- **Secure by default**: Only public key authentication is allowed
- **No password authentication**: PasswordAuthentication is disabled
- **No root login**: Root login is completely disabled
- **Volume-mapped authorized_keys**: Easy management of SSH keys via Docker volumes
- **Ubuntu 22.04 LTS base**: Reliable and well-supported base image

## Quick Start

### Easy Setup (Recommended)

Use the provided setup script to automatically configure your SSH keys and permissions:

```bash
./setup.sh
```

This script will:

- Create the `ssh-data` directory
- Copy your public SSH key to `authorized_keys`
- Set correct permissions (700 for directory, 600 for file)
- Set correct ownership (UID 1000 for bastion user)

After running the setup script, start the container with:

```bash
docker-compose up -d
```

### Manual Setup

### Using Docker Run

1. Create an `authorized_keys` file with your public SSH keys:

```bash
mkdir -p ./ssh-data
cat ~/.ssh/id_rsa.pub > ./ssh-data/authorized_keys
```

2. Set correct permissions (important for SSH security):

```bash
chmod 700 ./ssh-data
chmod 600 ./ssh-data/authorized_keys
# Set ownership to UID 1000 (bastion user in container)
sudo chown -R 1000:1000 ./ssh-data
```

3. Run the container:

```bash
docker run -d \
  --name ssh-bastion \
  -p 2222:22 \
  -v $(pwd)/ssh-data:/home/bastion/.ssh:ro \
  ssh-bastion-host
```

4. Connect to the bastion host:

```bash
ssh -p 2222 bastion@localhost
```

### Using Docker Compose

Create a `docker-compose.yml` file:

```yaml
services:
  ssh-bastion:
    build: .
    ports:
      - "2222:22"
    volumes:
      - ./ssh-data:/home/bastion/.ssh:ro
    restart: unless-stopped
```

Then run:

```bash
docker-compose up -d
```

## Building the Image

```bash
docker build -t ssh-bastion-host .
```

## Security Considerations

- The container runs SSH on port 22, which you should map to a different host port
- Only public key authentication is allowed - no password authentication
- Root login is disabled
- The bastion user has minimal permissions
- Mount the authorized_keys directory as read-only (`:ro`) for additional security

## Volume Mounting

The container expects the authorized_keys file to be mounted at:

```
/home/bastion/.ssh/authorized_keys
```

**Important:** For SSH to accept the keys, you must set the correct permissions on the host before mounting:

- `.ssh` directory: `chmod 700` and `chown 1000:1000`
- `authorized_keys` file: `chmod 600` and `chown 1000:1000`

The UID/GID 1000 corresponds to the `bastion` user inside the container.

You can either:

1. Mount the entire `.ssh` directory: `-v /path/to/ssh-data:/home/bastion/.ssh:ro`
2. Mount just the authorized_keys file: `-v /path/to/authorized_keys:/home/bastion/.ssh/authorized_keys:ro`

It's recommended to mount as read-only (`:ro`) for additional security.

## Configuration

The SSH daemon is configured with the following security settings:

- `PermitRootLogin no`
- `PasswordAuthentication no`
- `PubkeyAuthentication yes`
- `PermitEmptyPasswords no`
- `ChallengeResponseAuthentication no`
- `KbdInteractiveAuthentication no`
- `GatewayPorts yes` - Allows remote port forwarding from clients
- `AllowTcpForwarding yes` - Enables port forwarding for tunnel functionality
- `ClientAliveInterval 30` - Keeps connections alive with periodic pings
- `ClientAliveCountMax 99999` - Maintains long-lived tunnel connections

## Reverse SSH with AutoSSH

### Overview

A reverse SSH bastion allows you to securely access internal machines behind a firewall through a forward-facing bastion host. The client machine initiates an outbound SSH connection that creates a reverse tunnel, allowing inbound connections from the bastion.

**Use cases:**

- Access internal machines from outside the network without opening firewall ports
- Provide secure administrative access without exposing machines directly
- Create temporary secure tunnels to specific services

### Prerequisites

On the **client machine** (internal, behind firewall):

```bash
# Install autossh and openssh-client
sudo apt-get install autossh openssh-client

# Or on macOS:
brew install autossh
```

### Creating a Secure Key Pair

Generate a dedicated SSH key for the autossh tunnel. Using a separate key allows you to:

- Revoke access without affecting your personal SSH keys
- Restrict the key to only allow reverse port forwarding
- Grant different access levels to different machines

#### Generate the Key

Use Ed25519 for modern, secure key generation:

```bash
# Generate a new Ed25519 key (recommended)
ssh-keygen -t ed25519 -f ~/.ssh/autossh_bastion_key -C "autossh@internal-machine" -N ""

# Or use RSA (4096-bit) for broader compatibility
ssh-keygen -t rsa -b 4096 -f ~/.ssh/autossh_bastion_key -C "autossh@internal-machine" -N ""
```

Options:

- `-t ed25519` - Use Ed25519 (modern, compact, secure)
- `-f ~/.ssh/autossh_bastion_key` - Save to this file path
- `-C "comment"` - Add a descriptive comment
- `-N ""` - No passphrase (required for unattended autossh service)

#### Verify Key Generation

```bash
ls -la ~/.ssh/autossh_bastion_key*
# Should show:
# -rw------- autossh_bastion_key (private key - 600 permissions)
# -rw-r--r-- autossh_bastion_key.pub (public key)
```

#### Add Public Key to Bastion

On your **local machine** (where you cloned the bastion repo):

```bash
# Copy the public key to authorized_keys
cat ~/.ssh/autossh_bastion_key.pub >> ./ssh-data/authorized_keys

# Verify the key was added
cat ./ssh-data/authorized_keys
```

If you plan to add this to a deployed bastion:

```bash
# Add to a running container
docker exec ssh-bastion bash -c 'echo "YOUR_PUBLIC_KEY_HERE" >> /home/bastion/.ssh/authorized_keys'

# Then verify permissions
docker exec ssh-bastion chmod 600 /home/bastion/.ssh/authorized_keys
docker exec ssh-bastion chmod 700 /home/bastion/.ssh
```

#### Security Best Practices for Keys

1. **Protect your private key:**

   ```bash
   # Ensure private key has correct permissions
   chmod 600 ~/.ssh/autossh_bastion_key

   # Never share or commit private keys to version control
   # Add to .gitignore if in a repo directory
   echo "autossh_bastion_key" >> .gitignore
   ```

2. **Restrict the key (Optional - Advanced)**

   Add this option prefix to the public key in `authorized_keys` to restrict what the key can do:

   ```bash
   no-agent-forwarding,no-X11-forwarding,permitopen="127.0.0.1:8080",command="/usr/bin/false" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA...
   ```

   This key can only:
   - Create reverse tunnels to port 8080
   - Cannot open interactive shells
   - Cannot forward X11 or agent

3. **Rotate keys periodically:**

   ```bash
   # Generate a new key when needed
   ssh-keygen -t ed25519 -f ~/.ssh/autossh_bastion_key_new -C "autossh@internal-machine" -N ""

   # Add the new public key to authorized_keys
   cat ~/.ssh/autossh_bastion_key_new.pub >> ./ssh-data/authorized_keys

   # Once verified working, remove the old public key from authorized_keys
   # Then delete the old private key:
   rm ~/.ssh/autossh_bastion_key
   mv ~/.ssh/autossh_bastion_key_new ~/.ssh/autossh_bastion_key
   ```

4. **Manage multiple keys:**

   If you have different services needing bastion access, create separate keys for each:

   ```bash
   ssh-keygen -t ed25519 -f ~/.ssh/autossh_service1 -C "service1@internal" -N ""
   ssh-keygen -t ed25519 -f ~/.ssh/autossh_service2 -C "service2@internal" -N ""

   # Add all to authorized_keys
   cat ~/.ssh/autossh_service1.pub >> ./ssh-data/authorized_keys
   cat ~/.ssh/autossh_service2.pub >> ./ssh-data/authorized_keys
   ```

Create a reverse tunnel that forwards `localhost:8080` on the bastion to `localhost:8080` on your client:

```bash
ssh -R *:8080:localhost:8080 bastion@bastion.example.com -p 2222 -N -T
```

- `-R *:8080:localhost:8080` - Creates reverse tunnel (\* makes it accessible from outside)
- `-p 2222` - SSH port on bastion (adjust if needed)
- `-N` - Don't execute remote commands
- `-T` - Disable pseudo-terminal allocation

### Using AutoSSH for Automatic Reconnection

AutoSSH automatically reconnects if the tunnel drops, essential for reliable reverse proxies.

#### Manual Command

```bash
autossh -M 20000 -R *:8080:localhost:8080 bastion@bastion.example.com -p 2222 -N -T -o "ServerAliveInterval 30" -o "ServerAliveCountMax 3"
```

- `-M 20000` - Monitoring port (used internally by autossh)
- `-o "ServerAliveInterval 30"` - Keep-alive interval in seconds
- `-o "ServerAliveCountMax 3"` - Number of keep-alive probes before timeout

#### Setup as Systemd Service

Create `/etc/systemd/system/autossh-bastion.service`:

```ini
[Unit]
Description=AutoSSH Reverse Tunnel to Bastion Host
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=autossh
ExecStart=/usr/bin/autossh -M 20000 -R *:8080:localhost:8080 bastion@bastion.example.com -p 2222 -N -T -o "ServerAliveInterval 30" -o "ServerAliveCountMax 3" -o "StrictHostKeyChecking=accept-new" -i /home/autossh/.ssh/id_rsa
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Setup and start the service:

```bash
# Create autossh user
sudo useradd -m -s /bin/false autossh

# Create SSH key for autossh user
sudo -u autossh ssh-keygen -t ed25519 -f /home/autossh/.ssh/id_rsa -N ""

# Add the public key to bastion's authorized_keys
cat /home/autossh/.ssh/id_rsa.pub >> ./ssh-data/authorized_keys

# Fix permissions
sudo chown -R autossh:autossh /home/autossh/.ssh

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable autossh-bastion
sudo systemctl start autossh-bastion

# Check status
sudo systemctl status autossh-bastion
sudo journalctl -u autossh-bastion -f
```

#### Setup as Cron Job

For a simpler setup without systemd, add to a user's crontab:

```bash
crontab -e
```

Add:

```cron
*/5 * * * * pgrep -f 'autossh.*bastion' || /usr/bin/autossh -M 20000 -R *:8080:localhost:8080 bastion@bastion.example.com -p 2222 -N -T -o ServerAliveInterval=30 -o ServerAliveCountMax=3 -i ~/.ssh/id_rsa > /tmp/autossh.log 2>&1
```

This checks every 5 minutes if the tunnel is running, and reconnects if it's down.

### Multiple Port Forwards

Forward multiple services through a single tunnel:

```bash
autossh -M 20000 \
  -R *:8080:localhost:8080 \
  -R *:3306:localhost:3306 \
  -R *:5432:localhost:5432 \
  bastion@bastion.example.com -p 2222 -N -T \
  -o "ServerAliveInterval 30" -o "ServerAliveCountMax 3"
```

### Connecting Through the Bastion

From an external machine, to connect to a service via the bastion:

```bash
# Connect to the reverse-forwarded local service
mysql -h bastion.example.com -u user -p

# Or via SSH on forwarded port
ssh -p 8080 user@bastion.example.com
```

### Monitoring the Tunnel

Check if the tunnel is active:

```bash
ps aux | grep autossh
ss -tlnp | grep :8080  # Check if port is listening
```

### Troubleshooting AutoSSH

- **Tunnel keeps disconnecting**: Increase `ServerAliveCountMax` or check firewall timeout settings
- **Connection refused**: Verify `GatewayPorts yes` is set on bastion (it is by default)
- **Permission denied**: Check that the autossh user's public key is in `authorized_keys`
- **Port already in use**: Change the `8080` port to something else or stop conflicting processes

## Troubleshooting

### Connection Refused

- Ensure the container is running: `docker ps`
- Check container logs: `docker logs ssh-bastion`

### Permission Denied (publickey)

- Verify your public key is in the mounted authorized_keys file
- **Check file permissions and ownership on the host:**
  - `.ssh` directory must be `700` with owner `1000:1000`
  - `authorized_keys` file must be `600` with owner `1000:1000`
- Check permissions in container: `docker exec ssh-bastion ls -la /home/bastion/.ssh/`
- Ensure you're using the correct private key: `ssh -i /path/to/key -p 2222 bastion@localhost`

### Container Exits Immediately

- Check logs: `docker logs ssh-bastion`
- Verify the entrypoint script is executable

## License

MIT
