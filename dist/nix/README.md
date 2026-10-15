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

En Linux con controladores Mesa (por ejemplo, AMD e Intel), el devShell y el paquete instalado usan
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

El paquete instalado configura EGL igual que el devShell y conserva los
controladores del sistema para NVIDIA. También aplica el ajuste de WebKitGTK
para NVIDIA con Wayland; `CUBICLAUNCHER_WEBKIT_SOFTWARE=1` lo fuerza y
`CUBICLAUNCHER_WEBKIT_SOFTWARE=0` desactiva la detección automática.

Para verificar una corrección de EGL, ejecutar `./result/bin/cubiclauncher`
fuera de `nix develop` después del build. Instalar con `nix profile add .`.
Si ya existe una entrada del repositorio local, consultar su nombre con
`nix profile list` y actualizarla con `nix profile upgrade <nombre>`.

```bash
nix build .
ls -la result/bin
```

## Distribuir binarios con Cachix

El workflow `.github/workflows/nix-cache.yml` compila el paquete completo para
Linux x86_64 y Linux ARM64. Se ejecuta al publicar releases,
al terminar el workflow Publish Release y manualmente desde Actions. Publica el launcher y sus
dependencias; también guarda resultados intermedios para próximos builds.

### Activación inicial del mantenedor

1. Crear una **caché pública** en [Cachix](https://app.cachix.org), con firma
   administrada por Cachix, y generar un token de escritura para esa caché.
2. En GitHub → Settings → Secrets and variables → Actions, guardar el token
   como secreto `CACHIX_AUTH_TOKEN`. No incluirlo en archivos del repositorio.
3. Crear la variable de repositorio `CACHIX_CACHE_NAME` con el nombre real de
   la caché, por ejemplo `cubiclauncher` si se ha registrado con ese nombre.
4. Subir el workflow y ejecutarlo desde Actions → Nix Binary Cache. Sin la
   variable de nombre o el token, el workflow falla con un mensaje explicativo.

No se publica desde pull requests. El primer build puede tardar; los siguientes
reutilizan los resultados disponibles. Cada arquitectura se publica por separado.

Se seleccionan dos canales según la fecha de publicación en GitHub:

- `latest-release`: última release pública que no sea prerelease.
- `latest-beta`: última prerelease pública (incluye las versiones DEV).

Los borradores se excluyen. Después de compilar y subir ambos canales para las
tres plataformas, se actualizan los tags móviles `latest-release` y `latest-beta`.
Si falla un build, los tags conservan sus destinos anteriores. Si no hay ninguna
beta publicada, ese canal todavía no se crea. Reservar esos dos nombres para el
workflow y permitir su actualización en las reglas de tags del repositorio.

El workflow debe estar en la rama predeterminada. Las releases seleccionadas
deben incluir un flake compilable y la corrección de EGL; el workflow compila
exactamente el código de cada release, sin agregarle cambios de `develop`.

### Instalar usando los binarios publicados

Sustituir `NOMBRE_CACHE` por el nombre configurado por el proyecto:

```bash
nix run nixpkgs#cachix -- use NOMBRE_CACHE

# Elegir uno de los dos canales:
nix profile add github:CubicLauncherDevs/CubicLauncher/latest-release
# o:
nix profile add github:CubicLauncherDevs/CubicLauncher/latest-beta
```

Para actualizar conservando el canal elegido, consultar el nombre con
`nix profile list` y ejecutar `nix profile upgrade --refresh <nombre>`.

En NixOS, seguir las indicaciones de `cachix use` e incorporar la configuración
generada al sistema; aplicar después con `sudo nixos-rebuild switch`.

Para descargar un build concreto, usar la revisión que haya terminado de
publicarse en Actions:

```bash
nix profile add github:CubicLauncherDevs/CubicLauncher/REVISION
```

La caché debe contener el resultado del mismo código, `flake.lock` y plataforma.
Si no existe un resultado coincidente, Nix compila localmente. Los cambios
locales sin publicar también pueden provocar una compilación.

### Subir un build local existente

Después de configurar las credenciales con `cachix authtoken` siguiendo la
[documentación de Cachix](https://docs.cachix.org/getting-started):

```bash
nix build .
nix run nixpkgs#cachix -- push NOMBRE_CACHE ./result
```

Esto permite aprovechar el build local sin esperar al primer build de CI para
esa plataforma. Para que sirva a otros, publicar también la revisión del código
con la que se compiló.

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
