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
      in
      {
        packages = {
          default = pkgs.callPackage ./default.nix { };
          cubiclauncher = pkgs.callPackage ./default.nix { };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            bun
            cargo
            rustc
            pkg-config
            gtk3
            webkitgtk
            libsoup
            openssl
            librsvg
            glib-networking
            gsettings-desktop-schemas
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
