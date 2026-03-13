{
  self,
  system,
  lib,
  pkgs,
  ...
}:

pkgs.stdenv.mkDerivation {
  name = "integration-test";

  nativeBuildInputs = [
    pkgs.procps
    pkgs.util-linux
  ];

  src = lib.cleanSource ./..;

  installPhase = ''
    mkdir -p $out

    bash ./tests/test-entry.sh ${self.packages.${system}.default}/bin/ocelot
    echo "Success" > $out/success
  '';
}
