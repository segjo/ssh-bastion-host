# ============= Build Stage: Rust Dashboard =============
FROM rust:slim AS dashboard-builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dashboard source
COPY web-dashboard/Cargo.toml web-dashboard/Cargo.lock* ./
COPY web-dashboard/src ./src

# Build the dashboard
RUN cargo build --release

# ============= Runtime Stage =============
FROM ubuntu:24.04

# Set environment to avoid interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install OpenSSH server, procps, and runtime dependencies
RUN apt-get update && \
    apt-get install -y openssh-server procps ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    # Create necessary directories
    mkdir -p /var/run/sshd /root/.ssh && \
    # Create a user for SSH connections
    useradd -m -s /bin/bash bastion && \
    mkdir -p /home/bastion/.ssh && \
    chown -R bastion:bastion /home/bastion/.ssh && \
    chmod 700 /home/bastion/.ssh

# Copy dashboard binary from builder
COPY --from=dashboard-builder /build/target/release/ssh-bastion-dashboard /usr/local/bin/ssh-bastion-dashboard
RUN chmod +x /usr/local/bin/ssh-bastion-dashboard

# Configure SSH for security
RUN sed -i 's/#*PermitRootLogin.*/PermitRootLogin no/' /etc/ssh/sshd_config && \
    sed -i 's/#*PasswordAuthentication.*/PasswordAuthentication no/' /etc/ssh/sshd_config && \
    sed -i 's/#*PubkeyAuthentication.*/PubkeyAuthentication yes/' /etc/ssh/sshd_config && \
    sed -i 's/#*PermitEmptyPasswords.*/PermitEmptyPasswords no/' /etc/ssh/sshd_config && \
    sed -i 's/#*ChallengeResponseAuthentication.*/ChallengeResponseAuthentication no/' /etc/ssh/sshd_config && \
    sed -i 's/#*KbdInteractiveAuthentication.*/KbdInteractiveAuthentication no/' /etc/ssh/sshd_config && \
    echo "GatewayPorts yes" >> /etc/ssh/sshd_config && \
    echo "AllowTcpForwarding yes" >> /etc/ssh/sshd_config && \
    echo "ClientAliveInterval 30" >> /etc/ssh/sshd_config && \
    echo "ClientAliveCountMax 99999" >> /etc/ssh/sshd_config

# Expose SSH port and dashboard port
EXPOSE 22 8080

# Add entrypoint script
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Volume for authorized_keys
VOLUME ["/home/bastion/.ssh"]

ENTRYPOINT ["/entrypoint.sh"]
