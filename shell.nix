{ pkgs }:

with pkgs;

let
  rust = pkgs.rust-bin.stable.latest.default.override {
    extensions = [
      "rust-src"
      "rls-preview"
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
    pkg-config
    openssl

    rust
  ];
}
