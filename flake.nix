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
            alsa-lib
            glib-networking
            gsettings-desktop-schemas
            gst_all_1.gst-libav
            gst_all_1.gst-plugins-bad
            gst_all_1.gst-plugins-base
            gst_all_1.gst-plugins-good
            gst_all_1.gstreamer
            gtk3
            libgbm
            libglvnd
            libsoup_3
            libx11
            libxcursor
            libxi
            libxinerama
            libxrandr
            libxxf86vm
            mesa
            libpulseaudio
            pipewire
            wayland
            webkitgtk_4_1
          ]
          ++ lib.optionals stdenv.hostPlatform.isDarwin [
            apple-sdk
          ];
          shellHook = ''
            # GTK schemas and WebKitGTK's GIO TLS backend need explicit discovery
            # paths when running an unwrapped development binary in a Nix shell.
            ${(pkgsFor system).lib.optionalString (pkgsFor system).stdenv.hostPlatform.isLinux ''
              export XDG_DATA_DIRS="${(pkgsFor system).gsettings-desktop-schemas}/share/gsettings-schemas/${(pkgsFor system).gsettings-desktop-schemas.name}:${(pkgsFor system).gtk3}/share/gsettings-schemas/${(pkgsFor system).gtk3.name}:''${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
              export GIO_MODULE_DIR="${(pkgsFor system).glib-networking}/lib/gio/modules/"
              export GIO_EXTRA_MODULES="${(pkgsFor system).glib-networking}/lib/gio/modules''${GIO_EXTRA_MODULES:+:$GIO_EXTRA_MODULES}"
              # OpenAL and GLFW load audio and X11 libraries with dlopen.
              # GLFW needs libXi for raw mouse input and libXcursor for cursors.
              # Keep them discoverable on NixOS, including NVIDIA systems.
              export LD_LIBRARY_PATH="${(pkgsFor system).lib.makeLibraryPath (with (pkgsFor system); [ alsa-lib libpulseaudio pipewire libx11 libxcursor libxi libxinerama libxrandr libxxf86vm ])}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
              # Use Mesa built against the same glibc as this shell's toolchain.
              # The host driver in /run/opengl-driver may require a newer glibc
              # than the nixpkgs revision pinned in flake.lock.
              # Proprietary NVIDIA drivers must stay matched to the host kernel.
              if ! { [ -e /sys/module/nvidia ] || [ -e /proc/driver/nvidia ] || command -v nvidia-smi >/dev/null 2>&1; }; then
                export __EGL_VENDOR_LIBRARY_FILENAMES="''${__EGL_VENDOR_LIBRARY_FILENAMES:-${(pkgsFor system).mesa}/share/glvnd/egl_vendor.d/50_mesa.json}"
                export LIBGL_DRIVERS_PATH="''${LIBGL_DRIVERS_PATH:-${(pkgsFor system).mesa}/lib/dri}"
                export GBM_BACKENDS_PATH="''${GBM_BACKENDS_PATH:-${(pkgsFor system).mesa}/lib/gbm}"
                export LD_LIBRARY_PATH="${(pkgsFor system).lib.makeLibraryPath (with (pkgsFor system); [ libglvnd libgbm wayland mesa ])}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
                echo "Gráficos del flake: Mesa ${(pkgsFor system).mesa.version} / glibc ${(pkgsFor system).glibc.version}."
              fi
            ''}
            # WebKitGTK GPU rendering workarounds.
            #
            # WebKitGTK's DMA-BUF renderer is broken on the NVIDIA proprietary
            # driver under Wayland (WebKit crashes with "Error 71 / Protocol
            # error dispatching to Wayland display"), so software rendering is
            # enabled there. Every other setup (AMD/Intel, NVIDIA on X11, any
            # X11, macOS) keeps the accelerated DMA-BUF renderer.
            #
            # Detection is intentionally broad to cover as many setups as
            # possible (nvidia / nvidia-open modules, /proc node, or nvidia-smi
            # in PATH for Wayland sessions detected via either variable).
            # Override it when your environment differs:
            #   CUBICLAUNCHER_WEBKIT_SOFTWARE=1  -> force software rendering
            #   CUBICLAUNCHER_WEBKIT_SOFTWARE=0  -> force hardware rendering
            if [ -z "''${CUBICLAUNCHER_WEBKIT_SOFTWARE:-}" ]; then
              if [ -e /sys/module/nvidia ] || [ -e /proc/driver/nvidia ] || command -v nvidia-smi >/dev/null 2>&1; then
                if [ "''${XDG_SESSION_TYPE:-}" = "wayland" ] || [ -n "''${WAYLAND_DISPLAY:-}" ]; then
                  CUBICLAUNCHER_WEBKIT_SOFTWARE=1
                fi
              fi
            fi
            if [ "''${CUBICLAUNCHER_WEBKIT_SOFTWARE:-0}" = "1" ]; then
              export __NV_DISABLE_EXPLICIT_SYNC="''${__NV_DISABLE_EXPLICIT_SYNC:-1}"
              export WEBKIT_DISABLE_DMABUF_RENDERER="''${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
              echo "WebKitGTK: renderizado por software (NVIDIA + Wayland)."
            fi
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
