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

rustPlatform.buildRustPackage rec {
  pname = "cubiclauncher";
  version = "34.0.1";

  src = lib.cleanSource ./../..;

  cargoHash = "sha256-RXqSVsXae6KUeo6UivIcSF6RpMle9O3Y9GJ+9VU84Xk=";

  nodeModules = stdenv.mkDerivation {
    pname = "${pname}-node_modules";
    inherit version;
    # Dependency installation must depend only on the manifest and its lockfile.
    src = lib.fileset.toSource {
      root = ./../..;
      fileset = lib.fileset.unions [ ../../package.json ../../bun.lock ];
    };

    nativeBuildInputs = [
      bun
      writableTmpDirAsHomeHook
    ];

    dontConfigure = true;
    dontFixup = true;

    buildPhase = ''
      runHook preBuild
      test -f bun.lock || { echo "bun.lock is required for reproducible dependency hashes" >&2; exit 1; }
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
      x86_64-linux = "sha256-ls6BV3qHMB0LVbWk2j2oSR4IXZ17IaAWyMi4JH2JZiY=";
      aarch64-linux = "sha256-tmeeXF9mtcrKEKRM5JTgZBf0+Rzs8Nv73dH7B5Fwtv8=";
      aarch64-darwin = "sha256-UPe0OJPWc026U8H1/PrH8K+S3KuogWD5xXz5hrhPAm8=";
    }.${stdenv.hostPlatform.system} or (throw "Unsupported system ${stdenv.hostPlatform.system}");
  };

  postPatch = ''
    cp -r ${nodeModules}/node_modules .
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
}
