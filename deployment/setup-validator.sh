#!/bin/bash
set -e

echo "================================"
echo "Pilier Validator Setup Script"
echo "================================"
echo ""

# Check if docker is installed
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is not installed"
    echo "Install: https://docs.docker.com/engine/install/"
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "Error: Docker Compose is not installed"
    exit 1
fi

# Check if .env already exists
if [ -f .env ]; then
    echo "Warning: .env file already exists"
    read -p "Overwrite? (y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Setup cancelled"
        exit 0
    fi
fi

# Collect information
read -p "Validator name: " VALIDATOR_NAME
read -p "P2P port [30333]: " P2P_PORT
P2P_PORT=${P2P_PORT:-30333}

read -p "RPC port [9944]: " RPC_PORT
RPC_PORT=${RPC_PORT:-9944}

echo ""
echo "Bootnode address (get from Pilier team):"
read -p "Bootnode: " BOOTNODE_ADDR

# Create .env file
cat > .env << EOF
VALIDATOR_NAME=${VALIDATOR_NAME}
VERSION=v0.1.0
P2P_PORT=${P2P_PORT}
RPC_PORT=${RPC_PORT}
BOOTNODE_ADDR=${BOOTNODE_ADDR}
EOF

echo ""
echo "✅ Configuration saved to .env"
echo ""
echo "Next steps:"
echo "1. Review .env file"
echo "2. Run: docker-compose -f docker-compose.template.yml up -d"
echo "3. Check logs: docker-compose -f docker-compose.template.yml logs -f"
echo ""
echo "Generate validator keys:"
echo "docker-compose -f docker-compose.template.yml exec pilier-validator pilier-node key generate"
