#!/bin/sh
# Build Hop.app into ~/Applications, sign it, and (re)start it as a login agent. Run on the Mac.
set -eu
cd "$(dirname "$0")"
APP="$HOME/Applications/Hop.app"
KEYCHAIN="$HOME/Library/Keychains/signing.keychain-db"
# A fixed signing identity keeps the Accessibility grant across rebuilds; ad-hoc signing loses it
IDENTITY="Local Code Signing"

PATH="$HOME/.cargo/bin:$PATH" cargo build --release
mkdir -p "$APP/Contents/MacOS"
cp Info.plist "$APP/Contents/Info.plist"
cp target/release/hop "$APP/Contents/MacOS/Hop"
if [ -f "$KEYCHAIN" ]; then
    security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
    codesign --force --keychain "$KEYCHAIN" --sign "$IDENTITY" "$APP"
else
    echo "no $KEYCHAIN, signing ad hoc (the Accessibility grant will not survive a rebuild)" >&2
    codesign --force --sign - "$APP"
fi

PLIST="$HOME/Library/LaunchAgents/xyz.crazyboycjr.hop.plist"
cat > "$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>xyz.crazyboycjr.hop</string>
  <key>ProgramArguments</key><array><string>$APP/Contents/MacOS/Hop</string></array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>ProcessType</key><string>Interactive</string>
  <key>StandardErrorPath</key><string>$HOME/Library/Logs/Hop.log</string>
</dict></plist>
EOF
launchctl bootout "gui/$(id -u)/xyz.crazyboycjr.hop" 2>/dev/null || true
launchctl bootstrap "gui/$(id -u)" "$PLIST"
echo "Hop installed at $APP and started"
