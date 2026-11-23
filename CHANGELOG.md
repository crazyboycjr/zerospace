# Changelog

## 1.1.0 (2026-11-23)

cmd+shift+1 to cmd+shift+5 no longer send the focused window to a desktop.
macOS uses cmd+shift+3, 4 and 5 for screenshots and handles its own shortcuts
before ZeroSpace's, so three of those five keys could never send a window
unless the screenshot shortcuts were turned off. Now the screenshot shortcuts
keep working with nothing to turn off. cmd+shift+← and → still carry the focused window one
desktop over and follow it, and the socket's `send N` still sends a window to
a desktop and stays, for scripts.

Update with nix-darwin as before (`nix flake update zerospace`, then
`darwin-rebuild switch`), or rebuild with `sh build.sh`.

## 1.0.0 (2026-10-31)

The first public release: ZeroSpace switches macOS desktops with no animation
at all. Install it on macOS 27, with Rust and the Xcode Command Line Tools:

```sh
git clone https://github.com/crazyboycjr/zerospace
cd zerospace
sh build.sh
```

Allow Accessibility when macOS asks. To keep that permission across builds, sign
with a certificate of your own through `CODESIGN_IDENTITY`; the README's
[Signing](README.md#signing) section shows how.

ZeroSpace switches desktops through the window server's private bridged
operations, which macOS 27 accepts from an app with Accessibility access while
SIP stays on.

**Switching.** cmd+1 to cmd+5 go straight to a desktop. Desktops are numbered by
their place in the Mission Control bar of the display under the pointer,
full-screen apps included.

**Sending and carrying.** cmd+shift+1 to cmd+shift+5 send the focused window to
a desktop and stay; a full-screen app's slot is refused. cmd+shift+← and → carry
the focused window one desktop over and follow it, stepping over full-screen
apps; from a full-screen app they step to the nearest desktop that way without
moving a window.

**Focus.** Once a switch has landed, the frontmost window on screen there gets
the keyboard, and Finder does on an empty desktop. A window you sent somewhere
comes to the front when you next switch there with ZeroSpace. Hidden and
minimized windows are never picked, nor an app's invisible helper windows.

**The indicator.** Each switch flashes the desktop's number in the middle of the
screen for half a second; a send shows the way the window went, ←N or →N.

**Scripting.** A Unix socket at `~/.cache/zerospace.sock` takes `go N`, `send
N`, `carry left|right`, `move WINDOW N`, `peek` and `state`.

**Installing.** `build.sh` installs `~/Applications/ZeroSpace.app` and the login
agent `xyz.crazyboycjr.zerospace`. It signs ad hoc by default, or with the
identity in `CODESIGN_IDENTITY`, optionally from the keychain
`CODESIGN_KEYCHAIN` unlocked with `CODESIGN_KEYCHAIN_PASSWORD`; these can live
in `signing.env` beside it.

Tested on macOS 27.0.1 on Apple Silicon, with one display.
