# default.nix
{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage {
  pname = "task-athlete-cli";
  version = "0.1.0";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  meta = with pkgs.lib; {
    description = "A library for workout trackers";
    license = licenses.mit;
    maintainers = with maintainers; [ Vilhelm-Ian ];
  };
}

