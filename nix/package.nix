{
  lib,
  rustPlatform,
}:

let
  manifest = (lib.importTOML ../Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = manifest.name;
  inherit (manifest) version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
      ../explicit.example.toml
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  # Tests run in CI with cargo; some need a local HTTP server, which the Darwin sandbox blocks.
  doCheck = false;

  meta = {
    inherit (manifest) description;
    homepage = manifest.repository;
    license = with lib.licenses; [
      mit
      asl20
    ];
    mainProgram = "explicit";
  };
}
