{
  description = "Ocelot - Process supervisor and init system written in Rust Programming Language";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    inputs@{
      self,
      nixpkgs,
      flake-parts,
      fenix,
      crane,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {

      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      flake = {
        overlays.default = final: prev: { };
      };

      perSystem =
        {
          config,
          self',
          inputs',
          pkgs,
          system,
          ...
        }:
        let

          pkgs = import nixpkgs {
            inherit system;
            overlays = [
              self.overlays.default
              fenix.overlays.default
            ];
          };

          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

          rustToolchain =
            with fenix.packages.${system};
            combine [
              stable.rustc
              stable.cargo
              stable.clippy
              stable.rust-src
              stable.rust-std
              default.rustfmt
            ];

          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };

          cargoArgs = [
            "--workspace"
            "--bins"
            "--examples"
            "--tests"
            "--benches"
            "--all-targets"
          ];
          unitTestArgs = [ "--workspace" ];
        in
        {

          formatter = pkgs.treefmt;

          devShells.default = pkgs.callPackage ./devshell {
            inherit rustToolchain cargoArgs unitTestArgs;
          };

          packages = rec {
            ocelot = pkgs.callPackage ./devshell/package.nix {
              inherit (cargoToml.workspace.metadata.crane) name;
              inherit (cargoToml.workspace.package) version;
              inherit rustPlatform;
            };
            container = pkgs.callPackage ./devshell/container.nix {
              inherit (cargoToml.workspace.metadata.crane) name;
              inherit (cargoToml.workspace.package) version;
              inherit ocelot;
            };
            default = ocelot;
          };

          checks.integration-test = pkgs.callPackage ./devshell/integration-test.nix {
            inherit self system pkgs;
          };
        };
    };
}
