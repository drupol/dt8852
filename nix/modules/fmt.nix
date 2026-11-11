{ inputs, ... }:
{
  flake-file.inputs = {
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  imports = [
    inputs.treefmt-nix.flakeModule
  ];

  perSystem = { pkgs, ... }: {
    treefmt = {
      projectRootFile = "flake.nix";
      programs = {
        jsonfmt.enable = true;
        nixfmt = {
          enable = true;
          package = pkgs.nixfmt-rs;
        };
        oxfmt.enable = true;
        statix.enable = true;
        typos.enable = true;
        yamlfmt.enable = true;
      };
      settings = {
        no-cache = true;
        on-unmatched = "warn";
      };
    };
  };
}
