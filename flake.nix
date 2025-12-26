{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk.url = "github:nix-community/naersk";
    git-hooks-nix.url = "github:cachix/git-hooks.nix";
  };

  outputs = {flake-parts, ...} @ inputs:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [inputs.git-hooks-nix.flakeModule];

      systems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin"];

      perSystem = {
        system,
        config,
        lib,
        ...
      }: let
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [inputs.rust-overlay.overlays.default];
        };

        # for additional versions: https://github.com/oxalica/rust-overlay
        rustToolchain = pkgs.rust-bin.stable.latest.default;

        naerskLib = pkgs.callPackage inputs.naersk {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      in {
        packages.default = naerskLib.buildPackage {src = ./.;};

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustToolchain
            rust-analyzer
            pre-commit
          ];

          RUST_BACKTRACE = 1;

          shellHook = ''
            ${config.pre-commit.installationScript}
            echo "🦀 $(rustc --version)"
          '';
        };

        pre-commit = {
          check.enable = false; # Disabled because clippy needs network access for dependencies
          settings = {
            hooks = {
              rustfmt.enable = true;
              clippy.enable = true;
            };
            tools = {
              cargo = lib.mkForce rustToolchain;
              clippy = lib.mkForce rustToolchain;
            };
          };
        };
      };
    };
}
