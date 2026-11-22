cask "battery-agent" do
  version "0.1.1"
  sha256 "f7c3cebcc2c7e2fad446c4d0cf4f06aebfb95e37953c28b5e6a4bb58d95ce018"

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
