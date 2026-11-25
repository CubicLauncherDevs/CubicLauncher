# Iconos

Todos los iconos de la interfaz viven acá, agrupados por uso:

| Carpeta     | Contenido                                             |
| ----------- | ----------------------------------------------------- |
| `ui/`       | Controles y estados genéricos (check, close, loader…) |
| `nav/`      | Navegación y acciones de la barra lateral             |
| `instance/` | Secciones y acciones de una instancia                 |
| `log/`      | Consola de logs                                       |
| `brand/`    | Logos de marca (Cubic, Discord, Microsoft…)           |

`favicon.png` se sirve desde `src/app.html`.

## Formatos

- **SVG monocromo**: `fill="#000000"` o `stroke="#000000"`, sin colores propios. Se usan como máscara CSS y toman el color del texto (`currentColor`).
- **PNG/WebP o SVG a color**: se muestran como imagen. Para logos de marca a color usá `<img src="/icons/brand/...">` en lugar de `<Icon>`, porque la máscara los volvería monocromos.

## Cómo usar y reemplazar

1. Reemplazá el archivo manteniendo el mismo nombre, o agregá uno nuevo en la subcarpeta que corresponda.
2. Registrá la clave y la ruta en `src/lib/icons/registry.ts`.
3. Usalo con `<Icon name="ui:check" />` desde cualquier componente.

Los temas pueden sobrescribir una clave con su propio bloque `icons` en el JSON del tema (ver `getThemeIcon` en `src/lib/api/themeManager.ts`).

## No mover

`static/images/instances/` contiene los logos de loaders y presets de instancia. El backend en Rust detecta esos iconos por el prefijo `/images/` (`is_asset_icon`), así que deben quedarse ahí.
