{
  description = "CubicLauncher - Open-source Minecraft launcher";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config = { };
        };
        lib = pkgs.lib;
      in
      {
        packages = {
          default = pkgs.callPackage ./default.nix { };
          cubiclauncher = pkgs.callPackage ./default.nix { };
        };

        devShells.default = pkgs.mkShell {
          buildInputs =
            [
              pkgs.bun
              pkgs.cargo
              pkgs.rustc
              pkgs.pkg-config
            ]
            ++ lib.optionals pkgs.stdenv.isLinux [
              pkgs.gtk3
              pkgs.webkitgtk_4_1
              pkgs.libsoup_3
              pkgs.openssl
              pkgs.librsvg
              pkgs.glib-networking
              pkgs.gsettings-desktop-schemas
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.AppKit
              pkgs.darwin.apple_sdk.frameworks.WebKit
              pkgs.darwin.apple_sdk.frameworks.Foundation
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
              pkgs.darwin.apple_sdk.frameworks.Security
            ];

          shellHook = ''
            echo "CubicLauncher development shell"
            echo "Use 'bun install' to install JS deps"
            echo "Use 'bun run tauri dev' for development"
          '';
        };
      }
    );
}
