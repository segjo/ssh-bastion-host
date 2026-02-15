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
version: '3.8'

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
