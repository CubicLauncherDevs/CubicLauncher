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
