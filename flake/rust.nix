{inputs, ...}: {
  perSystem = {system, ...}: let
    pkgs = import inputs.nixpkgs {
      inherit system;
      overlays = [inputs.rust-overlay.overlays.default];
    };

    # for additional versions: https://github.com/oxalica/rust-overlay
    rustToolchain = pkgs.rust-bin.stable.latest.default;
  in {
    _module.args = {
      inherit pkgs rustToolchain;
      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };
    };
  };
}
