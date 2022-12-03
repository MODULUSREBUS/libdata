{
  inputs = {
    nix.url = github:MODULUSREBUS/nix;
  };

  outputs = { self, nix }:
    with nix.lib;
    eachSystem [ system.x86_64-linux ] (system: let
      pkgs = nix.packages.${system};
      custom-rust = pkgs.rust-bin.stable.latest.default.override {
        extensions = [
          "rust-src"
        ];
        targets = [
          "x86_64-unknown-linux-gnu"
        ];
      };
    in {
      devShell = pkgs.devshell.mkShell {
        name = "libdata";
        packages = with pkgs; [
          git
          gh
          gnumake

          custom-rust
          rust-analyzer
          cargo-insta
          cargo-edit

          pkg-config
          gcc
          openssl

          protobuf
        ];
        commands = [
          {
            name = "clippy";
            category = "rust";
            help = "rust linter";
            command = "cargo clippy";
            # command = "cargo clippy -- -W clippy::pedantic -A clippy::doc_markdown -A clippy::missing_errors_doc";
          }
        ];
        env = [
          {
            name = "PROTOC";
            value = "${pkgs.protobuf}/bin/protoc";
          }
        ];
      };
    });
}
