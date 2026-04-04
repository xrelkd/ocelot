{
  name,
  version,
  lib,
  fpm,
  ocelot-static,
  stdenv,
}:

let
  archMap = {
    "x86_64-linux" = "amd64";
    "aarch64-linux" = "arm64";
  };
in
stdenv.mkDerivation {
  pname = "${name}-deb";
  inherit version;

  src = ./.;

  nativeBuildInputs = [ fpm ];

  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall

    staging=$(mktemp -d)
    mkdir -p "$staging/usr/bin"
    mkdir -p "$staging/usr/share/bash-completion/completions"
    mkdir -p "$staging/usr/share/fish/vendor_completions.d"
    mkdir -p "$staging/usr/share/zsh/site-functions"

    cp ${ocelot-static}/bin/ocelot "$staging/usr/bin/"
    cp ${ocelot-static}/share/bash-completion/completions/* "$staging/usr/share/bash-completion/completions/"
    cp ${ocelot-static}/share/fish/vendor_completions.d/* "$staging/usr/share/fish/vendor_completions.d/"
    cp ${ocelot-static}/share/zsh/site-functions/* "$staging/usr/share/zsh/site-functions/"

    mkdir -p $out
    fpm -s dir -t deb \
      -n ${name} \
      -v ${version} \
      --architecture ${archMap.${stdenv.hostPlatform.system} or "amd64"} \
      --deb-compression xz \
      --prefix / \
      --chdir "$staging" \
      .

    cp *.deb $out/
    runHook postInstall
  '';

  meta = with lib; {
    description = "Process supervisor and init system written in Rust Programming Language (statically linked, DEB package)";
    homepage = "https://github.com/xrelkd/ocelot";
    license = licenses.gpl3Only;
    platforms = platforms.linux;
    maintainers = with maintainers; [ xrelkd ];
  };
}
