_: {
  perSystem = {
    pkgs,
    rustPlatform,
    ...
  }: {
    packages.default = rustPlatform.buildRustPackage {
      pname = "turbo-guacamole";
      version = (fromTOML (builtins.readFile ../Cargo.toml)).package.version;
      src = ../.;
      cargoLock = {
        lockFile = ../Cargo.lock;
      };
      nativeBuildInputs = [pkgs.pkg-config];
      buildInputs = [pkgs.openssl];
      release = true;
      doCheck = false;
      postInstall = ''
        cp -r static $out/static
      '';
    };
  };
}
