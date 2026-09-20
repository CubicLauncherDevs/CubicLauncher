<p align="center">
  <img src="static/images/cubic-white.svg" width="120" alt="Logo de CubicLauncher" />
</p>

<h1 align="center">CubicLauncher</h1>

<p align="center">
  Launcher de código abierto para Minecraft: Java Edition.<br />
  Gestión de instancias, versiones y contenido para Windows, Linux y macOS.
</p>

<p align="center">
  <a href="https://github.com/CubicLauncherDevs/CubicLauncher/actions/workflows/ci.yml"><img src="https://github.com/CubicLauncherDevs/CubicLauncher/actions/workflows/ci.yml/badge.svg?branch=develop" alt="Estado de integración continua" /></a>
  <a href="https://github.com/CubicLauncherDevs/CubicLauncher/releases/latest"><img src="https://img.shields.io/github/v/release/CubicLauncherDevs/CubicLauncher" alt="Última versión estable" /></a>
  <a href="https://github.com/CubicLauncherDevs/CubicLauncher/releases"><img src="https://img.shields.io/github/downloads/CubicLauncherDevs/CubicLauncher/total" alt="Descargas en GitHub" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/Licencia-GPL--3.0-blue" alt="Licencia GPL-3.0" /></a>
</p>

<p align="center">
  <a href="https://cubiclauncher.org">Sitio web</a> ·
  <a href="https://github.com/CubicLauncherDevs/CubicLauncher/releases">Descargas</a> ·
  <a href="https://github.com/CubicLauncherDevs/CubicLauncher/issues">Reportar un problema</a> ·
  <a href="CONTRIBUTING.md">Contribuir</a>
</p>

## Acerca del proyecto

CubicLauncher permite administrar instalaciones independientes de Minecraft desde una aplicación de escritorio. Cada instancia dispone de su propia configuración, versión del juego y contenido, lo que facilita mantener entornos Vanilla y con mods en un mismo lugar.

La aplicación utiliza **Tauri v2**, **Rust** y **SvelteKit**. El código fuente, las herramientas de desarrollo y las instrucciones de compilación están disponibles en este repositorio.

## Características

- **Instancias independientes:** creación, configuración, importación y exportación de instalaciones de Minecraft.
- **Migración de launchers:** copia directa de perfiles del launcher oficial e instancias de MultiMC (Forks), incluyendo PolyMC y Prism Launcher.
- **Versiones y loaders:** soporte para Vanilla, Fabric, Forge, Quilt y NeoForge.
- **Cuentas:** autenticación con Microsoft y Yggdrasil, además de perfiles sin conexión.
- **Contenido:** búsqueda e instalación de mods, paquetes de recursos y shaders mediante Modrinth y CurseForge; importación de modpacks.
- **Mundos y servidores:** administración de mundos locales y listas de servidores por instancia, con consultas de estado y conexión al juego.
- **Configuración de ejecución:** selección de Java, asignación de memoria y consola de registros.
- **Personalización:** temas, idiomas e integración con Discord Rich Presence.

## Instalación

### Descargas

