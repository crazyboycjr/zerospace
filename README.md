# ZeroSpace

Remove the motion from macOS desktop switching. A small Rust homage to TotalSpaces: cmd+N jumps straight to a desktop, instantly, with no slide.

| Keys | Action |
|---|---|
| cmd+1 … cmd+5 | jump to desktop N |
| cmd+shift+1 … cmd+shift+5 | send the focused window to desktop N and stay |
| cmd+shift+left / right | take the focused window one desktop over and follow it |

Every jump flashes the desktop number in the middle of the screen for half a second.

## How

macOS has no setting to turn the space-switch animation off, and the usual tricks (yabai, a fast synthetic swipe) either need SIP disabled or just play the slide quickly. ZeroSpace instead drives the window server directly through SkyLight's bridged operations (`SLSBridgedShowSpacesOperation`, `…HideSpacesOperation`, `…ManagedDisplaySetCurrentSpaceOperation`, `…MoveWindowsToManagedSpaceOperation`, each sent with `-performWithWMBridgeDelegate`). macOS 27 accepts them from any process that has Accessibility access, with SIP left on, and the switch simply happens: there is nothing to animate. Hot keys are Carbon hot keys, so ZeroSpace installs no event tap and never sits in the path of your mouse or typing.

Desktops are numbered by their slot in the Mission Control bar on the display under the pointer, full-screen apps included. ZeroSpace cannot create desktops; add them in Mission Control, and keep "Automatically rearrange Spaces based on most recent use" off so the numbers stay put.

## Build and install (on the Mac)

```sh
sh build.sh
```

This builds with cargo, installs `~/Applications/ZeroSpace.app`, signs it, and starts it as the login agent `xyz.crazyboycjr.zerospace` (log in `~/Library/Logs/ZeroSpace.log`). The first start asks for Accessibility access once.

Signing uses the self-signed identity "Local Code Signing" in `~/Library/Keychains/signing.keychain-db`, so the Accessibility grant survives rebuilds (an ad-hoc signature changes every build and loses it). The keychain has its own password so builds over ssh can sign without unlocking the login keychain.

## Scripting

A Unix socket at `~/.cache/zerospace.sock` takes the same actions as text:

```sh
printf 'go 3'      | nc -U ~/.cache/zerospace.sock
printf 'send 2'    | nc -U ~/.cache/zerospace.sock
printf 'carry left'| nc -U ~/.cache/zerospace.sock
printf 'move 1568 4' | nc -U ~/.cache/zerospace.sock   # a given window to desktop 4
printf 'state'     | nc -U ~/.cache/zerospace.sock
```
