# battery-agent

A macOS LaunchDaemon that reads the battery directly from IOKit (`AppleSmartBattery`)
and posts a sample to `POST /api/battery/health` every 30 seconds, plus one immediately
whenever the power source changes (adapter plugged in or pulled, charge level change). It stays resident
(launchd `KeepAlive`) instead of being started again for every sample. It replaces the old
`scripts/report_battery_health.sh` and sends the same payload.

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
`https://bs.example.com/api/battery/health`. Until it is set, the agent only logs that it is missing. To set it or change it
(it takes effect on the next cycle, with no restart needed):

    sudo defaults write /Library/Preferences/me.centralhardware.battery-agent ServerURL https://host/api/battery/health

`BATTERY_AGENT_URL` overrides the server URL for a manual run.

## Debug

    /usr/local/libexec/battery-agent --dry-run           # print the payload, send nothing
    tail -f /Library/Logs/battery-agent.log
    sudo launchctl kickstart -k system/me.centralhardware.battery-agent   # restart

## Build

    cd agent && ./packaging/build-dmg.sh   # on macOS; writes target/battery-agent-<version>.dmg
