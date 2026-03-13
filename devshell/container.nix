{
  name,
  version,
  dockerTools,
  ocelot,
  buildEnv,
  ...
}:

dockerTools.buildImage {
  inherit name;
  tag = "v${version}";

  copyToRoot = buildEnv {
    name = "image-root";
    paths = [ ocelot ];
    pathsToLink = [ "/bin" ];
  };

  config = {
    Entrypoint = [
      "${ocelot}/bin/ocelot"
      "idle"
    ];
  };
}
