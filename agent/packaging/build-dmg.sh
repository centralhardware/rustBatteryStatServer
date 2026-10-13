#!/bin/sh
# Builds a universal binary, wraps it in a .pkg and puts that in a .dmg.
# Run on macOS from the agent/ directory. Output: target/battery-agent-<version>.dmg
set -eu
cd "$(dirname "$0")/.."
VERSION=${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}
ID=me.centralhardware.battery-agent

for t in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --release --target "$t"
done

STAGE=target/pkgroot
rm -rf "$STAGE" target/dmg
mkdir -p "$STAGE/usr/local/libexec" "$STAGE/Library/LaunchDaemons" target/dmg
lipo -create -output "$STAGE/usr/local/libexec/battery-agent" \
    target/aarch64-apple-darwin/release/battery-agent \
    target/x86_64-apple-darwin/release/battery-agent
cp packaging/$ID.plist "$STAGE/Library/LaunchDaemons/"
chmod 755 packaging/scripts/* packaging/uninstall.command

pkgbuild --root "$STAGE" --scripts packaging/scripts \
    --identifier "$ID" --version "$VERSION" --install-location / \
    "target/dmg/Install battery-agent.pkg"
cp packaging/uninstall.command "target/dmg/Uninstall battery-agent.command"

hdiutil create -volname "battery-agent $VERSION" -srcfolder target/dmg \
    -ov -format UDZO "target/battery-agent-$VERSION.dmg"
