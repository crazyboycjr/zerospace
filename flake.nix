{
  description = "ZeroSpace: instant desktop switching on macOS";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    { self, nixpkgs }:
    {
      # Apple Silicon only: nixpkgs dropped x86_64-darwin in 26.11
      packages.aarch64-darwin.default = nixpkgs.legacyPackages.aarch64-darwin.callPackage ./package.nix { };

      # The login agent build.sh writes, for nix-darwin: same label, started at login and kept
      # running, logging to ~/Library/Logs/ZeroSpace.log of the system's primary user.
      darwinModules.default =
        { config, lib, pkgs, ... }:
        let
          cfg = config.services.zerospace;
          user = config.system.primaryUser;
          home = if user == null then "" else config.users.users.${user}.home or null;
        in
        {
          options.services.zerospace = {
            enable = lib.mkEnableOption "ZeroSpace, instant desktop switching, as a login agent";

            package = lib.mkOption {
              type = lib.types.package;
              default = pkgs.callPackage ./package.nix { };
              defaultText = lib.literalMD "ZeroSpace built with this system's nixpkgs";
              description = "The ZeroSpace package to run.";
            };

            logFile = lib.mkOption {
              type = lib.types.str;
              default = "${if home == null then "/Users/${user}" else home}/Library/Logs/ZeroSpace.log";
              defaultText = lib.literalExpression ''"<home of system.primaryUser>/Library/Logs/ZeroSpace.log"'';
              description = "Where the agent's standard error goes. launchd does not expand `~`.";
            };

            disableDesktopShortcuts = lib.mkOption {
              type = lib.types.bool;
              default = true;
              description = ''
                Turn off macOS's own "Switch to Desktop 1" to "Switch to Desktop 5" shortcuts (System
                Settings > Keyboard > Keyboard Shortcuts > Mission Control). macOS handles them before
                any app, so when they use cmd+1 to cmd+5 ZeroSpace never sees the keys and desktops
                switch with the usual slide. No other shortcut is changed.
              '';
            };
          };

          config = lib.mkIf cfg.enable {
            environment.systemPackages = [ cfg.package ];

            launchd.user.agents.zerospace.serviceConfig = {
              Label = "xyz.crazyboycjr.zerospace";
              ProgramArguments = [ "${cfg.package}/Applications/ZeroSpace.app/Contents/MacOS/ZeroSpace" ];
              RunAtLoad = true;
              KeepAlive = true;
              ProcessType = "Interactive";
              StandardErrorPath = cfg.logFile;
            };

            # Switch to Desktop 1 to 5 are the symbolic hot keys 118 to 122. -dict-add changes only
            # those entries, where writing AppleSymbolicHotKeys whole, as system.defaults does, would
            # reset every other shortcut; activateSettings applies it without logging out.
            system.activationScripts.postActivation.text = lib.mkIf cfg.disableDesktopShortcuts ''
              for id in 118 119 120 121 122; do
                launchctl asuser "$(id -u -- ${user})" sudo --user=${user} -- \
                  defaults write com.apple.symbolichotkeys AppleSymbolicHotKeys -dict-add "$id" '<dict><key>enabled</key><false/></dict>'
              done
              launchctl asuser "$(id -u -- ${user})" sudo --user=${user} -- \
                /System/Library/PrivateFrameworks/SystemAdministration.framework/Resources/activateSettings -u
            '';
          };
        };
    };
}
