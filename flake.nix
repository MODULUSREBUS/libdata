{
  inputs = {
    nix.url = github:MODULUSREBUS/nix;
  };

  outputs = { self, nix }:
    with nix.lib;
    eachSystem [ system.x86_64-linux ] (system: {
      devShell = import ./shell.nix {
        pkgs = nix.packages.${system};
      };
  });
}
