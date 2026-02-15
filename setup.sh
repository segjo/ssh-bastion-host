#!/bin/bash
set -e

echo "SSH Bastion Host Setup Script"
echo "=============================="
echo

# Check if ssh-data directory exists
if [ -d "./ssh-data" ]; then
    echo "Warning: ssh-data directory already exists."
    read -p "Do you want to overwrite it? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Setup cancelled."
        exit 0
    fi
    rm -rf ./ssh-data
fi

# Create ssh-data directory
echo "Creating ssh-data directory..."
mkdir -p ./ssh-data

# Check if user has an SSH public key
if [ -f "$HOME/.ssh/id_rsa.pub" ]; then
    echo "Found SSH public key: $HOME/.ssh/id_rsa.pub"
    read -p "Do you want to use this key? (Y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        cat "$HOME/.ssh/id_rsa.pub" > ./ssh-data/authorized_keys
    else
        echo "Please paste your public SSH key (press Ctrl+D when done):"
        cat > ./ssh-data/authorized_keys
    fi
elif [ -f "$HOME/.ssh/id_ed25519.pub" ]; then
    echo "Found SSH public key: $HOME/.ssh/id_ed25519.pub"
    read -p "Do you want to use this key? (Y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        cat "$HOME/.ssh/id_ed25519.pub" > ./ssh-data/authorized_keys
    else
        echo "Please paste your public SSH key (press Ctrl+D when done):"
        cat > ./ssh-data/authorized_keys
    fi
else
    echo "No SSH public key found."
    echo "Please paste your public SSH key (press Ctrl+D when done):"
    cat > ./ssh-data/authorized_keys
fi

# Set correct permissions
echo "Setting correct permissions..."
chmod 700 ./ssh-data
chmod 600 ./ssh-data/authorized_keys

# Set ownership to UID 1000 (bastion user in container)
echo "Setting ownership to UID 1000 (bastion user)..."
if command -v sudo &> /dev/null; then
    sudo chown -R 1000:1000 ./ssh-data
else
    echo "Warning: sudo not found. You may need to manually set ownership:"
    echo "  chown -R 1000:1000 ./ssh-data"
fi

echo
echo "Setup complete!"
echo
echo "Your authorized_keys file:"
cat ./ssh-data/authorized_keys
echo
echo "To start the SSH bastion host, run:"
echo "  docker-compose up -d"
echo "or"
echo "  docker run -d --name ssh-bastion -p 2222:22 -v \$(pwd)/ssh-data:/home/bastion/.ssh:ro ssh-bastion-host"
echo
echo "To connect:"
echo "  ssh -p 2222 bastion@localhost"
