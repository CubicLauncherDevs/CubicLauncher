{
  description = "CubicLauncher — launcher de Minecraft de código abierto con Tauri";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };
  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = system: import nixpkgs { inherit system; };
    in
    {
      packages = forAllSystems (system: {
        default = (pkgsFor system).callPackage ./dist/nix/package.nix { };
        cubiclauncher = self.packages.${system}.default;
      });
      devShells = forAllSystems (system: {
        default = (pkgsFor system).mkShell {
          name = "cubiclauncher-dev";
          packages = with (pkgsFor system); [
            bun
            cargo
            cargo-tauri
            clippy
            desktop-file-utils
            nixpkgs-fmt
            nodejs
            openssl
            pkg-config
            rustc
            rustfmt
          ]
          ++ lib.optionals stdenv.hostPlatform.isLinux [
            glib-networking
            gst_all_1.gst-libav
            gst_all_1.gst-plugins-bad
            gst_all_1.gst-plugins-base
            gst_all_1.gst-plugins-good
            gst_all_1.gstreamer
            gtk3
            libsoup_3
            webkitgtk_4_1
          ]
          ++ lib.optionals stdenv.hostPlatform.isDarwin [
            apple-sdk
          ];
          shellHook = ''
            # WebKitGTK uses GIO's TLS backend. In Nix shells the module can be
            # installed but not discoverable unless GIO_EXTRA_MODULES is set.
            ${(pkgsFor system).lib.optionalString (pkgsFor system).stdenv.hostPlatform.isLinux ''
              export GIO_EXTRA_MODULES="${(pkgsFor system).glib-networking}/lib/gio/modules''${GIO_EXTRA_MODULES:+:$GIO_EXTRA_MODULES}"
            ''}
            # WebKitGTK + NVIDIA proprietary driver on Wayland (Hyprland): the
            # DMA-BUF renderer trips "Error 71 (Protocol error) dispatching to
            # Wayland display". Explicit sync is off by default on NVIDIA EGL
            # and the DMABUF renderer is disabled as a reliable fallback.
            export __NV_DISABLE_EXPLICIT_SYNC=1
            export WEBKIT_DISABLE_DMABUF_RENDERER=1
            echo "Entorno de desarrollo de CubicLauncher listo."
            echo "Comandos útiles:"
            echo "  bun install"
            echo "  bun run tauri dev"
            echo "  bun run build"
            echo "  cargo build --release -p cubiclauncher"
          '';
        };
      });
      formatter = forAllSystems (system: (pkgsFor system).nixpkgs-fmt);

      checks = forAllSystems (system: {
        build = self.packages.${system}.default;
      });
    };
}
