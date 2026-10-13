#!/bin/sh
# Removes battery-agent. Double-click, or run with sudo.
[ "$(id -u)" -eq 0 ] || exec sudo "$0" "$@"
/bin/launchctl bootout system/me.centralhardware.battery-agent 2>/dev/null
rm -f /Library/LaunchDaemons/me.centralhardware.battery-agent.plist \
      /usr/local/libexec/battery-agent \
      /Library/Logs/battery-agent.log
/usr/sbin/pkgutil --forget me.centralhardware.battery-agent 2>/dev/null
echo "battery-agent removed."
