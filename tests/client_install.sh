#!/bin/bash

# Requires root to run
if [ "$EUID" -ne 0 ]; then
  echo "Please run with sudo"
  exit 1
fi

cd "$(dirname "$0")/.."

echo "Compiling AI Voice Client binaries..."
# Run cargo as the real user so we don't mess up target/ permissions with root
su -c "cd src/client/daemon && cargo build --release" $SUDO_USER
su -c "cd src/client/daemon && cargo build --release --bin interceptor" $SUDO_USER

echo "Installing binaries to /usr/local/bin..."
cp src/client/daemon/target/release/daemon /usr/local/bin/ai-voice-client
cp src/client/daemon/target/release/interceptor /usr/local/bin/ai-voice-interceptor
chmod +x /usr/local/bin/ai-voice-client
chmod +x /usr/local/bin/ai-voice-interceptor

echo "Creating client configuration file..."
mkdir -p /etc/ai-voice-server
# Recreate the test config file
cat << 'EOF' > /etc/ai-voice-server/client.env
# AI Voice Server Client Configuration
AI_VOICE_SERVER_WS_URL="ws://127.0.0.1:3000/stream"
EOF

echo "Configuring udevmon for global hotkey interception..."
mkdir -p /etc/interception

# 29 = Left Ctrl, 57 = Space
cat << 'EOF' > /etc/interception/udevmon.yaml
- JOB: "intercept -g $DEVNODE | /usr/local/bin/ai-voice-interceptor 29 57 | uinput -d $DEVNODE"
  DEVICE:
    EVENTS:
      EV_KEY: [KEY_SPACE, KEY_LEFTCTRL]
EOF

echo "Starting ydotool user service for auto-pasting..."
su -c "systemctl --user enable --now ydotool" $SUDO_USER

echo "Restarting udevmon service..."
if command -v systemctl >/dev/null 2>&1; then
    systemctl restart udevmon
    systemctl enable udevmon
else
    rc-service udevmon restart
    rc-update add udevmon default
fi

echo "Done! The client is fully installed for testing."
echo "You can run the client daemon by executing: ai-voice-client"
echo "Hold Left_Ctrl and press Space to test the hotkey."
