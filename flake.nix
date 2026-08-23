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
          ++ lib.optionals stdenv.isLinux [
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
          ++ lib.optionals stdenv.isDarwin [
            apple-sdk
          ];

          shellHook = ''
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
