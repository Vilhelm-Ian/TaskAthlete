{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage {
  pname = "task-athlete-cli";
  version = "0.1.0";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  buildInputs = with pkgs; [
    openssl
    pkg-config  
  ];

  # This tells openssl-sys where to find OpenSSL
  RUSTFLAGS = "-L ${pkgs.openssl.dev}/lib";
  OPENSSL_DIR = "${pkgs.openssl.dev}";
  OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
  OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";

  meta = with pkgs.lib; {
    description = "A library for workout trackers";
    license = licenses.mit;
    maintainers = with maintainers; [ Vilhelm-Ian ];
  };
}

