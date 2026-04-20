{
  description = "Dev shell for gradle-build-scan-server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    nix-wrapper-modules.url = "github:BirdeeHub/nix-wrapper-modules";
    nix-wrapper-modules.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    { self, nixpkgs, nix-wrapper-modules, ... }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      wlib = nix-wrapper-modules.lib;

      # Release binary URLs for tools not in nixpkgs
      binaryReleases = {
        aspect = {
          version = "2026.4.2";
          binaries = {
            "x86_64-linux" = {
              url = "https://github.com/aspect-build/aspect-cli/releases/download/v2026.4.2/aspect-cli-x86_64-unknown-linux-musl";
              sha256 = "58057a7bfb94838749cbb3fedc015baeefa1887caf00e1ed4dd5eb8ef00c6cef";
            };
            "aarch64-darwin" = {
              url = "https://github.com/aspect-build/aspect-cli/releases/download/v2026.4.2/aspect-cli-aarch64-apple-darwin";
              sha256 = "5cae50dcd8a2548ec433833a80d8e0ef3d41965c3673db700f8bbc52e5f15600";
            };
          };
        };
        gazelle = {
          version = "2025.40.148";
          binaries = {
            "x86_64-linux" = {
              url = "https://github.com/aspect-build/aspect-gazelle/releases/download/2025.40.148+4396cce/gazelle-linux_amd64_cgo";
              sha256 = "516da161c2a6997fb7e7e2880447428da01bf0220747fb0c35619bacd283f2af";
            };
            "aarch64-linux" = {
              url = "https://github.com/aspect-build/aspect-gazelle/releases/download/2025.40.148+4396cce/gazelle-linux_arm64_cgo";
              sha256 = "60a09694ddc455ac6cd458d6d89f21b82f3a7813b290dcd52307cd8dde74b5b3";
            };
            "aarch64-darwin" = {
              url = "https://github.com/aspect-build/aspect-gazelle/releases/download/2025.40.148+4396cce/gazelle-darwin_arm64_cgo";
              sha256 = "2dce01cde75c03c0190a9e40f0713e3b80844e1bed1af3ed73d684263d823896";
            };
          };
        };
        agent-browser = {
          version = "0.23.0";
          binaries = {
            "x86_64-linux" = {
              url = "https://github.com/vercel-labs/agent-browser/releases/download/v0.23.0/agent-browser-linux-x64";
              sha256 = "8a699c73fb697d6df4260c6a3e7f9c4a5dec8e08e33248ed01b0ae83fd8c7a4a";
            };
            "aarch64-linux" = {
              url = "https://github.com/vercel-labs/agent-browser/releases/download/v0.23.0/agent-browser-linux-arm64";
              sha256 = "bb9243ea0d989d856e7df052b93b8bd1c6748a0dbfe96284aba7a5caa2e58613";
            };
            "aarch64-darwin" = {
              url = "https://github.com/vercel-labs/agent-browser/releases/download/v0.23.0/agent-browser-darwin-arm64";
              sha256 = "354c1cdaf6b50ea01bbb7b20aef4ab67b45a3e1db8a32269edf8fd970e532683";
            };
          };
        };
      };

      # Create a wrapped tool using nix-wrapper-modules
      mkWrappedTool =
        { pkgs, name, version, url, sha256 }:
        wlib.wrapPackage (
          { config, wlib, ... }: {
            imports = [ wlib.modules.default ];
            config.pkgs = pkgs;
            config.binName = name;
            config.exePath = "bin/${name}";
            config.wrapperImplementation = "binary";
            config.package = pkgs.stdenv.mkDerivation {
              pname = name;
              inherit version;
              src = pkgs.fetchurl { inherit url sha256; };
              dontUnpack = true;
              installPhase = ''
                install -Dm755 $src $out/bin/${name}
              '';
            };
          }
        );

    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          # Build wrapped tools for this system (skip if no binary for this platform)
          wrappedTools = builtins.filter (t: t != null) (
            builtins.attrValues (builtins.mapAttrs (
              name: tool:
              let
                binary = tool.binaries.${system} or null;
              in
              if binary == null then null
              else mkWrappedTool {
                inherit pkgs name;
                inherit (tool) version;
                inherit (binary) url sha256;
              }
            ) binaryReleases)
          );

          # Wrapper so `bazel` invokes bazelisk
          bazel = pkgs.writeShellScriptBin "bazel" ''exec ${pkgs.bazelisk}/bin/bazelisk "$@"'';
        in
        {
          default = pkgs.mkShell {
            packages = [
              bazel
              pkgs.bazelisk
              pkgs.bazel-watcher # ibazel
              pkgs.buildifier
              pkgs.buildozer
              pkgs.buf
              pkgs.cargo
              pkgs.rustc
              pkgs.gcc
              pkgs.gradle
              pkgs.pre-commit
              pkgs.starpls
            ] ++ wrappedTools;

            # Linux: set NIX_LD so bazelisk can run downloaded Bazel binaries
            # On NixOS, this additionally requires `programs.nix-ld.enable = true`
            env = pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
              NIX_LD = pkgs.stdenv.cc.bintools.dynamicLinker;
              NIX_LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
                pkgs.stdenv.cc.cc.lib
                pkgs.zlib
              ];
            };
          };
        }
      );
    };
}
