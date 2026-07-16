#!/bin/bash
# /usr/bin/ai-voice-server
# Hardware-aware launcher for the AI Voice Server

# 1. Check for NVIDIA eGPU
if command -v nvidia-smi &> /dev/null && nvidia-smi &> /dev/null && [ -x /usr/bin/ai-voice-server-cuda ]; then
    echo "NVIDIA GPU detected! Launching CUDA-optimized server..."
    exec /usr/bin/ai-voice-server-cuda "$@"

# 2. Check for ROCm (AMD GPU)
elif command -v rocm-smi &> /dev/null && rocm-smi &> /dev/null && [ -x /usr/bin/ai-voice-server-rocm ]; then
    echo "AMD GPU detected! Launching ROCm-optimized server..."
    exec /usr/bin/ai-voice-server-rocm "$@"

# 3. Check for Vulkan (Intel Iris fallback / Universal)
elif command -v vulkaninfo &> /dev/null && vulkaninfo &> /dev/null && [ -x /usr/bin/ai-voice-server-vulkan ]; then
    echo "Launching Vulkan-optimized server..."
    exec /usr/bin/ai-voice-server-vulkan "$@"

# 4. Final Fallback (CPU)
else
    echo "No GPU APIs detected or matching binaries installed. Falling back to CPU..."
    if [ -x /usr/bin/ai-voice-server-cpu ]; then
        exec /usr/bin/ai-voice-server-cpu "$@"
    elif [ -x /usr/bin/ai-voice-server-vulkan ]; then
        exec /usr/bin/ai-voice-server-vulkan "$@"
    else
        echo "Error: No suitable server binary found!"
        exit 1
    fi
fi

