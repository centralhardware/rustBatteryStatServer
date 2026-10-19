# battery-agent

A macOS LaunchDaemon that reads the battery directly from IOKit (`AppleSmartBattery`)
and posts a sample to `POST /api/battery/health` every 10 seconds (configurable), plus one immediately
whenever the power source changes (adapter plugged in or pulled, charge level change). It stays resident
(launchd `KeepAlive`) instead of being started again for every sample. It replaces the old
`scripts/report_battery_health.sh` and sends the same payload.

Every cycle also samples any iPhone (or iPad) plugged into the Mac by USB, reported under its
UDID as `deviceId`. It reads the phone's AppleSmartBattery through usbmuxd and lockdownd (pure Rust,
nothing extra to install), so the phone must be unlocked once and have trusted this Mac
("Trust This Computer?"). Phones synced over Wi-Fi are skipped. A phone that can't be read
is logged once and doesn't hold up the Mac's own sample.

## Install

Download the `.dmg` from the `agent-v*` releases (or the workflow artifact), open it and
run **Install battery-agent.pkg**. It installs, as root:

- `/usr/local/libexec/battery-agent`: a universal binary (Apple Silicon and Intel)
- `/Library/LaunchDaemons/me.centralhardware.battery-agent.plist`

The daemon runs in the system domain, so it reports for every user and at the login window.
The package is unsigned, so the first time you open it, right-click it and choose Open.

Uninstall with **Uninstall battery-agent.command** from the same dmg.

## Configure

No server is built in. The installer asks for the endpoint URL on first install, e.g.
`https://bs.example.com` (the `/api/battery/health` path is optional). Until it is set, the agent only logs that it is missing. To set it or change it
(it takes effect on the next cycle, with no restart needed):

    sudo defaults write /Library/Preferences/me.centralhardware.battery-agent ServerURL https://host/api/battery/health

If the server has `API_TOKEN` set, store the same token:

    sudo defaults write /Library/Preferences/me.centralhardware.battery-agent AuthToken -string <token>
    sudo chmod 600 /Library/Preferences/me.centralhardware.battery-agent.plist   # the token is a secret

Sampling interval in seconds (default 10, minimum 5):

    sudo defaults write /Library/Preferences/me.centralhardware.battery-agent IntervalSeconds -int 30

`BATTERY_AGENT_URL` overrides the server URL for a manual run.

## Debug

    /usr/local/libexec/battery-agent --dry-run           # print the payload, send nothing
    /usr/local/libexec/battery-agent --dump              # raw AppleSmartBattery properties
    sudo /usr/local/libexec/battery-agent --dump-iphone  # the same, for USB-connected iPhones
    tail -f /Library/Logs/battery-agent.log
    sudo launchctl kickstart -k system/me.centralhardware.battery-agent   # restart

## Build

    cd agent && ./packaging/build-dmg.sh   # on macOS; writes target/battery-agent-<version>.dmg
