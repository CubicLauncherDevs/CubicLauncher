<p align="center">
  <img src="oldicon.png" width="120" alt="Logo de CubicLauncher" />
</p>

<h1 align="center">CubicLauncher</h1>

CubicLauncher es un launcher de código abierto para Minecraft: Java Edition. Está pensado para tener varias instalaciones del juego separadas, cada una con su propia versión, configuración y contenido, sin que se pisen entre sí.

Está hecho con Tauri v2, Rust y SvelteKit, y funciona en Windows, Linux y macOS.

[![CI](https://github.com/CubicLauncherDevs/CubicLauncher/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/CubicLauncherDevs/CubicLauncher/actions/workflows/ci.yml)
[![Última versión](https://img.shields.io/github/v/release/CubicLauncherDevs/CubicLauncher)](https://github.com/CubicLauncherDevs/CubicLauncher/releases/latest)
[![Descargas](https://img.shields.io/github/downloads/CubicLauncherDevs/CubicLauncher/total)](https://github.com/CubicLauncherDevs/CubicLauncher/releases)
[![Licencia](https://img.shields.io/badge/Licencia-GPL--3.0-blue)](LICENSE)

Sitio web: <https://cubiclauncher.org>

# Instalación

- Las versiones estables están en [GitHub Releases](https://github.com/CubicLauncherDevs/CubicLauncher/releases).
- Estado de las últimas compilaciones: <https://github.com/CubicLauncherDevs/CubicLauncher/actions>

| Plataforma | Archivo                                                  |
| ---------- | -------------------------------------------------------- |
| Windows    | Instalador NSIS (`.exe`)                                 |
| Linux      | AppImage, `.deb` y `.rpm`                                |
| macOS      | `.dmg` (Apple Silicon e Intel)                           |

## Arch Linux

El paquete [`cubiclauncher`](https://aur.archlinux.org/packages/cubiclauncher) está en el AUR:

```bash
yay -S cubiclauncher
```

También hay un [PKGBUILD](dist/arch/PKGBUILD) si prefieres compilarlo localmente. En Arch conviene usar este paquete en lugar del binario de las releases, porque este último se compila en Ubuntu y puede no ser compatible con las bibliotecas de tu sistema.

## Nix y NixOS

Con `flakes` y `nix-command` activados, configura la caché pública de
[Cachix](https://cubiclauncher.cachix.org) para descargar los binarios publicados
para Linux x86_64 y ARM64:

```bash
nix run nixpkgs#cachix -- use cubiclauncher
```

En NixOS, incorpora la configuración generada siguiendo las indicaciones del
comando y aplica los cambios con `sudo nixos-rebuild switch` antes de instalar.
Si tu configuración usa un flake en Git, agrega también los archivos generados
al repositorio con `git add` para que Nix pueda leerlos.

Elige la versión estable o la beta:

```bash
# Última versión estable publicada en la caché
nix profile add github:CubicLauncherDevs/CubicLauncher/latest-release

# Alternativa: última beta publicada en la caché
nix profile add github:CubicLauncherDevs/CubicLauncher/latest-beta

# Ejecutar sin instalar
nix run github:CubicLauncherDevs/CubicLauncher/latest-release
```

También puedes instalar un tag específico reemplazando `TAG` por el nombre
exacto de la versión en [Releases](https://github.com/CubicLauncherDevs/CubicLauncher/releases):

```bash
nix profile add github:CubicLauncherDevs/CubicLauncher/TAG
```

Nix usa Cachix automáticamente si está disponible el build de esa revisión y
arquitectura; de lo contrario, compila localmente. Los canales `latest-release`
y `latest-beta` se actualizan cuando termina correctamente su publicación en
la caché; si un canal aún no se publicó, su tag puede no existir.

El flake ofrece paquetes para `x86_64-linux`, `aarch64-linux` y `aarch64-darwin`;
la caché del proyecto se publica solo para Linux. Más detalles y comandos de
actualización en la [guía de Nix](dist/nix/README.md).

Para ver en qué otras distribuciones está disponible, revisa [Repology](https://repology.org/project/cubiclauncher/versions).

# Qué puede hacer

- Crear, configurar, importar y exportar instancias.
- Instalar Vanilla, Fabric, Forge, Quilt y NeoForge.
- Iniciar sesión con Microsoft, Yggdrasil o jugar sin conexión.
- Buscar e instalar mods, resource packs y shaders desde Modrinth y CurseForge, e importar modpacks.
- Administrar mundos y listas de servidores por instancia.
- Elegir la versión de Java y su proveedor (Azul Zulu, Eclipse Temurin, BellSoft Liberica, GraalVM, Amazon Corretto, Microsoft e IBM Semeru), ajustar la memoria y ver la consola de registros.
- Cambiar tema e idioma, y mostrar la actividad en Discord.

## Migrar desde otro launcher

Puedes copiar tus instancias desde el launcher oficial o desde MultiMC y sus forks (PolyMC, Prism Launcher, PineconeMC). Los originales no se tocan.

1. Ve a **Crear instancia → Local → Migrar**.
2. Elige el launcher de origen y pulsa **Buscar instalaciones**. Si usas una instalación portable o una ruta personalizada, usa **Elegir carpeta**.
3. Selecciona las instancias que quieras y pulsa **Migrar**.

Se copian mundos, mods, configuraciones, packs, capturas, servidores y opciones. Las versiones y loaders se instalan después desde la cola de descargas, y las cuentas hay que configurarlas de nuevo en CubicLauncher. Los componentes desconocidos y los parches del juego base no son compatibles, y no se admiten enlaces simbólicos dentro de los datos copiados.

# Ayuda y soporte

Si necesitas ayuda, puedes abrir un [issue](https://github.com/CubicLauncherDevs/CubicLauncher/issues). Si no es un error, probablemente sea más rápido preguntar en Discord.

[![Discord](https://img.shields.io/badge/Discord-CubicLauncher-5865F2?logo=discord&logoColor=white)](https://discord.gg/3xPwpUdPWT)

También tenemos un subreddit donde puedes dejar tus problemas y sugerencias:

[![Reddit](https://img.shields.io/badge/Reddit-r%2FCubicLauncher-FF4500?logo=reddit&logoColor=white)](https://www.reddit.com/r/CubicLauncher/)

Al reportar un error, incluye la versión del launcher, tu sistema operativo y los pasos para reproducirlo.

# Desarrollo

Si quieres contribuir, lee antes [CONTRIBUTING.md](CONTRIBUTING.md) y [TESTING.md](TESTING.md). Para charlar con el equipo, pasa por el Discord.

## Compilar

Necesitas:

- [Git](https://git-scm.com/)
- [Bun](https://bun.sh/)
- [Node.js](https://nodejs.org/) (una versión LTS)
- [Rust](https://www.rust-lang.org/tools/install) estable con edición 2024, junto con Cargo, rustfmt y Clippy
- Las [dependencias del sistema de Tauri v2](https://v2.tauri.app/start/prerequisites/) para tu plataforma

```bash
git clone https://github.com/CubicLauncherDevs/CubicLauncher.git
cd CubicLauncher
bun install --frozen-lockfile
bun run tauri dev
```

`bun run dev` levanta solo el frontend con Vite, pero lo que depende del backend no funcionará fuera de Tauri. Si usas Nix, corre `nix develop` primero.

| Comando                                   | Qué hace                                      |
| ----------------------------------------- | --------------------------------------------- |
| `bun run tauri dev`                       | Abre la app en modo desarrollo                |
| `bun run tauri build`                     | Compila y empaqueta para tu plataforma        |
| `bun run lint`                            | ESLint sobre el frontend                      |
| `bun run check`                           | Tipos y diagnósticos de Svelte                |
| `bun run test:all`                        | Tests del frontend y de Rust                  |
| `cargo fmt --all --check`                 | Revisa el formato de Rust                     |
| `cargo clippy --workspace -- -D warnings` | Análisis estático de Rust                     |

## Estructura

```text
src/          Frontend (interfaz, estado y servicios)
src-tauri/    Aplicación Tauri, comandos y servicios nativos
crates/       Bibliotecas internas del workspace de Rust
tests/        Tests del frontend
static/       Recursos estáticos
dist/         Archivos de empaquetado para distribuciones
.github/      CI y flujos de publicación
```

## Traducciones

Las traducciones se mantienen en el repositorio [Translations](https://github.com/CubicLauncherDevs/Translations). Ahí también está la información para colaborar.

## Versiones

El proceso de versionado y publicación está explicado en [VERSIONING.md](VERSIONING.md).

# Licencia

Todo el código del launcher se distribuye bajo la licencia [GPL-3.0-only](LICENSE).

La participación en el proyecto se rige por el [Código de conducta](CODE_OF_CONDUCT.md).
