# Nix / NixOS — CubicLauncher

Esta carpeta contiene el empaquetado de CubicLauncher para Nix y NixOS.

## Archivo generado

- `flake.nix` — Flake raíz del repositorio. Expone el paquete, el devShell,
  el formateador y un check básico.
- `flake.lock` — Bloqueo de las dependencias del flake. Debe mantenerse en Git
  para permitir instalaciones reproducibles desde GitHub.
- `package.nix` — Derivación con `cargo-tauri.hook` que compila el frontend,
  el binario de Tauri y empaqueta el `.deb` para extraerlo en `$out`.

## Requisitos

- Nix con `flakes` y `nix-command` habilitados.
- Sistemas soportados: `x86_64-linux`, `aarch64-linux` y `aarch64-darwin`.

## Comandos útiles

### Instalar el launcher

```bash
nix profile add github:CubicLauncherDevs/CubicLauncher
```

Desde el repositorio local:

```bash
nix profile add .
```

### Probar sin instalar

```bash
nix run github:CubicLauncherDevs/CubicLauncher
```

### Entorno de desarrollo

```bash
nix develop
bun install
bun run tauri dev
```

En Linux, el devShell configura `XDG_DATA_DIRS` con los esquemas de GTK3 y
`gsettings-desktop-schemas`, conservando las rutas existentes. También fija
`GIO_MODULE_DIR` y añade el backend TLS de `glib-networking` a
`GIO_EXTRA_MODULES` para que WebKitGTK pueda cargar recursos HTTPS.

En Linux con controladores Mesa (por ejemplo, AMD e Intel), el devShell usa
Mesa, EGL, GBM y Wayland de la misma revisión de Nixpkgs que la glibc del
compilador, fijada en `flake.lock`. También configura las rutas de carga para
evitar que EGL seleccione el Mesa de `/run/opengl-driver`, que puede requerir
una glibc más nueva y terminar con `EGL_BAD_PARAMETER`.

Al entrar al entorno se muestran las versiones de Mesa y glibc. Después de
cambiar el flake, salir del shell anterior y volver a ejecutar `nix develop`.
Las variables explícitas `__EGL_VENDOR_LIBRARY_FILENAMES`,
`LIBGL_DRIVERS_PATH` y `GBM_BACKENDS_PATH` se respetan; quitarlas antes de
entrar si se quiere usar los valores del flake.

Con NVIDIA propietario se conservan los controladores del sistema, que deben
coincidir con el módulo del kernel. Si requieren una glibc más nueva que la
del flake, usar el Nixpkgs del sistema en el entorno de desarrollo:

```bash
nix develop --override-input nixpkgs nixpkgs --no-write-lock-file
```

### Build local

```bash
nix build .
ls -la result/bin
```

## Actualizar el hash de Cargo (`cargoHash`)

`cargoHash` cubre el árbol de dependencias de Rust (`cargoDeps`). Su valor
depende únicamente del `Cargo.lock` **y de la versión de Cargo que provee el
nixpkgs fijado en `flake.lock`**, así que es el mismo para los tres sistemas y
no hace falta un hash por plataforma.

Cada vez que cambie `Cargo.lock` hay que regenerarlo:

```bash
# El build falla a propósito y muestra el hash correcto en la línea `got:`.
nix build --no-link \
  ".#packages.$(nix eval --raw --impure --expr builtins.currentSystem).default.cargoDeps"
```

Luego copiar ese valor en `cargoHash` de `dist/nix/package.nix`.

> **Importante:** generarlo siempre con `nix build .#...`, que respeta el
> `flake.lock` commiteado. Un hash obtenido con el canal de nixpkgs del sistema
> (o con un `flake.lock` distinto, por ejemplo tras un `nix flake update` local
> sin commitear) no coincidirá con CI y el trabajo **Nix Hashes** fallará. Si se
> actualiza `flake.lock`, hay que regenerar este hash en el mismo commit.

## Soportar otras arquitecturas

Para agregar una nueva plataforma hay que:

1. Añadir el sistema a `supportedSystems` en `flake.nix`.
2. Ejecutar el build en esa arquitectura para obtener el hash de
   `nodeModules` (porque `bun install` descarga binarios nativos opcionales).
3. Agregar el hash por sistema en `dist/nix/package.nix`.

> **Nota sobre `x86_64-darwin`:** la rama `nixos-unstable` actual ha dejado de
> dar soporte a Intel Mac. Para ese sistema hace falta apuntar el input
> `nixpkgs` a una rama que todavía lo soporte (por ejemplo `nixpkgs-26.05-darwin`).

## Notas

## Solución de problemas

### `error: the group 'nixbld' specified in 'build-users-group' does not exist`

Esta máquina tiene Nix instalado en modo multi-usuario pero no existe el
grupo `nixbld` que usa el daemon para builds. Para corregirlo, ejecutar como
root el script incluido:

```bash
sudo ./dist/nix/setup-nix-build-users.sh
```

Luego:

```bash
nix run .
# o
nix develop
```

## Notas

- El build usa `bun.lock` / `package-lock.json` existentes. No es necesario
  regenerar `package-lock.json` para Nix.
- Los artefactos del auto-actualizador se desactivan durante el empaquetado,
  ya que el launcher debe actualizarse a través de Nix.
