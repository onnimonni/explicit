# Prebuilt release binary from GitHub releases, for hosts without the binary cache (or without
# a Rust toolchain). The release workflow (.github/workflows/release.yml) builds the tarballs
# and writes their hashes and version to prebuilt-hashes.json, so this package always points at
# the latest published release, even after `version` in Cargo.toml has been bumped.
{
  lib,
  stdenv,
  fetchurl,
  autoPatchelfHook,
  full ? false,
}:

let
  hashes = lib.importJSON ./prebuilt-hashes.json;
  manifest = (lib.importTOML ../Cargo.toml).package;
  system = stdenv.hostPlatform.system;
  name = "explicit" + lib.optionalString full "-full";
  target =
    {
      "x86_64-linux" = "x86_64-unknown-linux-gnu";
      "aarch64-linux" = "aarch64-unknown-linux-gnu";
      "x86_64-darwin" = "x86_64-apple-darwin";
      "aarch64-darwin" = "aarch64-apple-darwin";
    }
    .${system} or (throw "explicit: no prebuilt binary for ${system}");
  hash = hashes.${system}.${name} or "";
in
stdenv.mkDerivation {
  pname = "${name}-prebuilt";
  inherit (hashes) version;

  src = fetchurl {
    url = "https://github.com/onnimonni/explicit/releases/download/v${hashes.version}/${name}-v${hashes.version}-${target}.tar.gz";
    # An empty hash (no release yet for this platform) fails at build time with the real hash.
    hash = if hash == "" then lib.fakeHash else hash;
  };

  # The tarball holds a single `explicit` binary.
  sourceRoot = ".";

  # The Linux binary needs its dynamic linker and libgcc_s patched; Darwin binaries are
  # self-contained.
  nativeBuildInputs = lib.optionals stdenv.hostPlatform.isLinux [ autoPatchelfHook ];
  buildInputs = lib.optionals stdenv.hostPlatform.isLinux [ stdenv.cc.cc.lib ];

  dontBuild = true;
  dontConfigure = true;

  installPhase = ''
    runHook preInstall
    install -Dm755 explicit $out/bin/explicit
    runHook postInstall
  '';

  meta = {
    description = manifest.description + " (prebuilt release binary)";
    homepage = manifest.repository;
    license = lib.licenses.gpl3Plus;
    mainProgram = "explicit";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
  };
}
