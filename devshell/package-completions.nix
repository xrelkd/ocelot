{
  runCommand,
  installShellFiles,
  ocelot,
}:

runCommand "caracal-completions"
  {
    nativeBuildInputs = [ installShellFiles ];
  }

  ''
    installShellCompletion --cmd ocelot \
      --bash <(${ocelot}/bin/ocelot completions bash) \
      --fish <(${ocelot}/bin/ocelot completions fish) \
      --zsh  <(${ocelot}/bin/ocelot completions zsh)
  ''
