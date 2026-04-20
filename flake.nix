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
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          # Shared libraries needed by agent-browser's bundled Chrome
          chromeLibs = pkgs.lib.makeLibraryPath [
            pkgs.alsa-lib
            pkgs.at-spi2-atk
            pkgs.atk
            pkgs.cairo
            pkgs.cups
            pkgs.dbus
            pkgs.expat
            pkgs.glib
            pkgs.gtk3
            pkgs.libdrm
            pkgs.libgbm
            pkgs.libglvnd
            pkgs.libX11
            pkgs.libXcomposite
            pkgs.libXdamage
            pkgs.libXext
            pkgs.libXfixes
            pkgs.libxcb
            pkgs.libxkbcommon
            pkgs.libXrandr
            pkgs.mesa
            pkgs.nspr
            pkgs.nss
            pkgs.pango
            pkgs.pipewire
            pkgs.stdenv.cc.cc.lib
            pkgs.systemd
            pkgs.zlib
          ];
        in
        {
          default = pkgs.mkShell {
            packages = [
              pkgs.bazelisk
              pkgs.gcc
              pkgs.gradle
              pkgs.jdk21
              pkgs.pre-commit
            ];

            # Linux: set NIX_LD so bazelisk can run downloaded Bazel binaries
            # On NixOS, this additionally requires `programs.nix-ld.enable = true`
            env = {
              JAVA_HOME = "${pkgs.jdk21}";
            } // pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
              NIX_LD = pkgs.stdenv.cc.bintools.dynamicLinker;
              NIX_LD_LIBRARY_PATH = chromeLibs;
            };

            shellHook = pkgs.lib.optionalString pkgs.stdenv.isLinux ''
              # Wrap agent-browser's bundled Chrome so it can find shared libraries.
              # agent-browser strips LD_LIBRARY_PATH when spawning Chrome, so we
              # inject it via a wrapper script with hardcoded paths.
              _wrap_agent_browser_chrome() {
                local chrome_dir="$HOME/.agent-browser/browsers"
                if [[ -d "$chrome_dir" ]]; then
                  for dir in "$chrome_dir"/chrome-*; do
                    local chrome="$dir/chrome"
                    if [[ -f "$chrome" ]] && ! head -1 "$chrome" 2>/dev/null | grep -q "^#!"; then
                      mv "$chrome" "$chrome.orig"
                      cat > "$chrome" << WRAPPER
              #!/usr/bin/env bash
              export LD_LIBRARY_PATH="${chromeLibs}"
              exec "\$(dirname "\$0")/chrome.orig" "\$@"
              WRAPPER
                      chmod +x "$chrome"
                    fi
                  done
                fi
              }
              _wrap_agent_browser_chrome
            '';
          };
        }
      );
    };
}
