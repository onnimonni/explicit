{ pkgs, ... }:

{
  packages = [
    pkgs.git
    pkgs.sd
  ];

  languages.rust.enable = true;

  git-hooks.hooks = {
    rustfmt.enable = true;
    clippy.enable = true;
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
