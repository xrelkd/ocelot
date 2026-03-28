{
  name,
  version,
  lib,
  rustPlatform,
  installShellFiles,
  cargo-nextest,
}:

rustPlatform.buildRustPackage {
  pname = name;
  inherit version;

  src = lib.cleanSource ./..;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [
    installShellFiles
  ];

  nativeCheckInputs = [
    cargo-nextest
  ];

  checkFlags = [ "--skip-ignored" ];
  checkPhase = ''
    runHook preCheck
    cargo nextest run $checkFlags
    runHook postCheck
  '';

  postInstall = ''
    installShellCompletion --cmd ocelot \
      --bash <($out/bin/ocelot completions bash) \
      --fish <($out/bin/ocelot completions fish) \
      --zsh  <($out/bin/ocelot completions zsh)
  '';

  meta = with lib; {
    description = "Process supervisor and init system written in Rust Programming Language";
    homepage = "https://github.com/xrelkd/ocelot";
    license = licenses.gpl3Only;
    platforms = platforms.linux;
    maintainers = with maintainers; [ xrelkd ];
    mainProgram = "ocelot";
  };
}
