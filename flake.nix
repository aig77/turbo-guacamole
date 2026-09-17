{
  nixConfig = {
    extra-substituters = [
      "https://cache.nixos.org"
      "https://nix-community.cachix.org"
    ];
    extra-trusted-public-keys = [
      "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    git-hooks-nix.url = "github:cachix/git-hooks.nix";
  };

  outputs = {flake-parts, ...} @ inputs:
    flake-parts.lib.mkFlake {inherit inputs;} ({inputs, ...}: let
      nixosModule = {
        config,
        pkgs,
        lib,
        ...
      }: let
        app = "turbo-guacamole";
        cfg = config.services.${app};
        package = inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      in {
        options.services.${app} = {
          enable = lib.mkEnableOption "turbo-guacamole";
          environmentFile = lib.mkOption {
            type = lib.types.nullOr lib.types.path;
            default = null;
            description = ''
              File of KEY=VALUE lines. Mainly used for DATABASE_URL/CACHE_URL when
              using external databases, or to override the local-mode values
              (e.g. /run/secrets/tg.env, chmod 600).
            '';
          };
          database = lib.mkOption {
            type = lib.types.submodule {
              options = {
                local = lib.mkEnableOption "local PostgreSQL and Redis instances";
                postgresUrl = lib.mkOption {
                  type = lib.types.str;
                  default = "";
                  description = "External PostgreSQL URL (required when local is disabled).";
                };
                redisUrl = lib.mkOption {
                  type = lib.types.str;
                  default = "";
                  description = "External Redis URL (required when local is disabled).";
                };
              };
            };
            default = {local = true;};
          };
          host = lib.mkOption {
            type = lib.types.str;
            default = "127.0.0.1";
          };
          port = lib.mkOption {
            type = lib.types.port;
            default = 8080;
          };
        };

        config = let
          dbUrl =
            if cfg.database.local
            then "postgresql:///${app}?host=/run/postgresql&user=${app}"
            else cfg.database.postgresUrl;
          cacheUrl =
            if cfg.database.local
            then "redis://localhost:6379"
            else cfg.database.redisUrl;
        in {
          assertions = [
            {
              assertion =
                cfg.database.local
                || cfg.environmentFile != null
                || (cfg.database.postgresUrl != "" && cfg.database.redisUrl != "");
              message = "services.turbo-guacamole: with database.local disabled, set an environmentFile with DATABASE_URL/CACHE_URL or provide database.postgresUrl and database.redisUrl.";
            }
          ];

          users.groups.${app} = {};
          users.users.${app} = {
            isSystemUser = true;
            group = app;
            description = "Turbo Guacamole service";
          };

          services.postgresql = lib.mkIf cfg.database.local {
            enable = true;
            ensureDatabases = [app];
            ensureUsers = [
              {
                name = app;
                ensureDBOwnership = true;
              }
            ];
          };
          services.redis.servers.${app} = lib.mkIf cfg.database.local {
            enable = true;
            port = 6379;
            appendOnly = true;
            settings = {
              maxmemory = "256mb";
              maxmemory-policy = "allkeys-lru";
            };
          };

          systemd.services.${app} = {
            description = "Turbo Guacamole";
            wantedBy = ["multi-user.target"];
            after =
              ["network.target"]
              ++ lib.optionals cfg.database.local ["postgresql.service" "postgresql-setup.service" "redis-${app}.service"];
            requires = lib.optionals cfg.database.local ["postgresql.service" "postgresql-setup.service" "redis-${app}.service"];
            serviceConfig =
              {
                ExecStart = "${package}/bin/${app}";
                Environment = [
                  "SERVICE_HOST=${cfg.host}"
                  "SERVICE_PORT=${toString cfg.port}"
                  "DATABASE_URL=${dbUrl}"
                  "CACHE_URL=${cacheUrl}"
                ];
                WorkingDirectory = package;
                StateDirectory = "${app}";
                User = "${app}";
                Group = "${app}";
                Restart = "on-failure";
                RestartSec = "5s";
              }
              // lib.optionalAttrs (cfg.environmentFile != null) {
                EnvironmentFile = toString cfg.environmentFile;
              };
          };

          systemd.services."${app}-schema" = lib.mkIf cfg.database.local {
            description = "Apply the turbo-guacamole database schema";
            after = ["postgresql.service" "postgresql-setup.service"];
            wantedBy = ["${app}.service"];
            wants = ["postgresql.service" "postgresql-setup.service"];
            before = ["${app}.service"];
            serviceConfig = {
              Type = "oneshot";
              RemainAfterExit = true;
              User = app;
              Group = app;
            };
            script = ''
              ${pkgs.postgresql}/bin/psql \
                -h /run/postgresql -d ${app} -U ${app} \
                -v ON_ERROR_STOP=1 \
                -f ${./sql/schema.sql}
            '';
          };
        };
      };

      tgTest = inputs.nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          nixosModule
          ({modulesPath, ...}: {
            imports = ["${modulesPath}/virtualisation/qemu-vm.nix"];
            services.turbo-guacamole.enable = true;
            services.turbo-guacamole.host = "0.0.0.0";
            virtualisation.forwardPorts = [
              {
                from = "host";
                proto = "tcp";
                host.port = 8080;
                guest.port = 8080;
              }
            ];
            virtualisation.graphics = false;
            networking.firewall.allowedTCPPorts = [ 8080 ];
            system.stateVersion = "26.05";
          })
        ];
      };
      tgVm = tgTest.config.system.build.vm;
    in {
      imports = [inputs.git-hooks-nix.flakeModule];

      systems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin"];

      flake.nixosConfigurations.tg-test = tgTest;

      flake.nixosModules.default = nixosModule;

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
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      in {
        packages.default = rustPlatform.buildRustPackage {
          pname = "turbo-guacamole";
          version = (fromTOML (builtins.readFile ./Cargo.toml)).package.version;
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = [pkgs.pkg-config];
          buildInputs = [pkgs.openssl];
          release = true;
          doCheck = false;
          postInstall = ''
            cp -r static $out/static
          '';
        };

        apps = lib.mkIf (system == "x86_64-linux") {
          vm = {
            type = "app";
            program = "${tgVm}/bin/run-nixos-vm";
            meta.description = "Run the turbo-guacamole NixOS test VM (UI on http://localhost:8080)";
          };
        };

        checks = lib.mkIf pkgs.stdenv.isLinux {
          default = pkgs.testers.runNixOSTest {
            name = "turbo-guacamole";
            defaults = {
              system.stateVersion = "26.05";
            };
            nodes.machine = {
              imports = [nixosModule];
            };
            testScript = ''
              machine.wait_for_unit("postgresql.service")
              machine.wait_for_unit("turbo-guacamole-schema.service")
              machine.wait_for_unit("turbo-guacamole.service")
              machine.wait_for_open_port(8080)
              machine.succeed("curl -sf http://127.0.0.1:8080/health")
              code = machine.succeed(
                  "curl -s -X POST -H 'Content-Type: application/json' "
                  "-d '{\"url\":\"https://example.com/tg-test\"}' "
                  "http://127.0.0.1:8080/shorten | "
                  "sed 's/.*\"code\":\"//; s/\".*//'"
              ).strip()
              machine.succeed(
                  "curl -s -D- -o /dev/null http://127.0.0.1:8080/" + code + " | grep -qiE '^HTTP/1.1 30[27]'"
              )
              machine.succeed("curl -sf http://127.0.0.1:8080/stats")
            '';
          };
        };

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
    });
}
