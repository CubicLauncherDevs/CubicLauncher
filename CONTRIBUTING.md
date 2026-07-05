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

## Flujo de trabajo

1. Abrí un issue primero si el cambio es grande o puede discutirse.
2. Trabajá en una rama propia.
3. Hacé commits pequeños y descriptivos.
4. Actualizá este archivo si cambian las reglas del proyecto.
5. Abrí un PR y asegurate de que pasen los checks de CI.
