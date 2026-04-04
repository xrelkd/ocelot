{
  name,
  version,
  lib,
  fpm,
  ocelot-static-cross,
  stdenv,
  rpm,
}:

let
  archMap = {
    "x86_64-linux" = "aarch64";
    "aarch64-linux" = "aarch64";
  };
in
stdenv.mkDerivation {
  pname = "${name}-rpm-aarch64";
  inherit version;

  src = ./.;

  nativeBuildInputs = [
    fpm
    rpm
  ];

  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall

    staging=$(mktemp -d)
    mkdir -p "$staging/usr/bin"
    mkdir -p "$staging/usr/share/bash-completion/completions"
    mkdir -p "$staging/usr/share/fish/vendor_completions.d"
    mkdir -p "$staging/usr/share/zsh/site-functions"

    cp ${ocelot-static-cross}/bin/ocelot "$staging/usr/bin/"
    cp ${ocelot-static-cross}/share/bash-completion/completions/* "$staging/usr/share/bash-completion/completions/"
    cp ${ocelot-static-cross}/share/fish/vendor_completions.d/* "$staging/usr/share/fish/vendor_completions.d/"
    cp ${ocelot-static-cross}/share/zsh/site-functions/* "$staging/usr/share/zsh/site-functions/"

    mkdir -p $out
    fpm -s dir -t rpm \
      -n ${name} \
      -v ${version} \
      --architecture aarch64 \
      --rpm-compression xz \
      --prefix / \
      --chdir "$staging" \
      .

    cp *.rpm $out/
    runHook postInstall
  '';

  meta = with lib; {
    description = "Process supervisor and init system written in Rust Programming Language (statically linked, RPM package, aarch64)";
    homepage = "https://github.com/xrelkd/ocelot";
    license = licenses.gpl3Only;
    platforms = platforms.linux;
    maintainers = with maintainers; [ xrelkd ];
  };
}
