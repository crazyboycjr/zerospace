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
          };
        };
    };
}
