#!/bin/bash

# Configuration Setup
CONFIG_DIR="$HOME/.config/ai-voice"
CONFIG_FILE="$CONFIG_DIR/client.conf"

# Default fallback values
SERVER_HOST="192.168.0.205"
SERVER_PORT="8000"

# Create a config file if it doesn't exist so the user can easily change it
if [ ! -f "$CONFIG_FILE" ]; then
    mkdir -p "$CONFIG_DIR"
    echo "SERVER_HOST=\"$SERVER_HOST\"" > "$CONFIG_FILE"
    echo "SERVER_PORT=\"$SERVER_PORT\"" >> "$CONFIG_FILE"
else
    # Load configuration from file
    source "$CONFIG_FILE"
fi

SERVER_URL="http://${SERVER_HOST}:${SERVER_PORT}"
AUDIO_FILE="/tmp/dictation.wav"
PID_FILE="/tmp/dictation.pid"

# --- DIAGNOSTICS & HELP ---
if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo "Voice-to-Text Dictation Client"
    echo "=============================="
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Description:"
    echo "  A push-to-talk dictation client designed to be bound to a system hotkey."
    echo "  Run once to start recording. Run again to stop, transcribe, and copy."
    echo ""
    echo "Options:"
    echo "  -h, --help    Show this help message and exit"
    echo "  --test        Ping the AI server to check if it's online and display the loaded model"
    echo ""
    echo "Configuration:"
    echo "  Settings are loaded from: $CONFIG_FILE"
    echo "  (You can edit this file to change the Server IP and Port)."
    echo ""
    exit 0
fi

# Handle --test flag
if [ "$1" == "--test" ]; then
    echo "Testing connection to server at $SERVER_URL..."
    # Ping the /status endpoint with a strict 2-second timeout
    STATUS=$(curl -s --max-time 2 "${SERVER_URL}/status")
    
    if [ $? -eq 0 ]; then
        MODEL=$(echo $STATUS | jq -r '.current_model')
        MSG="Connected! Server is running model: $MODEL"
        echo "$MSG"
        notify-send "Dictation Server OK" "$MSG"
        exit 0
    else
        MSG="Failed to connect to server at $SERVER_URL"
        echo "$MSG"
        notify-send "Dictation Server Offline" "$MSG"
        exit 1
    fi
fi
# ---------------------------------------

# If the PID file exists, we are currently recording. Stop and process!
if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    rm "$PID_FILE"
    
    # Stop recording
    kill $PID 2>/dev/null
    
    notify-send "Dictation" "Transcribing on Gentoo server..." -t 2000
    
    # Send to the Gentoo API
    # We add a timeout just in case the server vanished while we were recording
    RESPONSE=$(curl -s --max-time 15 -X POST -F "file=@$AUDIO_FILE" "${SERVER_URL}/transcribe")
    CURL_EXIT_CODE=$?
    
    # Error Handling: Did curl fail to connect?
    if [ $CURL_EXIT_CODE -ne 0 ]; then
        notify-send "Dictation Error" "Could not connect to server at $SERVER_HOST"
        exit 1
    fi
    
    TEXT=$(echo $RESPONSE | jq -r '.text')
    
    if [ "$TEXT" != "null" ] && [ -n "$TEXT" ]; then
        # Copy to Wayland clipboard
        echo -n "$TEXT" | wl-copy
        
        # Show GNOME notification with the transcribed text
        notify-send "Dictation Complete" "$TEXT"
        
        # If you want auto-pasting, uncomment the line below (requires ydotool)
        # ydotool key 29:1 47:1 47:0 29:0  # Ctrl Down, V Down, V Up, Ctrl Up
    else
        notify-send "Dictation Failed" "Server responded, but transcription was empty."
    fi

# If no PID file, start recording
else
    # Record audio in the background (16kHz, mono, 16-bit is perfect for Whisper)
    arecord -f S16_LE -c1 -r 16000 -q $AUDIO_FILE 2>/dev/null &
    
    # Save the Process ID so we can kill it on the next press
    echo $! > "$PID_FILE"
    notify-send "Dictation" "Listening... (Press shortcut again to stop)" -t 2000
fi
