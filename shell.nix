{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    pkg-config
    openssl
    rustc
    cargo
    rust-analyzer
    clippy
    llvmPackages.clang # Important for the linker
  ];
}
