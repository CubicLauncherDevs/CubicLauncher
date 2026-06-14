# Nix — CubicLauncher

## Build con el builder script (recomendado)

```bash
curl -O https://raw.githubusercontent.com/CubicLauncher/CubicLauncher/main/dist/nix/cubiclauncher-builder
bash cubiclauncher-builder
```

## Build desde checkout local

```bash
cd dist/nix && nix-build release.nix
# o con flakes:
cd dist/nix && nix build --extra-experimental-features 'nix-command flakes'
```

## Build desde GitHub

```bash
nix build "github:CubicLauncher/CubicLauncher?dir=dist/nix" \
  --extra-experimental-features 'nix-command flakes' \
  --no-write-lock-file
```

## Shell de desarrollo

```bash
cd dist/nix && nix develop --extra-experimental-features 'nix-command flakes'
# Después: bun install && bun run tauri dev
```

## ⚠️ Notas

- **Linux:** requiere `webkitgtk_4_1`, `gtk3`, `libsoup_3`, etc. Si tu nixpkgs usa otros nombres, ajustalos en `default.nix`.
- **macOS:** soporte experimental, requiere frameworks `AppKit`, `WebKit`, `Foundation`, `SystemConfiguration`, `Security`.
- `bun install` necesita red durante el build. Con `sandbox = true`, usar `--impure`.

## Desinstalar

```bash
# nix-build
rm result
rm -r cubiclauncher-build

# flakes
nix store delete $(nix store path --hash result) 2>/dev/null; rm result
```
