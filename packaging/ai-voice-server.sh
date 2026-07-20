#!/bin/bash
# /usr/bin/ai-voice-server
# Hardware-aware launcher for the AI Voice Server

# 1. Check for NVIDIA eGPU
if command -v nvidia-smi &> /dev/null && nvidia-smi &> /dev/null && [ -x /usr/bin/ai-voice-server-cuda ]; then
    echo "NVIDIA GPU detected! Waiting for CUDA compute subsystem (nvidia-uvm)..."
    
    # Deterministic wait for CUDA compute (nvidia-uvm) kernel module to become ready
    # We timeout after 10 seconds (50 * 0.2s) to prevent hanging
    for i in {1..50}; do
        if [ -e /dev/nvidia-uvm ]; then
            break
        fi
        sleep 0.2
    done

    # Even after device nodes are created, the eGPU needs time to be ready for compute over Thunderbolt.
    # If the system just booted (uptime < 120s), we must give the Nvidia driver 5 seconds to warm up the PCI bus.
    if [ $(awk '{print int($1)}' /proc/uptime) -lt 120 ]; then
        echo "Early boot detected. Giving the eGPU hardware 5 seconds to warm up compute..."
        sleep 5
    fi

    echo "Launching CUDA-optimized server..."
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
    echo "No GPU APIs detected. Falling back to CPU mode..."
    if [ -x /usr/bin/ai-voice-server-cpu ]; then
        exec /usr/bin/ai-voice-server-cpu "$@"
    elif [ -x /usr/bin/ai-voice-server-cuda ]; then
        exec /usr/bin/ai-voice-server-cuda "$@"
    elif [ -x /usr/bin/ai-voice-server-rocm ]; then
        exec /usr/bin/ai-voice-server-rocm "$@"
    elif [ -x /usr/bin/ai-voice-server-vulkan ]; then
        exec /usr/bin/ai-voice-server-vulkan "$@"
    else
        echo "Error: No suitable server binary found!"
        exit 1
    fi
fi

