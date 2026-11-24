# Building ZeroSpace

This page is for people who want to build ZeroSpace from source, or sign it
with a certificate of their own. To install ZeroSpace and use it, the
[README](../README.md) is the place to start: it installs ZeroSpace through
nix-darwin.

## Build it on your Mac

Building takes Rust and the Xcode Command Line Tools (`xcode-select
--install`).

```sh
git clone https://github.com/crazyboycjr/zerospace
cd zerospace
sh build.sh
```

`build.sh` builds with cargo, installs `~/Applications/ZeroSpace.app`, signs it,
and starts it as the login agent `xyz.crazyboycjr.zerospace`, which runs it at
every login, keeps it running, and logs to `~/Library/Logs/ZeroSpace.log`. The
first start asks for Accessibility access; allow it in System Settings, Privacy
& Security, Accessibility.

The README's advice on desktops applies here too, with one difference: nothing
turns off macOS's own Switch to Desktop shortcuts for you. Turn them off in
System Settings, Keyboard, Keyboard Shortcuts, Mission Control; macOS handles
them before any app, so if they use cmd+1 to cmd+5, ZeroSpace never sees those
keys and the desktops slide as before.

If you installed ZeroSpace through nix-darwin before, take the module out and
switch first, so that only one copy runs.

### Stop and remove

```sh
launchctl bootout gui/$(id -u)/xyz.crazyboycjr.zerospace
rm ~/Library/LaunchAgents/xyz.crazyboycjr.zerospace.plist
rm -r ~/Applications/ZeroSpace.app
```

Then take ZeroSpace out of System Settings, Privacy & Security, Accessibility.

## Build it with Nix

The repository is a Nix flake for Apple Silicon. To try a build without
installing anything:

```sh
nix build github:crazyboycjr/zerospace
```

The app is then `result/Applications/ZeroSpace.app`. It is signed ad hoc with
`rcodesign`, since a nix build cannot reach Apple's `codesign`, and it needs
`apple-sdk_26` from nixpkgs: an app linked against an older SDK gets macOS's
older window look. The flake pins a nixpkgs that has it.

## Signing

`build.sh` signs ad hoc unless told otherwise. That works, but an ad hoc
signature changes with every build, and macOS then takes ZeroSpace for a new app
and asks for Accessibility again. A self-signed certificate keeps the grant
across builds:

1. Open Keychain Access from Spotlight; since macOS 15 it is no longer in
   Utilities. Choose Keychain Access, Certificate Assistant, Create a
   Certificate.
2. Name it, for example Local Code Signing, with Identity Type Self-Signed Root
   and Certificate Type Code Signing.
3. Build with it:

```sh
CODESIGN_IDENTITY="Local Code Signing" sh build.sh
```

The first build with it asks whether codesign may use the key; choose Always
Allow. If ZeroSpace was built ad hoc before, its old entry in System Settings,
Privacy & Security, Accessibility stays ticked but no longer counts: remove it
with the minus button and allow ZeroSpace again.

To keep the setting, put the line `CODESIGN_IDENTITY="Local Code Signing"` in
`signing.env` beside `build.sh`, which git ignores. Where the login keychain is
locked, as over ssh, keep the certificate in a keychain of its own, give the
keychain's path in `CODESIGN_KEYCHAIN` and its password in
`CODESIGN_KEYCHAIN_PASSWORD`.

## The repository

| Path | What it holds |
| :--- | :--- |
| `src/main.rs` | The app: the keys, switching and carrying, focus, the indicator and the socket. |
| `src/sys.rs` | The macOS interfaces ZeroSpace uses, declared by hand: CoreFoundation, CoreGraphics, SkyLight, Accessibility, Carbon hot keys, libdispatch and the Objective-C runtime. |
| `build.rs` | Puts the private frameworks on the linker's search path, for SkyLight. |
| `Info.plist`, `assets/icon/` | The app bundle's property list and icon. |
| `flake.nix`, `package.nix` | The flake, its package and the nix-darwin module. |
| `build.sh` | The build and install described above. |
| `site/` | The homepage, published to GitHub Pages from `main`. |
