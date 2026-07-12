#!/bin/bash
# Add the local CUDA math libraries to the system path
export LD_LIBRARY_PATH="/home/elias/code/ai-voice-server/.venv/lib/python3.14/site-packages/nvidia/cublas/lib:/home/elias/code/ai-voice-server/.venv/lib/python3.14/site-packages/nvidia/cudnn/lib:$LD_LIBRARY_PATH"

# Allow passing the model as the first argument (e.g., ./start.sh medium.en). 
# If no argument is provided, default to small.en
export WHISPER_MODEL="${1:-small.en}"

echo "Starting Voice-to-Text API Server on port 8000 (Default Model: $WHISPER_MODEL)..."
uv run uvicorn server:app --host 0.0.0.0 --port 8000