Las versiones publicadas están disponibles en [GitHub Releases](https://github.com/CubicLauncherDevs/CubicLauncher/releases). Cada publicación incluye sus notas de cambios y los archivos de instalación correspondientes.

| Plataforma | Distribución                                                 |
| ---------- | ------------------------------------------------------------ |
| Windows    | Instalador NSIS (`.exe`).                                    |
| Linux      | AppImage y paquetes `.deb` y `.rpm`.                         |
| macOS      | Imagen `.dmg`, con compilaciones para Apple Silicon e Intel. |

### Arch Linux

El paquete [`cubiclauncher`](https://aur.archlinux.org/packages/cubiclauncher) está disponible en el AUR. Puede instalarse con un asistente como `yay`:

```bash
yay -S cubiclauncher
```

También se incluye un [PKGBUILD](dist/arch/PKGBUILD) para la compilación local. En Arch Linux se recomienda utilizar este empaquetado, ya que los binarios generados en Ubuntu pueden no ser compatibles con las versiones de las bibliotecas del sistema.

### Nix y NixOS

Con las funciones `flakes` y `nix-command` habilitadas:

```bash
# Instalar en el perfil de usuario
nix profile add github:CubicLauncherDevs/CubicLauncher

# Ejecutar sin instalar en el perfil
nix run github:CubicLauncherDevs/CubicLauncher
```

El flake define paquetes para `x86_64-linux`, `aarch64-linux` y `aarch64-darwin`. Los detalles de empaquetado, configuración y solución de problemas se documentan en la [guía de Nix](dist/nix/README.md).

La disponibilidad en otras distribuciones puede consultarse en [Repology](https://repology.org/project/cubiclauncher/versions).

## Migrar desde otro launcher

1. Abre **Crear instancia → Local → Migrar**.
2. Elige **Launcher oficial** o **MultiMC (Forks)** y pulsa **Buscar instalaciones**.
3. Para una instalación portable o una ruta personalizada, usa **Elegir carpeta**:
   - Oficial: la carpeta `.minecraft` que contiene `launcher_profiles.json`.
   - MultiMC, PolyMC o Prism: la carpeta del launcher, `instances` o una instancia individual.
4. Selecciona las instancias, revisa sus versiones y ajusta los nombres de destino si lo necesitas.
5. Pulsa **Migrar**. El asistente muestra el progreso y un resumen por instancia.

Se copian mundos, mods, configuraciones, packs, capturas, servidores y opciones del juego. Los originales se conservan. Las versiones y loaders se instalan desde la cola de descargas de CubicLauncher. Las cuentas se configuran desde CubicLauncher.

Los perfiles oficiales que comparten una carpeta de juego se indican en la vista previa; cada instancia seleccionada recibe su propia copia. Fabric incluye sus mappings Intermediary y NeoForge se reconoce también por sus argumentos de arranque. Los parches de loaders conocidos de MultiMC permiten recuperar sus versiones; la vista previa indica que se reinstalarán los componentes publicados, sin copiar cambios locales en bibliotecas o argumentos. Los componentes desconocidos y los parches del juego base aparecen como no compatibles. No se admiten enlaces simbólicos dentro de los datos copiados.

Puedes cancelar la operación durante la copia. Se eliminan los datos temporales y la instancia incompleta, conservando las instancias que ya terminaron. Cerrar el asistente o cambiar de pestaña también cancela la migración pendiente.

## Desarrollo

### Requisitos

- [Git](https://git-scm.com/) para obtener el código fuente.
- [Bun](https://bun.sh/) para instalar las dependencias y ejecutar los scripts del proyecto.
- [Node.js](https://nodejs.org/) en una versión LTS vigente.
- [Rust](https://www.rust-lang.org/tools/install) estable, con soporte para la edición 2024, junto con Cargo, rustfmt y Clippy.
- Las [dependencias del sistema de Tauri v2](https://v2.tauri.app/start/prerequisites/) correspondientes a la plataforma de desarrollo.

La CLI de Tauri forma parte de las dependencias del proyecto y se instala con `bun install`.

### Entorno local

```bash
git clone https://github.com/CubicLauncherDevs/CubicLauncher.git
cd CubicLauncher
bun install --frozen-lockfile
bun run tauri dev
```

`bun run tauri dev` inicia la aplicación de escritorio y el servidor de desarrollo del frontend. Para trabajar únicamente en la interfaz, `bun run dev` inicia Vite; las funciones que dependen del backend requieren el entorno de Tauri.

Con Nix, se puede preparar el entorno desde la raíz del repositorio mediante `nix develop` antes de ejecutar los comandos anteriores de instalación de dependencias y desarrollo.

### Comandos principales

Todos los comandos se ejecutan desde la raíz del repositorio.

| Comando                                   | Descripción                                                  |
| ----------------------------------------- | ------------------------------------------------------------ |
| `bun run tauri dev`                       | Inicia la aplicación de escritorio en modo de desarrollo.    |
| `bun run dev`                             | Inicia el servidor Vite del frontend.                        |
| `bun run tauri build`                     | Compila y empaqueta la aplicación para la plataforma actual. |
| `bun run build`                           | Genera la compilación de producción del frontend.            |
| `bun run lint`                            | Ejecuta ESLint sobre el frontend.                            |
| `bun run check`                           | Verifica tipos y diagnósticos de Svelte.                     |
| `bun run test:frontend`                   | Ejecuta los tests del frontend con Bun.                      |
| `bun run test:rust`                       | Ejecuta los tests y doctests del workspace Rust.             |
| `bun run test:all`                        | Ejecuta las suites del frontend y de Rust.                   |
| `cargo fmt --all --check`                 | Comprueba el formato del código Rust.                        |
| `cargo clippy --workspace -- -D warnings` | Ejecuta el análisis estático del workspace Rust.             |

La [guía de pruebas](TESTING.md) describe la organización de las suites, los requisitos de Chromium, las mediciones de rendimiento y los flujos de verificación manual.

## Arquitectura

El frontend se comunica con el backend mediante comandos IPC de Tauri. El backend gestiona las operaciones nativas y publica eventos que actualizan el estado de la interfaz.

| Componente               | Tecnologías                                           |
| ------------------------ | ----------------------------------------------------- |
| Aplicación de escritorio | Tauri v2 y WebView del sistema.                       |
| Interfaz                 | Svelte 5, SvelteKit y TypeScript.                     |
| Backend                  | Rust, Tokio y un workspace de crates internos.        |
| Integraciones            | Microsoft, Yggdrasil, Modrinth, CurseForge y Discord. |

### Estructura del repositorio

```text
src/                  Interfaz, estado y servicios del frontend
src-tauri/            Aplicación Tauri, comandos y servicios nativos
  src/tests/          Tests unitarios del backend de la aplicación
crates/               Bibliotecas internas del workspace Rust
  <crate>/src/tests/  Tests unitarios de cada biblioteca con suites propias
tests/frontend/       Tests del frontend agrupados por área
static/               Recursos estáticos de la interfaz
dist/                 Archivos de empaquetado para distribuciones
.github/              Integración continua y flujos de publicación
```

## Contribuir

Se aceptan contribuciones de código, documentación, traducciones y reportes de errores. La [guía de contribución](CONTRIBUTING.md) detalla las convenciones del proyecto, las comprobaciones previas a una pull request y el proceso de colaboración.

- **Errores y propuestas:** abrir un [issue](https://github.com/CubicLauncherDevs/CubicLauncher/issues) con una descripción del problema o del cambio sugerido. Para errores, incluir la versión del launcher, el sistema operativo y los pasos para reproducirlos.
- **Cambios de código:** seguir las convenciones y ejecutar las comprobaciones indicadas en [CONTRIBUTING.md](CONTRIBUTING.md) y [TESTING.md](TESTING.md).
- **Traducciones:** consultar las instrucciones de internacionalización en la guía de contribución y el repositorio [Translations](https://github.com/CubicLauncherDevs/Translations).

La participación en el proyecto se rige por el [Código de conducta](CODE_OF_CONDUCT.md).

## Comunidad y recursos

- [Sitio web](https://cubiclauncher.org): información general del proyecto.
- [Discord](https://discord.gg/3xPwpUdPWT): comunidad y conversaciones sobre el launcher.
- [Reddit](https://www.reddit.com/r/CubicLauncher/): publicaciones de la comunidad.
- [Estado de los servicios](https://dev.cubiclauncher.org/): disponibilidad de las API del proyecto.
- [Estado de dependencias Rust](https://deps.rs/repo/github/cubiclauncherdevs/cubiclauncher?path=src-tauri): informe de dependencias del backend.
- [Versionado y publicaciones](VERSIONING.md): documentación del proceso de versiones.

## Contribuyentes

CubicLauncher se desarrolla gracias a las personas que aportan código, documentación y mejoras al proyecto. La lista de contribuyentes de GitHub puede consultarse a continuación.

<a href="https://github.com/CubicLauncherDevs/CubicLauncher/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=CubicLauncherDevs/CubicLauncher" alt="Contribuyentes de CubicLauncher en GitHub" />
</a>

## Licencia

CubicLauncher se distribuye bajo la licencia [GNU General Public License v3.0](LICENSE), identificada como `GPL-3.0-only` en los metadatos del proyecto.
