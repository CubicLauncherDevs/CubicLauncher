{ lib
, stdenv
, rustPlatform
, apple-sdk ? null
, bun
, cargo-tauri
, darwin ? null
, desktop-file-utils
, glib-networking
, gtk3
, libsoup_3
, nodejs
, openssl
, pkg-config
, webkitgtk_4_1
, writableTmpDirAsHomeHook
, wrapGAppsHook4
,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "cubiclauncher";
  version = "33.0.0";

  src = lib.cleanSource ./../..;

  cargoHash = "sha256-27lecVNwif4GwegmTG/VEItYzIKvAK0wRE4R8YD/jnQ=";

  nodeModules = stdenv.mkDerivation {
    pname = "${finalAttrs.pname}-node_modules";
    inherit (finalAttrs) version src;

    nativeBuildInputs = [
      bun
      writableTmpDirAsHomeHook
    ];

    dontConfigure = true;
    dontFixup = true;

    buildPhase = ''
      runHook preBuild
      bun install --frozen-lockfile --allow-scripts --no-progress
      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall
      mkdir -p $out
      cp -r node_modules $out/node_modules
      runHook postInstall
    '';

    outputHashMode = "recursive";
    outputHashAlgo = "sha256";
    outputHash =
      {
        x86_64-linux = "sha256-vDlD6oXQsoTDz4wY74HRpIlxLMQOYAKejveKGrz7Guk=";
        aarch64-linux = "sha256-U3vIRaHjKZgJyg7A9GfbeMDxM1P6vlYRYw5/XUhQUZ8=";
        aarch64-darwin = "sha256-Tkp+mIt2lCUg4Q5RF49UDfl9zBNIAyKduoxsLARc2b4=";
      }.${stdenv.hostPlatform.system} or (throw "Unsupported system ${stdenv.hostPlatform.system}");
  };

  postPatch = ''
    cp -r ${finalAttrs.nodeModules}/node_modules .
    chmod -R +w node_modules
    patchShebangs --build node_modules

    # El launcher se actualiza a través de Nix (o manualmente), así que no
    # generamos artefactos del auto-actualizador durante el build.
    substituteInPlace src-tauri/tauri.conf.json \
      --replace-fail '"createUpdaterArtifacts": true' \
                  '"createUpdaterArtifacts": false'
  '';

  nativeBuildInputs = [
    bun
    cargo-tauri.hook
    nodejs
    pkg-config
  ]
  ++ lib.optionals stdenv.hostPlatform.isLinux [
    desktop-file-utils
    wrapGAppsHook4
  ];

  buildInputs = [
    openssl
  ]
  ++ lib.optionals stdenv.hostPlatform.isLinux [
    glib-networking
    gtk3
    libsoup_3
    webkitgtk_4_1
  ]
  ++ lib.optionals (stdenv.hostPlatform.isDarwin && apple-sdk != null) [ apple-sdk ]
  ++ lib.optionals (stdenv.hostPlatform.isDarwin && apple-sdk == null && darwin != null) (
    with darwin.apple_sdk.frameworks; [ WebKit Cocoa CoreFoundation Security ]
  );

  doCheck = false;

  postInstall = lib.optionalString stdenv.hostPlatform.isLinux ''
    if [ -f "$out/share/applications/cubiclauncher.desktop" ]; then
      ${lib.getExe' desktop-file-utils "desktop-file-edit"} \
        --set-key Exec --set-value "$out/bin/cubiclauncher" \
        "$out/share/applications/cubiclauncher.desktop"
    fi
  '';

  meta = {
    description = "Launcher de Minecraft de código abierto construido con Tauri";
    homepage = "https://github.com/CubicLauncherDevs/CubicLauncher";
    license = lib.licenses.gpl3Only;
    maintainers = [ ];
    mainProgram = "cubiclauncher";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
})
