{
  description = "Dev shell for gradle-build-scan-server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

      # Map nix system to tools.lock.json os/cpu values
      systemToOsCpu = {
        "x86_64-linux" = {
          os = "linux";
          cpu = "x86_64";
        };
        "aarch64-linux" = {
          os = "linux";
          cpu = "arm64";
        };
        "x86_64-darwin" = {
          os = "macos";
          cpu = "x86_64";
        };
        "aarch64-darwin" = {
          os = "macos";
          cpu = "arm64";
        };
      };

      # Tools lock file parsed (filter out $schema key)
      toolsLockRaw = builtins.fromJSON (builtins.readFile ./tools/tools.lock.json);
      toolsLock = builtins.removeAttrs toolsLockRaw [ "$schema" ];

      # Find the matching binary entry for a given system
      findBinary =
        toolName: system:
        let
          osCpu = systemToOsCpu.${system};
          binaries = toolsLock.${toolName}.binaries;
          matches = builtins.filter (
            b: b.os == osCpu.os && b.cpu == osCpu.cpu
          ) binaries;
        in
        if matches == [ ] then null else builtins.head matches;

      # Create a derivation for a "file" kind binary tool
      mkFileTool =
        {
          pkgs,
          name,
          binary,
        }:
        pkgs.stdenv.mkDerivation {
          pname = name;
          version = "latest";
          src = pkgs.fetchurl {
            url = binary.url;
            sha256 = binary.sha256;
          };
          dontUnpack = true;
          installPhase = ''
            mkdir -p $out/bin
            cp $src $out/bin/${name}
            chmod +x $out/bin/${name}
          '';
        };

      # Create a derivation for an "archive" kind tool
      mkArchiveTool =
        {
          pkgs,
          name,
          binary,
        }:
        pkgs.stdenv.mkDerivation {
          pname = name;
          version = "latest";
          src = pkgs.fetchurl {
            url = binary.url;
            sha256 = binary.sha256;
          };
          sourceRoot = ".";
          nativeBuildInputs = [ pkgs.gnutar pkgs.gzip ];
          installPhase = ''
            mkdir -p $out/bin
            cp ${binary.file} $out/bin/${name}
            chmod +x $out/bin/${name}
          '';
        };

      # Create the appropriate derivation based on binary kind
      mkTool =
        {
          pkgs,
          name,
          binary,
        }:
        if binary.kind == "archive" then
          mkArchiveTool { inherit pkgs name binary; }
        else
          mkFileTool { inherit pkgs name binary; };

    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          # Build tool derivations for tools that have binaries for this system
          toolDerivations = builtins.filter (t: t != null) (
            map (
              toolName:
              let
                binary = findBinary toolName system;
              in
              if binary == null then null else mkTool { inherit pkgs binary; name = toolName; }
            ) (builtins.attrNames toolsLock)
          );
        in
        {
          default = pkgs.mkShell {
            packages = [
              pkgs.bazelisk
              pkgs.gcc
              pkgs.gradle
              pkgs.pre-commit
            ] ++ toolDerivations;

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
