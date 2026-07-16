#!/bin/bash

# Requires root to run
if [ "$EUID" -ne 0 ]; then
  echo "Please run with sudo"
  exit 1
fi

echo "Removing interceptor..."
rm -f /usr/local/bin/ai-voice-interceptor

echo "Removing udevmon configuration..."
rm -f /etc/interception/udevmon.yaml

echo "Restarting udevmon service..."
if command -v systemctl >/dev/null 2>&1; then
    systemctl restart udevmon
else
    rc-service udevmon restart
fi

echo "Done! Test environment uninstalled cleanly."
