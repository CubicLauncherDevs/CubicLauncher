# Nix — CubicLauncher

## Build con el builder script (recomendado)

Equivalente al `makepkg -si` de Arch:

```bash
curl -O https://raw.githubusercontent.com/CubicLauncher/CubicLauncher/main/dist/nix/cubiclauncher-builder
bash cubiclauncher-builder
```

Clona el repo desde GitHub y compila con Nix automáticamente.

## Build desde checkout local

```bash
# Opción 1 — nix-build (no requiere flakes)
cd dist/nix && nix-build release.nix

# Opción 2 — flakes
cd dist/nix && nix build --extra-experimental-features 'nix-command flakes'
```

## Build desde GitHub (flakes)

```bash
nix build "github:CubicLauncher/CubicLauncher?dir=dist/nix" \
  --extra-experimental-features 'nix-command flakes'
```

## Shell de desarrollo

```bash
cd dist/nix && nix develop --extra-experimental-features 'nix-command flakes'
# Después: bun install && bun run tauri dev
```

## ⚠️ Notas

- `bun install` necesita red durante el build. Con `sandbox = true`, usar `--impure`.
- Solo Linux x86_64 por ahora.
