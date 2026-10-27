#!/bin/bash
# Build the Hermes agent sandbox Podman/Docker image
# Usage: ./build-sandbox-image.sh [IMAGE_TAG]

set -e

IMAGE_TAG="${1:-hermes-sandbox:latest}"

echo "Building Hermes agent sandbox image: ${IMAGE_TAG}"

# Try podman first, fall back to docker
if command -v podman &> /dev/null; then
    echo "Using podman"
    podman build -t "${IMAGE_TAG}" -f Dockerfile .
elif command -v docker &> /dev/null; then
    echo "Using docker"
    docker build -t "${IMAGE_TAG}" -f Dockerfile .
else
    echo "Error: Neither podman nor docker found"
    exit 1
fi

echo "Image built successfully: ${IMAGE_TAG}"