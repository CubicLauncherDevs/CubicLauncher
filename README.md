<div align="center">
  <img src="static/images/cubic.svg" width="120" alt="CubicLauncher" />
  <h1>CubicLauncher</h1>

[<img src="https://img.shields.io/badge/Licence-GPL--3.0-blue" alt="License">](https://github.com/CubicLauncherDevs/CubicLauncher/blob/main/LICENSE)
[![dependency status](https://deps.rs/repo/github/cubiclauncherdevs/cubiclauncher/status.svg?path=src-tauri)](https://deps.rs/repo/github/cubiclauncherdevs/cubiclauncher?path=src-tauri)
[<img src="https://img.shields.io/discord/1366945511273398342" alt="Discord">](https://discord.gg/3xPwpUdPWT)
[<img src="https://img.shields.io/github/downloads/CubicLauncherdevs/cubiclauncher/total.svg" alt="Downloads">](https://www.cubiclauncher.org/install)
[<img src="https://img.shields.io/website/https/www.cubiclauncher.org" alt="Web">](https://cubiclauncher.org)
</div>

---

**CubicLauncher** es un launcher de Minecraft multiplataforma construido sobre [Tauri v2](https://tauri.app) + [SvelteKit](https://kit.svelte.dev/) (frontend) y [Rust](https://www.rust-lang.org/) (backend nativo). Gestiona instancias aisladas de Minecraft con soporte para múltiples versiones, loaders de mods (Vanilla, Fabric, Forge, NeoForge), autenticación OAuth 2.0 (Microsoft) y Yggdrasil, y un sistema modular de crates Rust con caché binario.

## Stack

| Capa         | Tecnología                                                                          |
| ------------ | ----------------------------------------------------------------------------------- |
| Shell nativo | [Tauri v2](https://tauri.app) + Rust                                                |
| Frontend     | [Svelte 5](https://svelte.dev/) + [SvelteKit](https://kit.svelte.dev/) + TypeScript |
| Bundler      | [Vite 6](https://vite.dev/)                                                         |
| Backend      | Rust (edition 2024)                                                                 |
| Runtime JS   | [Bun](https://bun.sh/)                                                              |
| Auth         | OAuth 2.0 (Microsoft device-code), Yggdrasil, Cracked                               |
| Addons       | Modrinth + CurseForge API                                                            |
| UI           | Componentes nativos Svelte 5 (sin framework CSS)                                    |

## Paquetes Rust (crates/)

| Crate | Descripción |
| ----- | ----------- |
| **zellkern** | Modelo de datos de Minecraft. Define manifiestos de versiones, resolución de librerías, classpath, perfiles de instalación de Forge, extracción de nativos y el enum `Loader` (Vanilla/Fabric/Forge/NeoForge/Quilt). Crate base del que dependen todos los demás. |
| **aqua** | Motor de descargas y gestor JRE. `DownloadManager` con tipos batch (`MinecraftBatch`, `FabricBatch`, `ForgeBatch`, `GenericBatch`), reporte de progreso, e instalación de JREs vía la API de Azul Zulu. |
| **launchwerk** | Orquestador de lanzamiento. Construye el comando completo (classpaths, JVM args, tokens de auth, env vars, quick-play), spawne el proceso Minecraft, maneja stdout/stderr via broadcast channels, y soporta auto-refresh de tokens Microsoft/Yggdrasil (con `aes-gcm` para cifrado en disco). |
| **cubrinth** | Soporte de modpacks. Parsea archivos `.mrpack` (Modrinth), resuelve dependencias e instala mods en directorios de instancia. |
| **communicator** | Discord Rich Presence. Implementa el protocolo Discord IPC para mostrar estado de juego ("Idle" / "Playing {instance}") en Discord. |
| **ablage** | Caché binario en disco. Formato custom con CRC32, escrituras atómicas (temp+rename+fsync), y un índice ordenado para búsquedas rápidas. Usado para cachear manifiestos de Mojang, versiones de Fabric/Forge y metadata de mods. |

## Arquitectura

El frontend se comunica con el backend mediante **77 Tauri Commands** (IPC). El backend emite eventos via un event bus (`app-event` con 11 tipos de evento) que el frontend consume reactivamente.

### Backend

| Módulo | Función |
| ------ | ------- |
| `commands/` | 12 módulos, ~77 comandos IPC: instance CRUD, launch/kill, mods, screenshots, download, auth, settings, themes, modrinth, forge, discord, java, log window |
| `services/` | Lógica de negocio: `SettingsManager` (auto-save con debounce), `Launcher` (proceso Java), `DownloadQueue` (worker async con eventos de progreso), `InstanceManager` (CRUD + persistencia), `JavaManager` (JREs bundled 8/17/21/25), `AddonManager` (metadata de mods con caché 500 entries) |
| `core/` | Infraestructura: `PathManager` (resolución de directorios `~/.cubic/`), `EventBus` (wrapper de Tauri emit), `HttpClient` (reqwest singleton), `errors/` (jerarquía de errores con serialización JSON para i18n: `{"code":"ERROR_CODE","params":{...}}`) |
| `theme_watcher/` | File watcher (`notify` crate) del directorio de temas del usuario, emite `ThemeChanged` para edición en vivo |

### Frontend

| Módulo | Función |
| ------ | ------- |
| `api/cubicApi.ts` | Wrapper central de IPC. Todas las llamadas `invoke()` con error handling. Búsqueda de mods vía `fetch()` directo a Modrinth/CurseForge APIs (en el browser). |
| `api/launcherService.ts` | Hub de event listeners. Suscribe a `app-event` y actualiza `launcherStore` reactivamente. Helpers: `getActiveUser()`, `saveSettings()`, `killInst()`, `deleteInst()`. |
| `api/themeManager.ts` | Motor de temas. Carga temas vía IPC, aplica CSS custom properties, background images (lazy loading), font-face injection. |
| `state/state.svelte.ts` | Estado global reactivo con `$state` de Svelte 5. Un solo `launcherStore` con: instances, settings, notifications, update state. |
| `types/types.ts` | Tipos TypeScript compartidos: `InstanceDto`, `Settings`, `MinecraftUser`, `AppEvent` (discriminated union que espeja el enum Rust). |
| `i18n/` | Internacionalización: es, en, de, fr. |

### Decisiones clave

- **Errores serializados**: Todos los errores Rust se serializan como `{"code":"...", "params":{...}}` para display i18n en el frontend via `showErrorParsed()`.
- **`ablage` como base de caché**: Formato binario más eficiente que JSON para datasets grandes (manifiestos, versiones, metadata de mods).
- **Auth multi-cuenta**: Microsoft OAuth (device-code flow), Yggdrasil (con authlib-injector), Cracked. Tokens encriptados en disco, auto-refresh antes del launch.
- **Mod search en browser**: Búsqueda de mods se hace directamente en el frontend; la descarga/instalación se delega a Rust via IPC.
- **Forge + Java bump**: Auto-detecta Forge >= 36.2.26 y bumpa Java 8 a Java 17 para compatibilidad con ModLauncher 8.1.3+.
- **Log streaming**: stdout/stderr capturado via `tokio::sync::broadcast`, batcheado a 80ms, almacenado en ring buffers de 5000 líneas por instancia.

## Primeros pasos

### Prerrequisitos

- [Bun](https://bun.sh/) ≥ 1.x
- [Rust](https://www.rust-lang.org/tools/install) ≥ 1.85 (edition 2024)
- [Tauri CLI v2](https://v2.tauri.app/start/prerequisites/)

### Instalación

```bash
git clone https://github.com/CubicLauncher/CubicLauncher.git
cd CubicLauncher
bun install
```

### Scripts

| Comando               | Descripción                                   |
| --------------------- | --------------------------------------------- |
| `bun run dev`         | Servidor de desarrollo Vite (solo frontend)   |
| `bun run build`       | Build de producción del frontend              |
| `bun run check`       | Type-check con `svelte-check`                 |
| `bun run tauri dev`   | Entorno de desarrollo Tauri (frontend + Rust) |
| `bun run tauri build` | Build completo de la aplicación Tauri         |

## Compilación

```bash
bun run tauri build
```

El binario se genera en `src-tauri/target/release/`. Consulta la [documentación oficial](https://www.cubiclauncher.com/docs/main.html#desarrollo/compilacion) para builds específicos por plataforma.

### Arch Linux

Descargá solo el [PKGBUILD](dist/arch/PKGBUILD) y compilá (clona el repo automáticamente):

```bash
mkdir cubiclauncher-build && cd cubiclauncher-build
wget https://raw.githubusercontent.com/CubicLauncher/CubicLauncher/main/dist/arch/PKGBUILD
makepkg -si
```

> ⚠️ **Compilar localmente es obligatorio.** Los binarios de CI (Ubuntu) pueden no ser compatibles con Arch Linux por su modelo rolling release.

> ⚠️ **Inestable** — Revisá `dist/arch/IMPORTANTE.md`.

## Comunidad

| Plataforma | URL                                                            |
| ---------- | -------------------------------------------------------------- |
| Discord    | [https://discord.gg/7VaqSrPukm](https://discord.gg/7VaqSrPukm) |

## Licencia

Distribuido bajo [GNU General Public License v3.0](LICENSE).
