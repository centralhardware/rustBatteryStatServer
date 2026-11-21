cask "battery-agent" do
  version "0.1.0"
  sha256 "5a21c7f97ffed0c0a03b7be0809797145e596352eb180fea241037ae16eab256"

  url "https://github.com/centralhardware/rustBatteryStatServer/releases/download/agent-v#{version}/battery-agent-#{version}.dmg"
  name "battery-agent"
  desc "Reports battery stats to BatteryStatServer"
  homepage "https://github.com/centralhardware/rustBatteryStatServer"

  pkg "Install battery-agent.pkg"

  # Mirrors packaging/uninstall.command.
  uninstall launchctl: "me.centralhardware.battery-agent",
            pkgutil:   "me.centralhardware.battery-agent",
            delete:    "/Library/Logs/battery-agent.log"

  zap trash: "/Library/Preferences/me.centralhardware.battery-agent.plist"
end
