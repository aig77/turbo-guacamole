{
  inputs,
  withSystem,
  ...
}: let
  nixosModule = {pkgs, ...}: {
    imports = [./nixos-module.nix];
    services.turbo-guacamole.package = withSystem pkgs.stdenv.hostPlatform.system (
      {config, ...}: config.packages.default
    );
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
        networking.firewall.allowedTCPPorts = [8080];
        system.stateVersion = "26.05";
      })
    ];
  };
  tgVm = tgTest.config.system.build.vm;
in {
  flake.nixosModules.default = nixosModule;
  flake.nixosConfigurations.tg-test = tgTest;

  perSystem = {
    system,
    pkgs,
    lib,
    ...
  }: {
    checks = lib.mkIf pkgs.stdenv.isLinux {
      default = pkgs.testers.runNixOSTest {
        name = "turbo-guacamole";
        defaults = {
          system.stateVersion = "26.05";
        };
        nodes.machine = {
          imports = [nixosModule];
          services.turbo-guacamole.enable = true;
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

    apps = lib.mkIf (system == "x86_64-linux") {
      vm = {
        type = "app";
        program = "${tgVm}/bin/run-nixos-vm";
        meta.description = "Run the turbo-guacamole NixOS test VM (UI on http://localhost:8080)";
      };
    };
  };
}
