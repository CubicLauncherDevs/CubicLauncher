<p align="center">
  <img src="static/images/cubic.svg" width="120" alt="CubicLauncher" />
  <h1 align="center">CubicLauncher</h1>
</p>

<p align="center">
  CubicLauncher es un launcher de Minecraft multiplataforma construido sobre <a href="https://tauri.app">Tauri v2</a> + <a href="https://kit.svelte.dev/">SvelteKit</a> (frontend) y <a href="https://www.rust-lang.org/">Rust</a> (backend nativo). Gestiona instancias aisladas de Minecraft con soporte para múltiples versiones, loaders de mods (Vanilla, Fabric, Forge, Quilt, NeoForge), autenticación OAuth 2.0 (Microsoft) y Yggdrasil, y un sistema modular de crates Rust con caché binario.
</p>

<p align="center">
  <a href="https://github.com/CubicLauncherDevs/CubicLauncher/blob/main/LICENSE"><img src="https://img.shields.io/badge/Licence-GPL--3.0-blue" alt="License"></a>
  <a href="https://deps.rs/repo/github/cubiclauncherdevs/cubiclauncher?path=src-tauri"><img src="https://deps.rs/repo/github/cubiclauncherdevs/cubiclauncher/status.svg?path=src-tauri" alt="Dependency status"></a>
  <a href="https://discord.gg/3xPwpUdPWT"><img src="https://img.shields.io/github/downloads/cubiclauncherdevs/cubiclauncher/total" alt="Downloads"></a>
  <a href="https://cubiclauncher.org"><img src="https://img.shields.io/website/https/www.cubiclauncher.org" alt="Web"></a>
</p>

---

## Installation

<a href="https://repology.org/project/cubiclauncher/versions">
    <img src="https://repology.org/badge/vertical-allrepos/cubiclauncher.svg" alt="Packaging status" align="right">
</a>

### Prerrequisitos

- [Bun](https://bun.sh/) ≥ 1.x
- [Rust](https://www.rust-lang.org/tools/install) ≥ 1.85 (edition 2024)
- [Tauri CLI v2](https://v2.tauri.app/start/prerequisites/)

```bash
git clone https://github.com/CubicLauncherDevs/CubicLauncher.git
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

## Stack

| Capa         | Tecnología                                                                          |
| ------------ | ----------------------------------------------------------------------------------- |
| Shell nativo | [Tauri v2](https://tauri.app) + Rust                                                |
| Frontend     | [Svelte 5](https://svelte.dev/) + [SvelteKit](https://kit.svelte.dev/) + TypeScript |
| Backend      | Rust (edition 2024)                                                                 |
| Auth         | OAuth 2.0 (Microsoft device-code), Yggdrasil, Cracked                               |
| Addons       | Modrinth + CurseForge API                                                           |

## Arquitectura

El frontend se comunica con el backend mediante **77 Tauri Commands** (IPC). El backend emite eventos via un event bus (`app-event` con 11 tipos de evento) que el frontend consume reactivamente.

Para más detalle ver [CONTRIBUTING.md](CONTRIBUTING.md) y [TESTING.md](TESTING.md).

## Community

[![Discord](https://img.shields.io/badge/Discord-7289DA?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/7VaqSrPukm)

## Building

```bash
bun run tauri build
```

### Arch Linux

CubicLauncher está disponible en el AUR como [`cubiclauncher`](https://aur.archlinux.org/packages/cubiclauncher).

```bash
yay -S cubiclauncher
# o
paru -S cubiclauncher
```

También podés descargar el [PKGBUILD](dist/arch/PKGBUILD) y compilar manualmente:

```bash
mkdir cubiclauncher-build && cd cubiclauncher-build
wget https://raw.githubusercontent.com/CubicLauncherDevs/CubicLauncher/main/dist/arch/PKGBUILD
makepkg -si
```

> ⚠️ Compilar localmente es obligatorio. Los binarios de CI (Ubuntu) pueden no ser compatibles con Arch Linux por su modelo rolling release.

## License

Distribuido bajo [GNU General Public License v3.0](LICENSE).
