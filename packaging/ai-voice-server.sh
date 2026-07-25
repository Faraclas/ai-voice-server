#!/bin/bash
# /usr/bin/ai-voice-server
# Hardware-aware launcher for the AI Voice Server

echo "Starting AI Voice Server hardware detection..."

# 1. Check for NVIDIA CUDA server binary
if [ -x /usr/bin/ai-voice-server-cuda ]; then
    echo "NVIDIA CUDA server binary found. Waiting for GPU driver & nvidia-uvm subsystem to stabilize..."
    
    # Retry loop: Poll nvidia-smi and /dev/nvidia-uvm for up to 120 seconds
    # This solves early boot race conditions where the NVIDIA open kernel driver takes time to reset.
    for i in {1..120}; do
        if command -v nvidia-smi &> /dev/null && nvidia-smi &> /dev/null && [ -e /dev/nvidia-uvm ]; then
            echo "NVIDIA GPU & CUDA subsystem ready after ${i} seconds! Launching CUDA-optimized server..."
            exec /usr/bin/ai-voice-server-cuda "$@"
        fi
        sleep 1
    done
    
    echo "Warning: NVIDIA GPU did not respond within 120 seconds."
fi

# 2. Check for ROCm (AMD GPU) server binary
if [ -x /usr/bin/ai-voice-server-rocm ]; then
    echo "AMD ROCm server binary found. Waiting for ROCm subsystem..."
    for i in {1..30}; do
        if command -v rocm-smi &> /dev/null && rocm-smi &> /dev/null; then
            echo "AMD GPU ready! Launching ROCm-optimized server..."
            exec /usr/bin/ai-voice-server-rocm "$@"
        fi
        sleep 1
    done
    echo "Warning: AMD GPU did not respond within 30 seconds."
fi

# 3. Check for Vulkan server binary
if [ -x /usr/bin/ai-voice-server-vulkan ]; then
    echo "Checking Vulkan subsystem..."
    if command -v vulkaninfo &> /dev/null && vulkaninfo &> /dev/null; then
        echo "Vulkan subsystem ready! Launching Vulkan-optimized server..."
        exec /usr/bin/ai-voice-server-vulkan "$@"
    fi
fi

# 4. Fallback Execution
if [ -x /usr/bin/ai-voice-server-cpu ]; then
    echo "Launching CPU-dedicated server..."
    exec /usr/bin/ai-voice-server-cpu "$@"
elif [ -x /usr/bin/ai-voice-server-cuda ]; then
    echo "Launching CUDA binary as CPU fallback..."
    exec /usr/bin/ai-voice-server-cuda "$@"
elif [ -x /usr/bin/ai-voice-server-rocm ]; then
    echo "Launching ROCm binary as CPU fallback..."
    exec /usr/bin/ai-voice-server-rocm "$@"
elif [ -x /usr/bin/ai-voice-server-vulkan ]; then
    echo "Launching Vulkan binary as CPU fallback..."
    exec /usr/bin/ai-voice-server-vulkan "$@"
else
    echo "Error: No suitable server binary found!"
    exit 1
fi
