{
  inputs,
  ...
}:
{
  flake-file.inputs = {
    make-shell.url = "github:nicknovitski/make-shell";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };

  imports = [
    inputs.make-shell.flakeModules.default
    inputs.git-hooks.flakeModule
  ];

  perSystem =
    { pkgs, ... }:
    {
      pre-commit.settings.hooks = {
        commitizen.enable = true;
      };

      make-shells.default = {
        packages = with pkgs; [
          cargo
          clippy
          just
          pkg-config
          rust-analyzer
          rustc
          rustfmt
          udev
        ];

        shellHook = ''
          export PATH=$PWD/target/debug:$PATH
          export RUST_SRC_PATH="${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        '';
      };
    };
}
