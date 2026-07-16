#!/bin/bash
# /usr/bin/ai-voice-server
# Hardware-aware launcher for the AI Voice Server

# 1. Check for NVIDIA eGPU
if command -v nvidia-smi &> /dev/null && nvidia-smi &> /dev/null; then
    echo "NVIDIA GPU detected! Launching CUDA-optimized server..."
    exec /usr/bin/ai-voice-server-cuda "$@"

# 2. Check for Vulkan (Intel Iris fallback)
elif command -v vulkaninfo &> /dev/null && vulkaninfo &> /dev/null; then
    echo "No NVIDIA GPU found. Launching Vulkan-optimized server..."
    exec /usr/bin/ai-voice-server-vulkan "$@"

# 3. Final Fallback
else
    echo "No GPU APIs detected. Falling back to CPU..."
    # The Vulkan binary can gracefully fall back to CPU internally
    exec /usr/bin/ai-voice-server-vulkan "$@"
fi
