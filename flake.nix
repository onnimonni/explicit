{
  description = "Lint prose in Markdown and code comments: structure, spelling, grammar, AI slop, links and diagrams";

  nixConfig = {
    extra-substituters = [ "https://onnimonni.cachix.org" ];
    extra-trusted-public-keys = [ "onnimonni.cachix.org-1:bAPuRbTAiFMLNLoojt7KlqhQcpdeTN/OMIL22fP3LyM=" ];
  };

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        explicit = pkgs.callPackage ./nix/package.nix { };
        default = explicit;
      });

      overlays.default = final: _prev: {
        explicit = final.callPackage ./nix/package.nix { };
      };

      # devenv module; import it with `imports: [ explicit/devenv ]` in devenv.yaml.
      devenvModules.default = ./devenv/devenv.nix;

      formatter = forAllSystems (pkgs: pkgs.nixfmt-rfc-style);
    };
}
