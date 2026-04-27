#!/bin/bash
# push-orchestrator.sh - Fast local build and push for keryx-orchestrator

set -e

REGISTRY="ghcr.io"
REPO_OWNER="jzacharie"
IMAGE_NAME="keryx-orchestrator"
TAG="latest"

echo "🚀 Starting fast local build for $IMAGE_NAME..."

# 1. Build the binary locally (super fast with incremental cache)
echo "📦 Building binary with Cargo..."
cargo build --release --bin keryx-orchestrator

# 2. Build the Docker image using the fast Dockerfile
echo "🐳 Building Docker image..."
docker build -t $REGISTRY/$REPO_OWNER/$IMAGE_NAME:$TAG -f services/orchestrator/Dockerfile.fast .

# 3. Push to GHCR
echo "📤 Pushing to $REGISTRY..."
docker push $REGISTRY/$REPO_OWNER/$IMAGE_NAME:$TAG

echo "✅ Successfully pushed $IMAGE_NAME:$TAG"
echo "💡 You can now rollout the deployment in Kubernetes:"
echo "   kubectl rollout restart deployment keryx-orchestrator -n keryx"
