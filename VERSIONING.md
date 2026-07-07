# Versionado de CubicLauncher

CubicLauncher usa un sistema de versiones secuencial simple:

- **Major** — número entero que incrementa con cada release (27, 28, 29...)
- **Patch** — revisiones o hotfixes sobre un mismo major

## Motivación del cambio

El sistema anterior (`AAMMP`, ej. `2606c`) era básicamente copiar el
formato de snapshots de Mojang, mezclando fecha y parche en un código
críptico. Requería tabla de mapeo, el semver interno se veía raro
(major `26`), y nadie lo entendía de un vistazo. El nuevo sistema es
secuencial simple: cada release es solo un número que sube. Se explica
solo.

## Formato de versión

| Release | User-facing | Semver (interno) |
|---------|-------------|------------------|
| Major   | `27`        | `27.0.0`         |
| Rev 1   | `27 rev 1`  | `27.0.1`         |
| Rev 2   | `27 rev 2`  | `27.0.2`         |
| Major   | `28`        | `28.0.0`         |

- Si `PATCH == 0` → se muestra solo el major (`"27"`)
- Si `PATCH > 0` → se muestra major + `" rev "` + patch (`"27 rev 1"`)

## Nota técnica

Internamente el proyecto usa semver (`MAJOR.MINOR.PATCH`) requerido por Tauri
para el sistema de actualizaciones automáticas. El mapeo es directo:

| User-facing | Semver  |
|-------------|---------|
| `27`        | `27.0.0`|
| `27 rev 1`  | `27.0.1`|
| `28`        | `28.0.0`|

> **Nota:** El `MINOR` en semver siempre es `0`. Solo se usa `MAJOR` y `PATCH`.

## Proceso de publicación

CubicLauncher publica las versiones en dos pasos para evitar distribuir
binarios no revisados.

### 1. Crear un tag

Los tags siguen el formato interno de semver:

```bash
git tag v31.0.0
git push origin v31.0.0
```

Para prereleases:

```bash
git tag v31.0.0-alpha.1
git push origin v31.0.0-alpha.1
```

### 2. Esperar el draft

Al pushear el tag, el workflow `Build Release Draft` (o `Build Prerelease Draft`)
compila la app en Linux, Windows y macOS, y sube los binarios a un **Release
Draft** de GitHub con notas autogeneradas. El release **no es público** todavía.

### 3. Revisar y publicar

Una vez que el draft esté completo, ejecutá el workflow manual
`Publish Release` desde GitHub Actions indicando el tag. Solo ahí el release se
vuelve público y el sistema de actualizaciones de Tauri lo distribuye a los
usuarios.

### Builds de desarrollo

Cada push a la rama `develop` dispara una build de prerelease que deja los
binarios como artefactos de la ejecución de GitHub Actions. No crea un release
público ni una nueva versión.
