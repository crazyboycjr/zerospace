#!/bin/sh
# Build ZeroSpace.app into ~/Applications, sign it, and (re)start it as a login agent. Run on the Mac.
#
# Signing: CODESIGN_IDENTITY names the identity to sign with (default "-", ad hoc), optionally
# found in the keychain CODESIGN_KEYCHAIN, unlocked with CODESIGN_KEYCHAIN_PASSWORD. Ad hoc works,
# but macOS then forgets the Accessibility grant at every rebuild; a self-signed Code Signing
# certificate keeps it. The variables can live in signing.env next to this script (ignored by git).
set -eu
cd "$(dirname "$0")"
[ -f signing.env ] && . ./signing.env
APP="$HOME/Applications/ZeroSpace.app"
LABEL=$(/usr/libexec/PlistBuddy -c "Print CFBundleIdentifier" Info.plist)
IDENTITY="${CODESIGN_IDENTITY:--}"

PATH="$HOME/.cargo/bin:$PATH" cargo build --release
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp Info.plist "$APP/Contents/Info.plist"
cp assets/icon/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
cp target/release/zerospace "$APP/Contents/MacOS/ZeroSpace"
if [ -n "${CODESIGN_KEYCHAIN:-}" ]; then
    [ -n "${CODESIGN_KEYCHAIN_PASSWORD:-}" ] && security unlock-keychain -p "$CODESIGN_KEYCHAIN_PASSWORD" "$CODESIGN_KEYCHAIN"
    codesign --force --keychain "$CODESIGN_KEYCHAIN" --sign "$IDENTITY" "$APP"
else
    codesign --force --sign "$IDENTITY" "$APP"
fi
[ "$IDENTITY" = "-" ] && echo "signed ad hoc: macOS will ask for Accessibility again after each rebuild" >&2

PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
cat > "$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>$LABEL</string>
  <key>ProgramArguments</key><array><string>$APP/Contents/MacOS/ZeroSpace</string></array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>ProcessType</key><string>Interactive</string>
  <key>StandardErrorPath</key><string>$HOME/Library/Logs/ZeroSpace.log</string>
</dict></plist>
EOF
launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
launchctl bootstrap "gui/$(id -u)" "$PLIST"
echo "ZeroSpace installed at $APP and started"
