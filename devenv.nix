{ pkgs, config, ... }:

let
  # The three clippy builds of CI (.github/workflows/ci.yml): default, without optional features
  # and full. Dead code behind a feature gate only shows up in one of them. Runs devenv's
  # toolchain through the wrapped `cargo-clippy` of the built-in hook, not whatever `cargo` is
  # on the PATH of the committing shell.
  clippyHook = name: features: {
    enable = true;
    inherit name;
    entry = "${config.git-hooks.hooks.clippy.package}/bin/cargo-clippy clippy --all-targets ${features} -- -D warnings";
    files = "\\.rs$|^Cargo\\.(toml|lock)$";
    pass_filenames = false;
  };
in
{
  packages = [
    pkgs.git
    pkgs.sd
  ];

  languages.rust.enable = true;

  git-hooks.hooks = {
    rustfmt.enable = true;
    clippy = {
      enable = true;
      settings = {
        denyWarnings = true;
        extraArgs = "--all-targets";
      };
    };
    clippy-lite = clippyHook "clippy (no optional features)" "--no-default-features --features mermaid";
    clippy-full = clippyHook "clippy (harper, voikko, swedish)" "--features harper,voikko,swedish";
    explicit = {
      enable = true;
      name = "explicit";
      entry = "cargo run --quiet -- check --offline";
      files = "\\.(md|rs|toml|nix|sh)$";
      pass_filenames = true;
      # Fixtures contain deliberate mistakes for the end-to-end test.
      excludes = [
        "^tests/fixtures/"
        "^eval/"
      ];
    };
  };

  enterTest = ''
    cargo test
  '';
}
