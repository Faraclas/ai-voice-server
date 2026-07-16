#!/bin/bash

# Requires root to run
if [ "$EUID" -ne 0 ]; then
  echo "Please run with sudo"
  exit 1
fi

echo "Removing client binaries..."
rm -f /usr/bin/ai-voice-client
rm -f /usr/bin/ai-voice-interceptor

echo "Removing client configuration..."
rm -rf /etc/ai-voice-server

echo "Removing udevmon configuration..."
rm -f /etc/interception/udevmon.yaml

echo "Stopping ydotool user service..."
su -c "XDG_RUNTIME_DIR=/run/user/\$(id -u) systemctl --user stop ydotool" $SUDO_USER
su -c "XDG_RUNTIME_DIR=/run/user/\$(id -u) systemctl --user disable ydotool" $SUDO_USER

echo "Restarting udevmon service..."
if command -v systemctl >/dev/null 2>&1; then
    systemctl restart udevmon
else
    rc-service udevmon restart
fi

echo "Done! Client test environment uninstalled cleanly."
