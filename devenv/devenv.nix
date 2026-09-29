# devenv module for explicit.
#
# devenv.yaml:
#   inputs:
#     explicit:
#       url: github:onnimonni/explicit
#   imports:
#     - explicit/devenv
#
# The binary comes from the flake input (prebuilt on onnimonni.cachix.org) and a git hook runs
# `explicit check` on staged files when the git-hooks input is present.
#
# The default package is the lite build (no Harper). For the Harper grammar engines, in devenv.nix:
#   explicit.package = inputs.explicit.packages.${pkgs.stdenv.hostPlatform.system}.explicit-full;
{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

let
  cfg = config.explicit;
  system = pkgs.stdenv.hostPlatform.system;
  fromInput = inputs.explicit.packages.${system}.default or null;
in
{
  options.explicit = {
    enable = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Add explicit to the environment.";
    };

    package = lib.mkOption {
      type = lib.types.package;
      default = if fromInput != null then fromInput else pkgs.callPackage ../nix/package.nix { };
      defaultText = lib.literalExpression "inputs.explicit.packages.\${system}.default";
      example = lib.literalExpression "inputs.explicit.packages.\${system}.explicit-full";
      description = ''
        The explicit package: the lite build (`spellbook` engine, no Harper) by default; set it to
        `explicit-full` for the Harper grammar engines. Builds from source when the input is not
        named `explicit`.
      '';
    };

    hook = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = inputs ? git-hooks;
        defaultText = lib.literalExpression "inputs ? git-hooks";
        description = "Run explicit as a git pre-commit hook.";
      };
      args = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ "--offline" ];
        description = "Extra arguments for `explicit check`.";
      };
      files = lib.mkOption {
        type = lib.types.str;
        default = "\\.(md|markdown|mdx|rs|go|py|js|jsx|ts|tsx|sh|bash|nix|ex|exs|zig|c|h|cc|cpp|hpp|rb|java|kt|cs|php|toml|ya?ml)$";
        description = "Regex of files the hook checks.";
      };
      excludes = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        description = "Regexes of files the hook skips.";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    packages = [ cfg.package ];
    cachix.pull = [ "onnimonni" ];

    git-hooks.hooks.explicit = lib.mkIf cfg.hook.enable {
      enable = true;
      name = "explicit";
      entry = "${lib.getExe cfg.package} check ${lib.escapeShellArgs cfg.hook.args}";
      inherit (cfg.hook) files excludes;
      pass_filenames = true;
    };
  };
}
