_: {
  perSystem = {
    config,
    lib,
    pkgs,
    rustToolchain,
    ...
  }: {
    devShells.default = pkgs.mkShell {
      packages = with pkgs; [
        rustToolchain
        rust-analyzer
        pre-commit
        docker-compose
        postgresql
        redis
        flyctl
      ];

      RUST_BACKTRACE = 1;

      shellHook = ''
        ${config.pre-commit.installationScript}
        echo "✂️ Keep it short... and memory safe."
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
}
