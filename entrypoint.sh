#!/bin/sh
set -e

# Ensure correct permissions on .ssh directory and authorized_keys
if [ -f /home/bastion/.ssh/authorized_keys ]; then
    # Try to set ownership and permissions, but don't fail if read-only
    chown bastion:bastion /home/bastion/.ssh/authorized_keys 2>/dev/null || echo "Note: Could not change ownership of authorized_keys (possibly read-only mount)"
    chmod 600 /home/bastion/.ssh/authorized_keys 2>/dev/null || echo "Note: Could not change permissions of authorized_keys (possibly read-only mount)"
    echo "Authorized keys file found"
else
    echo "Warning: No authorized_keys file found in /home/bastion/.ssh/"
    echo "Mount your authorized_keys file to /home/bastion/.ssh/authorized_keys"
fi

# Ensure .ssh directory has correct permissions
chown -R bastion:bastion /home/bastion/.ssh 2>/dev/null || echo "Note: Could not change ownership of .ssh directory (possibly read-only mount)"
chmod 700 /home/bastion/.ssh 2>/dev/null || echo "Note: Could not change permissions of .ssh directory (possibly read-only mount)"

# Execute the command passed to the container
exec "$@"
