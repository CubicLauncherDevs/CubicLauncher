# Testing

Lista de verificaciones para correr antes de mergear una PR o antes de lanzar una release.

## Checks automáticos (SIEMPRE)

```bash
# Frontend
bun install
bun run lint
bun run check
bun run build

# Rust
cd src-tauri
cargo fmt --check
cargo clippy -- -D warnings
cargo build --release
cd ..
```

## Build completo de Tauri

```bash
bun run tauri build
```

## Flujos manuales a verificar

### Instancias

- [ ] Crear instancia Vanilla y lanzarla.
- [ ] Crear instancia Fabric, descargarla y lanzarla.
- [ ] Crear instancia Forge, descargarla y lanzarla.
- [ ] Crear instancia Quilt, descargarla y lanzarla.

### Descarga de versiones

- [ ] Abrir el drawer "Descargar Versiones" desde la sidebar.
- [ ] Cambiar entre tabs: Releases, Snapshots, Alphas, Fabric, Forge, Quilt.
- [ ] Filtrar por instaladas/no instaladas y versión mayor.
- [ ] Descargar una versión de cada tipo.

### Mods / Resource Packs / Shaders

- [ ] Buscar mods en Modrinth y CurseForge.
- [ ] Agregar mods al basket y descargarlos.
- [ ] Verificar que aparezcan en la pestaña "Mods" de la instancia.
- [ ] Repetir para Resource Packs y Shaders si aplica.

### Autenticación

- [ ] Cambiar entre usuarios guardados.
- [ ] Agregar cuenta offline.
- [ ] (Si se puede) probar Microsoft / Yggdrasil.

### Modpacks y themes

- [ ] Arrastrar un `.mrpack` o `.zip` al launcher e importarlo.
- [ ] Cambiar de tema y verificar que apliquen las variables CSS.
- [ ] Importar un theme `.zip` o `.cbth`.

### Ajustes generales

- [ ] Cambiar idioma y verificar traducciones.
- [ ] Cambiar RAM min/max de una instancia.
- [ ] Cambiar versión de Java en una instancia.
- [ ] Activar/desactivar Discord Rich Presence y snapshots/alpha.

### UI / misc

- [ ] Sidebar responsive en tamaño reducido.
- [ ] Drawer de edición de instancia: cambiar icono, nombre, versión.
- [ ] Cerrar y reabrir el modal de crear instancia: no debe quedar estado sucio.
