{
  lib,
  stdenv,
  rustPlatform,
  apple-sdk ? null,
  alsa-lib,
  bun,
  cargo-tauri,
  darwin ? null,
  desktop-file-utils,
  glib-networking,
  gtk3,
  libsoup_3,
  libpulseaudio,
  libx11,
  libxcursor,
  libxi,
  libxinerama,
  libxrandr,
  libxxf86vm,
  nodejs,
  openssl,
  pkg-config,
  pipewire,
  webkitgtk_4_1,
  writableTmpDirAsHomeHook,
  wrapGAppsHook4,
}:

rustPlatform.buildRustPackage rec {
  # Identidad y versión del paquete.
  pname = "cubiclauncher";
  version = "36.0.0";

  # Código fuente del proyecto, excluyendo los archivos que Nix
  # considera innecesarios mediante cleanSource.
  src = lib.cleanSource ./../..;

  # Hash de las dependencias de Rust gestionadas por Cargo.
  # Debe actualizarse cuando cambie el conjunto de dependencias.
  cargoHash = "sha256-+fxdDOufduR41HpG8gfRDZCDMm8znqEUcs//Zy072bc=";

  # Derivación independiente para las dependencias del frontend.
  #
  # Separar node_modules del paquete principal permite reutilizar
  # las dependencias cuando cambian archivos ajenos a los manifiestos.
  # El hash de salida garantiza que el resultado coincida con el esperado.
  nodeModules = stdenv.mkDerivation {
    pname = "${pname}-node_modules";
    inherit version;

    # Solo se incluyen los manifiestos necesarios para instalar
    # las dependencias de JavaScript con Bun.
    # Esto evita que otros cambios del repositorio invaliden esta derivación.
    src = lib.fileset.toSource {
      root = ./../..;
      fileset = lib.fileset.unions [
        ../../package.json
        ../../bun.lock
      ];
    };

    # Herramientas necesarias durante la instalación de dependencias.
    nativeBuildInputs = [
      bun
      writableTmpDirAsHomeHook
    ];

    # Esta derivación únicamente prepara node_modules.
    dontConfigure = true;
    dontFixup = true;

    buildPhase = ''
      runHook preBuild

      # Exigir el archivo de bloqueo para mantener una instalación reproducible.
      test -f bun.lock || {
        echo "bun.lock is required for reproducible dependency hashes" >&2
        exit 1
      }

      # Instalar las versiones exactas registradas en bun.lock.
      # Se permiten scripts de instalación por si alguna dependencia los necesita.
      bun install --frozen-lockfile --allow-scripts --no-progress

      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall

      # Exportar las dependencias instaladas como salida de la derivación.
      mkdir -p "$out"
      cp -r node_modules "$out/node_modules"

      runHook postInstall
    '';

    # Calcular el hash sobre el árbol completo de archivos generado.
    outputHashMode = "recursive";
    outputHashAlgo = "sha256";

    # El resultado de las dependencias puede variar según la arquitectura.
    # Se define un hash independiente para cada plataforma compatible.
    outputHash = {
      x86_64-linux = "sha256-iznqcE94oIL7yM8FqX9fO6OHdmR1j8splPOi4fQ3sNI=";
      aarch64-linux = "sha256-GrQ4QZ0bcrDL4kvapQdfxmMMTCjJvuvC31o7tPBwk/8=";
      aarch64-darwin = "sha256-NYewXMyVnQOnvF9537DjXESd83J6uURZMYAIFsq7C/Q=";
    }.${stdenv.hostPlatform.system}
      or (throw "Unsupported system ${stdenv.hostPlatform.system}");
  };

  # Preparar el código fuente antes de compilar la aplicación.
  postPatch = ''
    # Copiar las dependencias del frontend generadas previamente.
    cp -r ${nodeModules}/node_modules .
    chmod -R +w node_modules

    # Ajustar los intérpretes de los scripts de las dependencias
    # para que puedan ejecutarse correctamente durante la compilación.
    patchShebangs --build node_modules

    # CubicLauncher se actualiza mediante Nix o manualmente.
    # Por ello, no es necesario generar artefactos del actualizador
    # automático durante la compilación del paquete.
    substituteInPlace src-tauri/tauri.conf.json \
      --replace-fail '"createUpdaterArtifacts": true' \
                  '"createUpdaterArtifacts": false'
  '';

  # Herramientas necesarias durante la compilación.
  # No todas forman parte de las bibliotecas que necesita la aplicación
  # cuando ya está instalada.
  nativeBuildInputs = [
    bun
    cargo-tauri.hook
    nodejs
    pkg-config
  ]
  ++ lib.optionals stdenv.hostPlatform.isLinux [
    # Herramientas para integrar la aplicación con el escritorio Linux.
    desktop-file-utils
    wrapGAppsHook4
  ];

  # Bibliotecas y frameworks necesarios para compilar y enlazar la aplicación.
  buildInputs = [
    # Soporte para operaciones criptográficas y conexiones TLS.
    openssl
  ]
  ++ lib.optionals stdenv.hostPlatform.isLinux [
    # Dependencias de GTK y WebKit utilizadas por la interfaz de Tauri.
    glib-networking
    gtk3
    libsoup_3
    webkitgtk_4_1
  ]
  ++ lib.optionals (stdenv.hostPlatform.isDarwin && apple-sdk != null) [
    # Utilizar el SDK de Apple proporcionado explícitamente, si existe.
    apple-sdk
  ]
  ++ lib.optionals (
    stdenv.hostPlatform.isDarwin
    && apple-sdk == null
    && darwin != null
  ) (
    # Si no se proporciona apple-sdk, utilizar los frameworks disponibles
    # en el conjunto de paquetes de Darwin.
    with darwin.apple_sdk.frameworks; [
      WebKit
      Cocoa
      CoreFoundation
      Security
    ]
  );

  # Desactivar la fase de pruebas de Cargo para este paquete.
  # Se puede habilitar cuando las pruebas sean compatibles y estables
  # dentro del entorno de compilación de Nix.
  doCheck = false;

  # Minecraft hereda parte del entorno de ejecución del launcher.
  #
  # OpenAL y GLFW pueden cargar bibliotecas de audio y X11 dinámicamente.
  # Estas dependencias no siempre se detectan durante el análisis ELF,
  # por lo que deben estar disponibles mediante LD_LIBRARY_PATH.
  #
  # El wrapper de GTK propaga estas rutas al launcher y a los procesos
  # que este inicia, como el propio Minecraft.
  preFixup = lib.optionalString stdenv.hostPlatform.isLinux ''
    gappsWrapperArgs+=(--prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath [
      alsa-lib
      libpulseaudio
      pipewire
      libx11
      libxcursor
      libxi
      libxinerama
      libxrandr
      libxxf86vm
    ]}")
  '';

  # Corregir la entrada .desktop después de instalar el paquete.
  #
  # Se establece la ruta absoluta del ejecutable instalado por Nix
  # para que los entornos de escritorio puedan iniciar CubicLauncher
  # sin depender del PATH ni de rutas relativas.
  postInstall = lib.optionalString stdenv.hostPlatform.isLinux ''
    if [ -f "$out/share/applications/cubiclauncher.desktop" ]; then
      ${lib.getExe' desktop-file-utils "desktop-file-edit"} \
        --set-key Exec --set-value "$out/bin/cubiclauncher" \
        "$out/share/applications/cubiclauncher.desktop"
    fi
  '';

  # Metadatos utilizados por Nix y por las herramientas de gestión de paquetes.
  meta = {
    # Descripción breve del proyecto.
    description = "Launcher de Minecraft de código abierto construido con Tauri";

    # Repositorio oficial del proyecto.
    homepage = "https://github.com/CubicLauncherDevs/CubicLauncher";

    # Licencia GPL versión 3, sin cláusula de versiones posteriores.
    license = lib.licenses.gpl3Only;

    # Lista de identificadores de los mantenedores reconocidos por Nixpkgs.
    # Puede completarse cuando se asignen los mantenedores del paquete.
    maintainers = [ ];

    # Nombre del ejecutable principal instalado.
    mainProgram = "cubiclauncher";

    # Plataformas en las que se declara que el paquete puede utilizarse.
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
