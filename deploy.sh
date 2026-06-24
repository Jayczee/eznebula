#!/bin/bash
set -euo pipefail

# ============================================================
# EZNebula Deployment Script
# Target: root@www.jayczee.cn (Debian)
# Working dir: /data
# ============================================================

SERVER="root@www.jayczee.cn"
WORK_DIR="/data"
PROJECT_DIR="${WORK_DIR}/eznebula"
REPO_URL="https://github.com/Jayczee/eznebula.git"
BRANCH="${EZNEBULA_BRANCH:-main}"

# ---- Env vars with defaults ----
# IMPORTANT: EZNEBULA_LIGHTHOUSE_IP must be the public IP/hostname that clients
# can reach. Default: extract hostname from SERVER variable.
SERVER_HOST=$(echo "${SERVER}" | cut -d@ -f2)
export EZNEBULA_PORT="${EZNEBULA_PORT:-8080}"
export EZNEBULA_LIGHTHOUSE_PORT="${EZNEBULA_LIGHTHOUSE_PORT:-4242}"
export EZNEBULA_LIGHTHOUSE_IP="${EZNEBULA_LIGHTHOUSE_IP:-${SERVER_HOST}}"
export EZNEBULA_DATA_DIR="${EZNEBULA_DATA_DIR:-/data/eznebula-data}"
export JAVA_OPTS="${JAVA_OPTS:--Xms128m -Xmx256m}"

echo "=============================================="
echo " EZNebula Deploy"
echo " Server:  ${SERVER}"
echo " Dir:     ${PROJECT_DIR}"
echo " Port:    ${EZNEBULA_PORT} (HTTP)"
echo " LH Port: ${EZNEBULA_LIGHTHOUSE_PORT} (UDP)"
echo " LH IP:   ${EZNEBULA_LIGHTHOUSE_IP}"
echo " Data:    ${EZNEBULA_DATA_DIR}"
echo "=============================================="

# Step 1: Pull / clone code
echo ""
echo "[1/4] Syncing code (branch: ${BRANCH})..."
ssh ${SERVER} "BRANCH=${BRANCH}" bash -s << 'ENDSSH'
WORK_DIR="/data"
REPO_URL="https://github.com/Jayczee/eznebula.git"

if [ -d "${WORK_DIR}/eznebula/.git" ]; then
    echo "  Repository exists, pulling ${BRANCH}..."
    cd ${WORK_DIR}/eznebula
    git fetch origin
    git checkout ${BRANCH}
    git pull origin ${BRANCH}
else
    echo "  Cloning ${BRANCH}..."
    mkdir -p ${WORK_DIR}
    git clone -b ${BRANCH} ${REPO_URL} ${WORK_DIR}/eznebula
fi
echo "  Git done."
ENDSSH

# Step 2: Build the app (Maven + Docker)
echo ""
echo "[2/4] Building on server..."
ssh ${SERVER} bash -s << ENDSSH
set -e
cd /data/eznebula

# Build Spring Boot JAR
echo "  Building Spring Boot app..."
cd eznebula-backend
if ! command -v mvn &> /dev/null; then
    echo "  Maven not found, installing..."
    apt-get update -qq && apt-get install -y -qq maven
fi
mvn clean package -DskipTests -q
cd ..

echo "  Build complete."
ENDSSH

# Step 3: Docker build & restart
echo ""
echo "[3/4] Docker compose up..."
ssh ${SERVER} bash -s << ENDSSH
set -e
cd /data/eznebula

echo "  Stopping old container..."
docker compose down 2>/dev/null || true

echo "  Building Docker image..."
docker compose build --no-cache

echo "  Starting services..."
docker compose up -d

echo "  Waiting for healthy..."
sleep 5
docker compose ps
ENDSSH

# Step 4: Verify
echo ""
echo "[4/4] Verifying..."
ssh ${SERVER} bash -s << ENDSSH
if docker compose -f /data/eznebula/docker-compose.yml ps | grep -q "Up"; then
    echo "  Container is running."
    docker compose -f /data/eznebula/docker-compose.yml logs --tail=20
else
    echo "  ERROR: Container not running!"
    docker compose -f /data/eznebula/docker-compose.yml logs
    exit 1
fi
ENDSSH

echo ""
echo "=============================================="
echo " Deployment complete!"
echo " API:        http://\$(echo ${SERVER} | cut -d@ -f2):${EZNEBULA_PORT}/api/v1/health"
echo " Lighthouse: udp://\$(echo ${SERVER} | cut -d@ -f2):${EZNEBULA_LIGHTHOUSE_PORT}"
echo "=============================================="
