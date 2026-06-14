{
  lib,
  stdenv,
  rustPlatform,
  bun,
  pkg-config,
  copyDesktopItems,
  # Linux
  gtk3,
  webkitgtk_4_1,
  libsoup_3,
  openssl,
  librsvg,
  glib-networking,
  gsettings-desktop-schemas,
  # Darwin
  darwin,
  libiconv,
}:

let
  info = builtins.fromJSON (builtins.readFile ../../package.json);
  version = info.version;
in

rustPlatform.buildRustPackage {
  pname = "cubiclauncher";
  inherit version;

  src = ../../.;

  cargoLock.lockFile = ../../Cargo.lock;
  cargoBuildFlags = [ "--package cubiclauncher" ];

  preBuild = ''
    bun install --frozen-lockfile
    bun run build
  '';

  nativeBuildInputs =
    [
      bun
      pkg-config
      libiconv
    ]
    ++ lib.optionals stdenv.isLinux [ copyDesktopItems ];

  buildInputs =
    lib.optionals stdenv.isLinux [
      gtk3
      webkitgtk_4_1
      libsoup_3
      openssl
      librsvg
      glib-networking
      gsettings-desktop-schemas
    ]
    ++ lib.optionals stdenv.isDarwin [
      darwin.apple_sdk.frameworks.AppKit
      darwin.apple_sdk.frameworks.WebKit
      darwin.apple_sdk.frameworks.Foundation
      darwin.apple_sdk.frameworks.SystemConfiguration
      darwin.apple_sdk.frameworks.Security
    ];

  installPhase = ''
    runHook preInstall

    install -Dm755 target/release/cubiclauncher "$out/bin/cubiclauncher"

    if [[ "$(uname -s)" == "Linux" ]]; then
      install -Dm644 src-tauri/icons/32x32.png "$out/share/icons/hicolor/32x32/apps/cubiclauncher.png"
      install -Dm644 src-tauri/icons/128x128.png "$out/share/icons/hicolor/128x128/apps/cubiclauncher.png"
      install -Dm644 src-tauri/icons/128x128@2x.png "$out/share/icons/hicolor/256x256/apps/cubiclauncher.png"
      install -Dm644 src-tauri/icons/icon.png "$out/share/icons/hicolor/512x512/apps/cubiclauncher.png"

      mkdir -p "$out/share/applications"
      cat > "$out/share/applications/cubiclauncher.desktop" << EOF
    [Desktop Entry]
    Type=Application
    Name=CubicLauncher
    Comment=Open source Minecraft launcher
    Exec=$out/bin/cubiclauncher
    Icon=cubiclauncher
    Terminal=false
    Categories=Game;
    StartupWMClass=cubiclauncher
    EOF
    fi

    install -Dm644 LICENSE "$out/share/licenses/$pname/LICENSE"

    runHook postInstall
  '';

  meta = {
    description = "Open-source Minecraft launcher built with Tauri + SvelteKit";
    homepage = "https://github.com/CubicLauncher/CubicLauncher";
    license = lib.licenses.gpl3Only;
    maintainers = with lib.maintainers; [ ];
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
