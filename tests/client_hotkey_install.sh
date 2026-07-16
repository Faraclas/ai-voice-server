#!/bin/bash

# Requires root to run
if [ "$EUID" -ne 0 ]; then
  echo "Please run with sudo"
  exit 1
fi

cd "$(dirname "$0")/.."

echo "Building interceptor..."
# Run cargo as the real user so we don't mess up target/ permissions with root
su -c "cd src/client/daemon && cargo build --release --bin interceptor" $SUDO_USER

echo "Installing interceptor to /usr/local/bin..."
cp src/client/daemon/target/release/interceptor /usr/local/bin/ai-voice-interceptor
chmod +x /usr/local/bin/ai-voice-interceptor

echo "Configuring udevmon..."
mkdir -p /etc/interception

# 29 = Left Ctrl, 57 = Space
cat << 'EOF' > /etc/interception/udevmon.yaml
- JOB: "intercept -g $DEVNODE | /usr/local/bin/ai-voice-interceptor 29 57 | uinput -d $DEVNODE"
  DEVICE:
    EVENTS:
      EV_KEY: [KEY_SPACE, KEY_LEFTCTRL]
EOF

echo "Restarting udevmon service..."
if command -v systemctl >/dev/null 2>&1; then
    systemctl restart udevmon
    systemctl enable udevmon
else
    rc-service udevmon restart
    rc-update add udevmon default
fi

echo "Done! The interception-tools pipeline is installed and active."
echo "Hold Left_Ctrl and press Space to test."
