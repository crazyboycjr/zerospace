# ZeroSpace.app as build.sh lays it out, built by nix and signed ad hoc.
{ lib, rustPlatform, apple-sdk_26, rcodesign }:

rustPlatform.buildRustPackage {
  pname = "zerospace";
  inherit ((lib.importTOML ./Cargo.toml).package) version;

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./build.rs
      ./src
      ./Info.plist
      ./assets/icon/AppIcon.icns
    ];
  };

  cargoLock.lockFile = ./Cargo.lock;

  # rcodesign signs the bundle ad hoc without Apple's codesign, which the build cannot reach
  nativeBuildInputs = [ rcodesign ];
  # The SDK build.sh gets from the Command Line Tools; an SDK before 26 gives the older window look
  buildInputs = [ apple-sdk_26 ];

  doCheck = false; # no tests

  postInstall = ''
    contents=$out/Applications/ZeroSpace.app/Contents
    mkdir -p $contents/MacOS $contents/Resources
    mv $out/bin/zerospace $contents/MacOS/ZeroSpace
    ln -s $contents/MacOS/ZeroSpace $out/bin/zerospace
    cp Info.plist $contents/Info.plist
    cp assets/icon/AppIcon.icns $contents/Resources/AppIcon.icns
  '';

  # After stripping, which rewrites the binary. The linker's own signature covers the binary
  # alone; sealing the bundle binds Info.plist, so the identifier is the bundle id, as with
  # build.sh, and `codesign --verify` passes.
  postFixup = ''
    rcodesign sign $out/Applications/ZeroSpace.app
  '';

  meta = {
    description = "Remove the motion from macOS desktop switching";
    homepage = "https://github.com/crazyboycjr/zerospace";
    license = lib.licenses.mit;
    mainProgram = "zerospace";
    platforms = lib.platforms.darwin;
  };
}
