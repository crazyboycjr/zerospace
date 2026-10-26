# Hop

Instant desktop switching for macOS, a small TotalSpaces replacement in Rust.

| Keys | Action |
|---|---|
| cmd+1 … cmd+5 | go to desktop N, with no animation |
| cmd+shift+1 … cmd+shift+5 | send the focused window to desktop N and stay where you are |
| cmd+shift+left / right | take the focused window one desktop over and follow it |

Hop switches desktops and moves windows through SkyLight's bridged window-management operations (`SLSBridgedShowSpacesOperation`, `…HideSpacesOperation`, `…ManagedDisplaySetCurrentSpaceOperation`, `…MoveWindowsToManagedSpaceOperation`, each sent with `-performWithWMBridgeDelegate`). macOS 27 accepts them from any process that has Accessibility access, with SIP left on, and the switch lands at once: there is no swipe animation to skip. InstantSpaceSwitcher's PR #110 found the approach. Hot keys are Carbon hot keys, so Hop installs no event tap and cannot stall input.

Desktops are counted per display (the one under the pointer), full-screen apps excluded. Hop cannot create desktops; add them in Mission Control. Keep "Automatically rearrange Spaces based on most recent use" off so the numbers stay put.

## Build and install (on the Mac)

```sh
sh build.sh
```

This builds with cargo, installs `~/Applications/Hop.app`, signs it, and starts it as the login agent `xyz.crazyboycjr.hop` (log in `~/Library/Logs/Hop.log`). The first start asks for Accessibility access.

Signing uses the self-signed identity "Local Code Signing" in `~/Library/Keychains/signing.keychain-db`, so the Accessibility grant survives rebuilds (an ad-hoc signature changes with every build and loses it). The keychain has its own password so builds over ssh can sign without unlocking the login keychain.

## Scripting

A Unix socket at `~/.cache/hop.sock` takes the same actions as text:

```sh
printf 'go 3' | nc -U ~/.cache/hop.sock
printf 'send 2' | nc -U ~/.cache/hop.sock
printf 'carry left' | nc -U ~/.cache/hop.sock
printf 'move 1568 4' | nc -U ~/.cache/hop.sock   # a given window to desktop 4
printf 'state' | nc -U ~/.cache/hop.sock
```
