# Nix / NixOS — CubicLauncher

Esta carpeta contiene el empaquetado de CubicLauncher para Nix y NixOS.

## Archivo generado

- `flake.nix` — Flake raíz del repositorio. Expone el paquete, el devShell,
  el formateador y un check básico.
- `package.nix` — Derivación con `cargo-tauri.hook` que compila el frontend,
  el binario de Tauri y empaqueta el `.deb` para extraerlo en `$out`.

## Requisitos

- Nix con `flakes` y `nix-command` habilitados.
- Sistema soportado actualmente: `x86_64-linux`.

## Comandos útiles

### Instalar el launcher

```bash
nix profile install github:CubicLauncherDevs/CubicLauncher
```

Desde el repositorio local:

```bash
nix profile install .
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

### Build local

```bash
nix build .
ls -la result/bin
```

## Soportar otras arquitecturas

Para agregar `aarch64-linux` (u otra plataforma Linux) hay que:

1. Añadir el sistema a `supportedSystems` en `flake.nix`.
2. Ejecutar el build en esa arquitectura para obtener el hash de
   `nodeModules` (porque `bun install` descarga binarios nativos opcionales).
3. Agregar el hash por sistema en `dist/nix/package.nix`.

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
