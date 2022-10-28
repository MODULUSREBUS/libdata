{
  inputs = {
    nix.url = github:MODULUSREBUS/nix;
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nix, crane }:
    with nix.lib;
    eachSystem [ system.x86_64-linux ] (system: let
      pkgs = nix.packages.${system};
      craneLib = crane.lib.${system};
    in {
      devShell = import ./shell.nix {
        inherit pkgs;
      };
      packages.default = craneLib.buildPackage {
        src = craneLib.cleanCargoSource ./.;
        buildInputs = with pkgs; [
          pkg-config
          openssl
          protobuf
        ];
        PROTOC="${pkgs.protobuf}/bin/protoc";
      };
  });
}
