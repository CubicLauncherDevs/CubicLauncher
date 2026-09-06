# Testing

Lista de verificaciones para correr antes de mergear una PR o antes de lanzar una release.

## Checks automáticos (SIEMPRE)

```bash
# Frontend
bun install
bun run lint
bun run check
bun test
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

### CSP y avatares

- Ejecutar `cargo test -p cubiclauncher --lib commands::avatar::tests` para validar skins modernas, legacy, HD y dimensiones invalidas.
- Compilar con `bun run tauri build --debug --no-bundle` y abrir el ejecutable generado. Vite por si solo no comprueba los hashes CSP que inyecta Tauri.
- En DevTools, comprobar que el arranque de SvelteKit funciona sin bloqueos de `script-src` y que la politica de produccion no permite scripts inline arbitrarios ni `http://localhost:*`.
- Verificar avatar Microsoft, Yggdrasil y offline en ambas barras laterales, lista de cuentas y cabecera del perfil. Forzar un fallo del comando `get_avatar_svg` para comprobar el fallback HTTP y una respuesta de error del fallback para comprobar el avatar en cache/predeterminado.
- Confirmar que los avatares se renderizan como `<img src="data:image/svg+xml,...">`, no como SVG insertado con `{@html}`. Los scripts y recursos externos del SVG no deben ejecutarse/cargarse en este contexto; los PNG embebidos deben seguir visibles.
- Cambiar temas, incluidas fuentes locales y CSS personalizado, y revisar skins/capas 3D, capturas, traducciones e imagenes del marketplace sin nuevas violaciones CSP.
- Ejecutar `bun run tauri dev` y comprobar la conexion WebSocket de HMR tras editar un componente.

`dangerousDisableAssetCspModification` excluye solo `style-src`: los temas crean estilos inline dinamicos. Inyectar hashes/nonces en esa directiva haria que el navegador ignorase `'unsafe-inline'` y bloqueara esos estilos. La modificacion automatica de `script-src` permanece activa.

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
- [ ] En Local, mostrar dos archivos del mismo proyecto y seleccionar cada uno sin errores de claves duplicadas. Eliminar uno debe conservar el otro y sus metadatos.
- [ ] Activar/desactivar un mod con otra version instalada: solo debe renombrarse el archivo seleccionado, conservando la seleccion tras el refresco.
- [ ] Cambiar de Modrinth/CurseForge a Local con una peticion pendiente: la respuesta tardia no debe reemplazar ni mezclarse con los archivos locales.

### Autenticación

- [ ] Cambiar entre usuarios guardados.
- [ ] Agregar cuenta offline.
- [ ] (Si se puede) probar Microsoft / Yggdrasil.

### Ventanas WebView2 (Windows)

- Ejecutar `cargo test -p cubiclauncher --lib core::webview::tests` para comprobar que Microsoft y logs heredan las opciones de entorno de `main` sin copiar sus ajustes de ventana.
- [ ] Abrir Microsoft y la consola de logs con la ventana principal abierta. Ambas deben permanecer abiertas y cargar su contenido, sin errores `0x8007139F` de WebView2.
- [ ] Cerrar y volver a abrir ambas ventanas. Completar o cancelar el login de Microsoft.
- [ ] Activar el cierre del launcher al iniciar un juego, mantener la consola de logs abierta y salir del juego. La ventana principal debe reaparecer y permitir abrir Microsoft y logs otra vez.

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
