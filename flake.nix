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
              targets.x86_64-unknown-linux-musl.stable.rust-std
              default.rustfmt
            ];

          rustToolchainMusl =
            with fenix.packages.${system};
            combine [
              stable.rustc
              stable.cargo
              stable.clippy
              stable.rust-src
              stable.rust-std
              targets.x86_64-unknown-linux-musl.stable.rust-std
            ];

          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };

          rustPlatformMusl = pkgs.pkgsStatic.makeRustPlatform {
            cargo = rustToolchainMusl;
            rustc = rustToolchainMusl;
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
            inherit
              rustToolchain
              rustToolchainMusl
              cargoArgs
              unitTestArgs
              ;
          };

          packages = rec {
            default = ocelot;
            ocelot = pkgs.callPackage ./devshell/package.nix {
              inherit (cargoToml.workspace.metadata.crane) name;
              inherit (cargoToml.workspace.package) version;
              inherit rustPlatform;
            };
            ocelot-static = pkgs.pkgsStatic.callPackage ./devshell/package-static.nix {
              inherit (cargoToml.workspace.metadata.crane) name;
              inherit (cargoToml.workspace.package) version;
              rustPlatform = rustPlatformMusl;
            };
            container = pkgs.callPackage ./devshell/container.nix {
              inherit (cargoToml.workspace.metadata.crane) name;
              inherit (cargoToml.workspace.package) version;
              inherit ocelot;
            };
            check-format = pkgs.callPackage ./devshell/format.nix { };
          };
        };
    };
}
