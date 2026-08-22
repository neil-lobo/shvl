{
  pkgs ? import <nixpkgs> { },
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;
  cargoLock.lockFile = ./Cargo.lock;
  src = pkgs.lib.cleanSource ./.;

  nativeBuildInputs = [ pkgs.installShellFiles ];

  postInstall = pkgs.lib.optionalString (pkgs.stdenv.buildPlatform.canExecute pkgs.stdenv.hostPlatform) ''
    installShellCompletion --cmd shvl \
      --bash <($out/bin/shvl completion bash)
  '';
}
