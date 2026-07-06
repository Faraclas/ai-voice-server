#!/bin/bash

# Configuration
# Change this to the exact IP of your Gentoo machine
SERVER_URL="http://192.168.0.205:8000/transcribe"
AUDIO_FILE="/tmp/dictation.wav"
PID_FILE="/tmp/dictation.pid"

# If the PID file exists, we are currently recording. Stop and process!
if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    rm "$PID_FILE"
    
    # Stop recording
    kill $PID 2>/dev/null
    
    # (Optional) Play a stop beep so you know it's processing
    # paplay /usr/share/sounds/freedesktop/stereo/message.oga &
    
    # Notify user that transcription has started
    notify-send "Dictation" "Transcribing on Gentoo server..." -t 2000
    
    # Send to the Gentoo API and parse out the text using jq
    RESPONSE=$(curl -s -X POST -F "file=@$AUDIO_FILE" $SERVER_URL)
    TEXT=$(echo $RESPONSE | jq -r '.text')
    
    if [ "$TEXT" != "null" ] && [ -n "$TEXT" ]; then
        # Copy to Wayland clipboard
        echo -n "$TEXT" | wl-copy
        
        # Show GNOME notification with the transcribed text
        notify-send "Dictation Complete" "$TEXT"
        
        # If you want auto-pasting, uncomment the line below (requires ydotool)
        # ydotool key 29:1 47:1 47:0 29:0  # Ctrl Down, V Down, V Up, Ctrl Up
    else
        notify-send "Dictation Failed" "Could not transcribe audio. Is the server running?"
    fi

# If no PID file, start recording
else
    # (Optional) Play a start beep
    # paplay /usr/share/sounds/freedesktop/stereo/bell.oga &
    
    # Record audio in the background (16kHz, mono, 16-bit is perfect for Whisper)
    arecord -f S16_LE -c1 -r 16000 -q $AUDIO_FILE &
    
    # Save the Process ID so we can kill it on the next press
    echo $! > "$PID_FILE"
    notify-send "Dictation" "Listening... (Press shortcut again to stop)" -t 2000
fi
