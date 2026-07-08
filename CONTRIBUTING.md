# Contribuir a CubicLauncher

Gracias por interesarte en mejorar CubicLauncher. Este documento resume cómo colaborar sin romper el build ni el estilo del proyecto.

## Requisitos

- [Node.js](https://nodejs.org/) >= 20
- [Bun](https://bun.sh/) >= 1.x
- [Rust](https://www.rust-lang.org/tools/install) >= 1.85 (edition 2024)
- [Tauri CLI v2](https://v2.tauri.app/start/prerequisites/)

## Instalación local

```bash
git clone https://github.com/CubicLauncherDevs/CubicLauncher.git
cd CubicLauncher
bun install
```

## Comandos antes de commitear

Siempre corré estos comandos antes de subir cambios:

```bash
# Frontend
bun run lint       # ESLint + plugin Svelte
bun run check      # svelte-check + TypeScript
bun run format     # Prettier en src/

# Backend (Rust)
cd src-tauri
cargo fmt --check
cargo clippy -- -D warnings
cd ..
```

## Convenciones de código

### Commits

- Los mensajes de commit van en **español**, en infinitivo o imperativo.
- Ejemplos válidos:
    - `Arreglar warns de vite`
    - `Agregar drawer de descarga de versiones`
    - `Refactorizar manejo de errores en VersionSelectorStep`

### Frontend (Svelte 5 + TypeScript)

- Usá runes: `$state`, `$derived`, `$effect`, `$props`, `$bindable`.
- **No uses `$state(...)` para envolver `SvelteMap` ni `SvelteSet`**; ya son reactivos por sí solos.
- Evitá `any`. Para íconos SVG tipá el `...rest` con `SVGAttributes<SVGSVGElement>`.
- Preferí `$derived` / `$derived.by` sobre `$state` + `$effect` cuando solo calculás un valor.
- Agregá `key` a los bloques `{#each}`.
- Para props pasadas pero no usadas en el template, usalas, renombralas a `_nombre` o ajustá la interfaz.

### Backend (Rust)

- Seguí `cargo fmt`.
- Resolvé todos los warnings de `cargo clippy`.
- Los errores deben serializarse como `{"code":"...","params":{...}}` para i18n en el frontend.

### Internacionalización

Si agregás textos nuevos en la UI, agregalos en:

- `src/lib/i18n/es.json`
- `src/lib/i18n/en.json`
- `src/lib/i18n/de.json`
- `src/lib/i18n/fr.json`

## Labels de Pull Request

Cuando abras un PR, asignale al menos una de estas labels para que el changelog
quede organizado:

| Label | Uso |
|-------|-----|
| `breaking` | Cambio que rompe compatibilidad con versiones anteriores. |
| `feature` | Nueva funcionalidad visible para el usuario. |
| `core` | Cambios en la lógica de backend Rust (servicios, crates, commands). |
| `ui` | Cambios puramente visuales en Svelte o CSS. |
| `i18n` | Nuevas traducciones o cambios en archivos de idioma. |
| `bug` / `fix` / `patch` | Corrección de errores o hotfix. |
| `perf` | Mejoras de rendimiento. |
| `test` | Nuevos tests o modificaciones de tests. |
| `docs` | Cambios en documentación. |
| `chore` / `refactor` / `deps` / `ci` | Tareas de mantenimiento. |
| `ignore-for-release` | Cambios que no deben aparecer en el changelog. |

## Flujo de release

CubicLauncher usa un flujo de release en dos etapas:

1. **Crear el tag**: al pushear un tag `v*` (p. ej. `v31.0.0`), el workflow
   `Build Release Draft` compila la app en todas las plataformas y sube los
   binarios a un **Release Draft** de GitHub con notas autogeneradas.
2. **Publicar manualmente**: cuando el draft esté listo, ejecutá el workflow
   manual `Publish Release` desde GitHub Actions. Solo en ese momento el
   release se vuelve público y el updater de Tauri lo empieza a distribuir.

Para prereleases (`v*-alpha*`, `v*-beta*`, `v*-rc*`) el proceso es el mismo,
pero el release se marca como prerelease. Además, cada push a la rama
`develop` dispara una build de prerelease que deja los binarios como artefactos
de la ejecución, sin crear un release público.

## Flujo de trabajo

1. Abrí un issue primero si el cambio es grande o puede discutirse.
2. Trabajá en una rama propia.
3. Hacé commits pequeños y descriptivos.
4. Actualizá este archivo si cambian las reglas del proyecto.
5. Abrí un PR y asegurate de que pasen los checks de CI (`lint`, `check`, tests
   de Rust, build del frontend, etc.).
6. Una vez mergeado a `main` o `develop`, el equipo decide cuándo publicar un
   nuevo tag y ejecutar el release correspondiente.
