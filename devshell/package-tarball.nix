{
  name,
  version,
  ocelot-static,
  target ? "x86_64-unknown-linux-musl",
  runCommand,
}:

runCommand "${name}-${version}-${target}.tar.gz" { } ''
  mkdir stage
  cd stage

  cp -r ${ocelot-static}/bin .
  cp -r ${ocelot-static}/share .

  chmod -R +w .

  mkdir -p $out
  tar -czvf $out/${name}-${version}-${target}.tar.gz \
      --owner=0 --group=0 --numeric-owner \
      bin share
''
