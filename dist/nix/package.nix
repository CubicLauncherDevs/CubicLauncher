{ lib
, stdenv
, rustPlatform
, bun
, cargo-tauri
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
  version = "32.0.0";

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
        x86_64-linux = "sha256-jg3N+04W6Ry8mXxJyWzL9NY2ug5qiU52A7Rqr3GWnxs=";
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
    desktop-file-utils
    nodejs
    pkg-config
    wrapGAppsHook4
  ];

  buildInputs = [
    glib-networking
    gtk3
    libsoup_3
    openssl
    webkitgtk_4_1
  ];

  doCheck = false;

  postInstall = ''
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
    platforms = lib.platforms.linux;
  };
})
