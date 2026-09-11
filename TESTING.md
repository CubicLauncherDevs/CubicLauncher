# Testing

Lista de verificaciones para correr antes de mergear una PR o antes de lanzar una release.

## Checks automáticos (SIEMPRE)

```bash
# Frontend
bun install
bun run lint
bun run check
bun test --conditions=browser
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

### Mundos de las instancias

- Ejecutar `cargo test -p cubiclauncher --lib world_manager` y `cargo test -p cubiclauncher --lib world_operation_lock`.
- Medición reproducible: `cargo test -p cubiclauncher --lib world_performance_fixture -- --ignored --nocapture`. Genera 1000 mundos pequeños y una región sintética de 256 MiB de ceros; informa tiempos de listado frío/con caché e importación/exportación. No representa la compresión de regiones reales.
- [ ] Abrir Mundos en una instancia vacía, Vanilla y con mods; comprobar iconos, búsqueda, orden y paginación de 50 elementos.
- [ ] Importar una carpeta y ZIP con `level.dat` en raíz o en una carpeta interior. Repetir la importación: debe crear otra carpeta sin sobrescribir.
- [ ] Exportar y volver a importar un mundo con dimensiones y datos de mods. Verificarlo en Minecraft.
- [ ] Renombrar un mundo con caracteres Unicode: comprobar el nombre dentro de Minecraft y que la carpeta no cambie.
- [ ] Copiar la semilla de un mundo: comprobar que se pega correctamente en el portapapeles y que mundos sin semilla muestran el aviso correspondiente.
- [ ] Restablecer el icono de un mundo: comprobar que desaparece de la lista y se elimina el archivo `icon.png`.
- [ ] Abrir datapacks de un mundo: debe abrirse la carpeta `datapacks` del mundo en el explorador del sistema.
- [ ] Duplicar y eliminar la copia tras confirmar; el original debe conservarse.
- [ ] Mostrar un mundo con `level.dat` dañado y `level.dat_old` válido; debe ofrecer abrir/exportar sin habilitar renombrar.
- [ ] Calcular tamaño bajo demanda; iniciar/cerrar Minecraft y comprobar bloqueo de operaciones y actualización del listado.
- [ ] Durante una copia o ZIP grande, comprobar progreso y respuesta de la interfaz. Intentar lanzar, renombrar o eliminar la instancia: debe rechazarlo hasta terminar la operación.
- [ ] Cambiar rápidamente entre instancias durante una carga/operación: las respuestas antiguas no deben aparecer en otra instancia. Volver y actualizar al terminar.

### Consola y crashes

- Ejecutar `cargo test -p cubiclauncher --lib services::launcher::tests` y `cargo test -p cubiclauncher --lib services::instance_manager::manager::tests`.
- [ ] Con "abrir consola al iniciar" desactivado, lanzar sin el Java requerido: debe aparecer el modal de Java, sin ventana de logs ni evento de crash.
- [ ] Probar un fallo anterior a la creacion del proceso (por ejemplo, ejecutable Java invalido): debe conservarse el error de la instancia sin abrir logs ni emitir un crash.
- [ ] Forzar el cierre desde el launcher: no debe abrirse una consola ni generarse un evento o snapshot de crash. Una consola ya abierta puede permanecer visible.
- [ ] Provocar un crash real de un proceso iniciado: debe abrirse la consola y conservarse el snapshot. La salida normal no debe abrirla.
- [ ] Volver a lanzar una instancia que se cerro por la fuerza y provocar un crash: debe detectarse como un nuevo crash.
- [ ] Verificar que la apertura manual y la preferencia "abrir consola al iniciar" siguen funcionando.

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

- Ejecutar `bun test --conditions=browser tests/themeManager.test.mjs tests/themeDiagnostics.test.mjs` y `cargo test -p cubiclauncher --lib theme` para comprobar cargas concurrentes, limpieza de recursos, avisos, importacion con rollback, watcher y cache de tema activo.
- [ ] Arrastrar un `.mrpack` o `.zip` al launcher e importarlo.
- [ ] Cambiar de tema y verificar que apliquen las variables CSS.
- [ ] Importar un theme `.zip` o `.cbth`.
- [ ] Comparar temas V1/V2 con fondos, fuentes, iconos y `Inject.css`: deben conservar su apariencia, incluso con `injects_css` ausente o desactivado.
- [ ] Cambiar rapidamente A -> B -> A y simular un fallo de lectura: el tema anterior debe conservarse y las cargas tardias no deben mezclar fuentes o imagenes.
- [ ] Volver a seleccionar un tema ya cargado (por ejemplo, abrir una consola de logs): no debe re-leer el JSON desde Rust si el tema esta cacheado.
- [ ] Reimportar el tema activo desde ajustes y arrastrando el archivo, reemplazando iconos y fondos sin cambiar sus nombres. Comprobar la actualizacion en la ventana principal y una consola abierta.
- [ ] Editar y eliminar recursos en subcarpetas del tema activo; comprobar que el watcher sigue funcionando despues de reimportar. Importar otro tema por arrastre no debe cambiar ni recargar el activo.
- [ ] Importar un reemplazo corrupto: la instalacion anterior debe seguir intacta. Comprobar tambien paquetes antiguos exportados en Windows con separadores inversos.
- [ ] Comprobar avisos no bloqueantes en ajustes con mas de 12 fuentes, `Inject.css` mayor de 256 KiB o un fondo de mas de 16.777.216 pixeles. Los recursos no deben recortarse ni rechazarse por estos avisos; las validaciones anteriores siguen vigentes.
- [ ] Verificar watcher e importaciones en Windows/macOS y con el directorio de temas enlazado o montado en otro disco.

Los avisos de temas son orientativos, no mediciones de CPU/GPU. La estimacion del fondo cuenta una superficie RGBA (cuatro bytes por pixel), no toda la memoria del WebView. No se incorporan nuevos limites de rendimiento ni proteccion estricta frente a paquetes de descompresion extrema.

### Ajustes generales

- [ ] Cambiar idioma y verificar traducciones.
- [ ] Cambiar RAM min/max de una instancia.
- [ ] Cambiar versión de Java en una instancia.
- [ ] Activar/desactivar Discord Rich Presence y snapshots/alpha.

### UI / misc

- [ ] Sidebar responsive en tamaño reducido.
- [ ] Drawer de edición de instancia: cambiar icono, nombre, versión.
- [ ] Cerrar y reabrir el modal de crear instancia: no debe quedar estado sucio.
