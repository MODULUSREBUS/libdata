{ pkgs }:

with pkgs;

let
  rust = pkgs.rust-bin.stable.latest.default.override {
    extensions = [
      "rust-src"
    ];
    targets = [
      "x86_64-unknown-linux-gnu"
    ];
  };

in
mkShell {
  buildInputs = [
    git
    gh
    gnumake

    rust
    rust-analyzer
    cargo-insta
    cargo-edit

    pkg-config
    openssl

    protobuf
  ];
  PROTOC="${protobuf}/bin/protoc";
}
