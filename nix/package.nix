{
  lib,
  rustPlatform,
  # With the Harper grammar engines (cargo feature `harper`), Finnish (`voikko`) and Swedish
  # (`swedish`). The default lite build has only the `spellbook` engine, with the same spelling
  # and word-class data.
  full ? false,
}:

let
  manifest = (lib.importTOML ../Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = manifest.name + lib.optionalString full "-full";
  inherit (manifest) version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
      ../examples
      ../dictionaries
      ../explicit.example.toml
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  # Finnish (`voikko`, pure Rust with the voikko-fi data embedded) and Swedish only in the
  # full build.
  buildFeatures = lib.optionals full [
    "harper"
    "voikko"
    "swedish"
  ];

  # Tests run in CI with cargo; some need a local HTTP server, which the Darwin sandbox blocks.
  doCheck = false;

  meta = {
    inherit (manifest) description;
    homepage = manifest.repository;
    license = lib.licenses.gpl3Plus;
    mainProgram = "explicit";
  };
}
