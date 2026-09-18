# Testing

Lista de verificaciones para correr antes de mergear una PR o antes de lanzar una release.

## Organización de los tests

```text
tests/frontend/
├── marketplace/       # Estado y selección del marketplace
├── servers/           # API, cachés y recursos de servidores
│   └── fixtures/      # Escenarios ejecutados en procesos aislados
├── themes/            # Aplicación de temas y diagnósticos
└── ui/                # Animaciones e iconos

src-tauri/src/tests/
├── commands/          # Avatares, idiomas, instancias e importación de temas
├── core/              # Errores, rutas y ventanas WebView
├── services/          # Descargas, instancias, servidores y mundos
└── theme_watcher.rs   # Eventos y ciclo de vida del watcher

crates/<crate>/src/tests/  # Tests de aqua, communicator, launchwerk y zellkern
```

### Convenciones

- **Frontend:** agregar archivos `*.test.mjs` en el área correspondiente. Las
  fixtures propias de una suite van en su subcarpeta `fixtures/`, sin el sufijo
  `.test`, para evitar que Bun las ejecute como suites independientes.
- **Rust:** ubicar los tests en `src/tests/` del crate, reflejando la ruta del
  módulo de producción (por ejemplo, `services/instance_manager/handle.rs`).
  Para los módulos definidos en `mod.rs`, usar el nombre del área como archivo,
  como `tests/theme_watcher.rs`. Las suites con nombre propio pueden tener un
  archivo descriptivo, como `tests/commands/themes/import.rs`.
- Registrar cada suite Rust desde el módulo que verifica mediante
  `#[cfg(test)]` y `#[path = "ruta/relativa/al/test.rs"]`. El archivo contiene
  directamente los tests y puede usar `use super::*;`. Así mantiene acceso a
  miembros privados y conserva los filtros existentes, como
  `services::launcher::tests`. No hace falta un `mod tests` global en `lib.rs`.
- Los doctests permanecen junto a la API que documentan. Los ejemplos de
  `crates/*/examples/` se ejecutan con `cargo run -p <crate> --example <nombre>`.

## Ejecutar tests

Desde la raíz del repositorio:

```bash
bun run test:frontend  # Todas las suites Bun, con las condiciones de Svelte browser
bun run test:rust      # cargo test --workspace, incluidos doctests
bun run test:all       # Frontend y Rust

# Una sola área o archivo frontend
bun test --conditions=browser ./tests/frontend/themes
bun test --conditions=browser ./tests/frontend/servers/servers.test.mjs

# Un crate o módulo Rust; los filtros no dependen de la ubicación del archivo
cargo test -p aqua
cargo test -p cubiclauncher --lib services::launcher::tests
```

`ui/perfAnimations.test.mjs` usa Chromium/Chrome en modo headless. Permite elegir
el ejecutable con `CHROME_BIN`; en su defecto busca primero `google-chrome` o
`google-chrome-stable` y después `chromium` o `chromium-browser` en `PATH`.
Si no encuentra ninguno, el test se marca como omitido; una ruta explícita inválida
produce un error. CI usa `/usr/bin/google-chrome` y `dbus-run-session` para evitar
los lanzadores Snap y las esperas de servicios de escritorio durante el arranque.
La prueba espera a que terminen las animaciones finitas usando la API del navegador.
Los tests de manifiestos de `aqua` requieren conexión a Internet.

Las mediciones sintéticas de rendimiento Rust están marcadas con `#[ignore]`
y se ejecutan explícitamente:

```bash
cargo test -p cubiclauncher --lib world_performance_fixture -- --ignored --nocapture
cargo test -p cubiclauncher --lib server_performance_fixture -- --ignored --nocapture --test-threads=1
```

CI ejecuta `bun run test:frontend` y `cargo test --workspace --verbose` en sus
respectivos jobs, además de lint, tipos, formato, Clippy y build frontend.

## Checks automáticos (SIEMPRE)

```bash
# Frontend
bun install
bun run lint
bun run check
bun run test:frontend
bun run build

# Rust (desde la raíz)
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
bun run test:rust
cargo build --workspace --release
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
- [ ] En Descargas > OptiFine, elegir Minecraft y descargar una versión. Comprobar progreso, Java automático y marca de instalado.
- [ ] Crear o editar una instancia con OptiFine instalado y lanzarla. Comprobar versiones legacy (1.12.2) y modernas (1.20.1).
- [ ] Activar loaders inestables y comprobar versiones preliminares de OptiFine. Probar actualizar el catálogo y reintentar después de un error de red.

La prueba de integración de OptiFine descarga los clientes y los instaladores oficiales, ejecuta el parcheador y comprueba perfiles y bibliotecas en un directorio temporal:

```bash
# Requiere Internet y Java en PATH (o indicar OPTIFINE_TEST_JAVA).
cargo test -p aqua optifine_official_install_smoke -- --ignored --nocapture
```

### Personalizar la interfaz

```bash
cargo test -p cubiclauncher --lib services::settings_manager
bun test --conditions=browser ./tests/frontend/ui/interfacePreferences.test.mjs ./tests/frontend/ui/interfaceLayout.test.mjs
```

La regresión de interfaz monta los controles, el Drawer, las listas virtualizadas
y los menús reales en Chrome, Chromium o Firefox. Recorre los cinco temas incluidos
y un tema con variables en `rem`/`calc()` y `Inject.css`, con cuatro escalas y tres
densidades. Simula el viewport CSS resultante del zoom nativo en una ventana de
800 × 600; el zoom nativo se verifica por separado mediante su llamada IPC.
Comprueba la conservación de estilos del tema y altura de las tarjetas, los
espacios entre filas, menús dentro del viewport, cambios de tema en vivo, el final
de las listas, recuperación de un fallo de zoom y restablecimiento.

- [ ] En Tauri, abrir Personalizar → Interfaz y probar 90 %, 100 %, 110 % y 125 %
  con la ventana en su tamaño mínimo. Comprobar clics, menús, scroll y paneles.
- [ ] Cambiar entre densidad del tema, compacta y cómoda con listas largas abiertas.
  Cambiar el tema, incluyendo uno importado con fuentes y CSS propios.
- [ ] Reiniciar y abrir una ventana de logs: la escala elegida debe conservarse.
- [ ] Restablecer valores: recuperar 100 % y el espaciado original del tema.

### Rendimiento de la personalización

```bash
bun test --conditions=browser ./tests/frontend/ui/saveQueue.test.mjs ./tests/frontend/ui/settingsPersistence.test.mjs ./tests/frontend/themes/themeMetrics.test.mjs
```

- Las pruebas de guardado agrupan 100 solicitudes en una escritura del último
  estado; con una escritura ya en curso, solo añaden una escritura pendiente.
  Comprueban el vaciado al cerrar, recuperación de errores y descarte de lecturas
  antiguas que podrían sobrescribir cambios locales.
- Las pruebas de medidas simulan 100 entregas de `ResizeObserver`: las consultas
  de estilo se limitan a la medición inicial, usando después `contentRect`.
- La regresión de interfaz también verifica cambios sin efecto, controles
  disponibles mientras se guarda, identidad estable de las preferencias y
  deslizadores que guardan al soltar o al desmontarse. Monta notificaciones reales
  para comprobar que los cambios visuales no reinician temporizadores ni crean
  temporizadores para avisos persistentes o descargas en curso.

### Personalizar las notificaciones

```bash
cargo test -p cubiclauncher --lib services::settings_manager
bun test --conditions=browser ./tests/frontend/ui/notificationPreferences.test.mjs
bun test --conditions=browser ./tests/frontend/ui/drawerGestures.test.mjs
```

La regresión de gestos monta los controles reales dentro del Drawer y usa Chrome,
Chromium o Firefox en modo headless (se omite si no encuentra ninguno). Comprueba
los cuatro sentidos del panel, exclusión de controles, guardado de deslizadores,
cancelación y pérdida de captura, y cierre desde una zona libre.

- [ ] Comprobar el orden Launcher → Minecraft → Personalizar → Java y abrir
  Personalizar → Notificaciones.
- [ ] Sin activar «Personalizar notificaciones», comprobar el formato clásico:
  esquina superior derecha, ancho según el contenido y tiempos originales de
  cada aviso. Los controles personalizados deben estar deshabilitados.
- [ ] Activar la casilla, cambiar opciones y desactivarla: el formato clásico
  regresa y las preferencias se recuperan al volver a activarla.
- [ ] Elegir las seis posiciones y los tres tamaños de cuadro. Usar «Probar
  notificación» y comprobar que no se sale de la ventana, incluso con textos largos.
- [ ] Cambiar el tamaño del título (12–24), el mensaje (11–22), mayúsculas y
  negrita por separado. El mensaje debe conservar la escritura original.
- [ ] Arrastrar los deslizadores: el aviso cambia en vivo y el guardado ocurre al
  soltar. El panel no debe moverse ni cerrarse. Comprobar también que arrastrar una
  zona libre sigue cerrándolo y que cancelar el gesto lo devuelve a su posición.
  Reiniciar el launcher y comprobar que las preferencias se conservan.
- [ ] Elegir una duración entre 3 y 30 segundos. Los avisos persistentes y las
  descargas en curso no deben desaparecer; las completadas usan la duración elegida.
- [ ] Cambiar posición o tipografía con un aviso activo: su temporizador no debe
  reiniciarse. Probar también con reducción de animaciones.
- [ ] Repetir la prueba y cerrar el apartado: no deben acumularse avisos de prueba,
  estilos de cuenta atrás ni temporizadores después de desmontarlos.
- [ ] Restablecer valores y comprobar que se desactiva la personalización.
  Cargar una configuración anterior con notificaciones destacadas: las opciones
  se conservan como formato amplio centrado, pero requieren activación explícita.

### Cierre del launcher durante el juego

```bash
cargo test -p cubiclauncher --lib services::launch
```

La opción «Ocultar el launcher al jugar» espera el
mensaje `Setting user...`, como PolyMC. Destruye el WebView principal y mantiene
el backend supervisando las sesiones. Sin ese mensaje, la ventana permanece
abierta. La consola conserva su ajuste independiente.

- [ ] Activar la opción y lanzar Vanilla y un modpack. La ventana debe cerrarse al
  aparecer el mensaje y regresar con su posición/tamaño al salir del juego.
- [ ] Repetir con la opción desactivada y con la consola automática activada.
- [ ] Comprobar un fallo al ejecutar Java y una salida anterior a `Setting user`:
  la ventana debe permanecer disponible.
- [ ] Forzar un crash después del cierre de la ventana: deben reaparecer la ventana
  principal y la consola con las últimas líneas del error.
- [ ] Terminar el proceso desde la consola y probar una salida inmediatamente
  después de `Setting user`: debe restaurarse una sola ventana principal.
- [ ] Lanzar dos instancias antes de que se cierre la ventana. Terminar ambas en
  distinto orden: el backend debe seguir supervisando la segunda tras salir de la
  primera y la reapertura no debe fallar por una etiqueta `main` duplicada.
- [ ] En Windows, comprobar la reapertura para descartar bloqueos de WebView2.
  En Linux y macOS, comprobar también el foco y el estado maximizado.
- [ ] Repetir varios ciclos y observar memoria/CPU con el monitor del sistema:
  los handles y lectores de instancias terminadas deben liberarse. Con la ventana
  principal y la consola cerradas, se conserva un historial de hasta 5000 líneas
  por sesión sin generar lotes IPC ni activar un temporizador periódico de logs.

### Rendimiento de instalados

```bash
cargo test -p cubiclauncher --lib mod_catalog -- --nocapture
cargo test -p cubiclauncher --lib addon_manager
bun test --conditions=browser ./tests/frontend/marketplace
```

La fixture Rust genera 50, 100 y 200 JAR con metadatos e iconos PNG, sin usar la
red. Imprime tiempos de catálogo frío/caliente, procesamiento de metadatos,
iconos de 16 filas y bytes de respuesta frente a la carga de todos los iconos.
Los tiempos son orientativos: esta fixture no mide FPS ni sustituye un modpack
real con JAR grandes. Los tests verifican reutilización tras renombrar un mod,
invalidación de archivos modificados y límites de caché.

Las pruebas frontend verifican consultas limitadas al área visible, búsquedas
con objetos reutilizados, caché de iconos acotada y descarte de respuestas
tardías. En el launcher, comprobar apertura, desplazamiento rápido, búsqueda,
cambio de instancia y activación por lotes con un modpack real; en lista y
tarjetas los iconos deben aparecer progresivamente sin mover la selección.

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

### Servidores de las instancias

- Ejecutar `cargo test -p cubiclauncher --lib server_`, `cargo test -p zellkern server_launch_tests` y `bun test --conditions=browser ./tests/frontend/servers`.
- Medición reproducible: `cargo test -p cubiclauncher --lib server_performance_fixture -- --ignored --nocapture --test-threads=1`. Genera listas de 50/500/1000 servidores con PNG de 32×32; mide carga de metadatos, iconos de la primera página, preparación de destinos y serialización IPC. Cuenta lecturas, decodificaciones, asignaciones Rust, pico/retención de heap y tiempo de CPU del hilo en Linux (`/proc/thread-self/schedstat`). El contador de asignaciones solo existe en tests.
- [ ] Abrir Servidores en una instancia sin `servers.dat`; añadir, editar, reordenar y eliminar entradas, incluidas direcciones duplicadas. Comprobar la lista dentro de Minecraft.
- [ ] Probar Preguntar/Aceptar/Rechazar paquetes de recursos y comprobar que se conservan iconos y campos de mods al editar nombres.
- [ ] Actualizar estado con servidores accesibles y sin respuesta; verificar descripción, jugadores, ping e iconos PNG.
- [ ] Probar dominios con SRV, IP, puertos personalizados e IPv6; comprobar la conexión con Vanilla y loaders, en una versión moderna y otra anterior a Quick Play.
- [ ] Conectar con el Java requerido sin instalar: instalar desde el diálogo y comprobar que el reintento conserva el servidor elegido.
- [ ] Cambiar rápidamente entre instancias, salir durante una consulta y actualizar varias veces; las respuestas anteriores no deben contaminar la lista actual.
- [ ] Iniciar Minecraft: la edición queda bloqueada, pero las consultas siguen disponibles. Al cerrar el juego se recarga la lista.
- [ ] Modificar `servers.dat` externamente con un formulario abierto: al guardar debe informar un conflicto. Un archivo corrupto debe informar el error sin sobrescribirlo.
- [ ] Verificar búsqueda y paginación con más de 50 servidores, navegación por teclado y diseño estrecho.
- [ ] Con 1000 entradas, consultar únicamente las 50 visibles y la seleccionada; direcciones duplicadas deben compartir consulta. Comprobar como máximo 2 consultas activas por ventana y 4 globalmente.
- [ ] Volver a una página reciente: reutilizar estados durante 30 segundos e iconos durante 5 minutos. Renombrar/reordenar/cambiar paquetes de recursos conserva resultados; cambiar dirección invalida el icono. Actualizar fuerza las consultas.
- [ ] Escribir rápidamente en el buscador, cambiar de página y ocultar/cerrar la ventana: cancelar consultas antiguas sin respuestas tardías ni acumulación. Las cachés tienen un límite de 100 entradas y 512 KiB estimados de texto cada una, además de los recursos de hasta 51 filas activas; cerrar la pestaña libera ambos.

Referencia sintética en Linux, build de desarrollo, una ejecución antes/después (no representa la RAM total ni el tiempo de arranque del launcher):

| Servidores | Pico heap antes → después | CPU del hilo antes → después | IPC antes → después |
| --- | --- | --- | --- |
| 50 | 1.44 → 0.82 MB | 66.5 → 50.8 ms | 0.286 → 0.287 MB |
| 500 | 12.98 → 3.24 MB | 557.1 → 250.2 ms | 2.859 → 0.335 MB |
| 1000 | 25.93 → 6.16 MB | 1099.9 → 491.3 ms | 5.719 → 0.388 MB |

La comparación anterior cargaba/decodificaba la lista completa dos veces (lectura y preparación del ping). Ahora las dos lecturas son metadatos prestados del buffer NBT e iconos de la primera página: 50 decodificaciones y 50 destinos de ping en los tres tamaños. Con 1000 entradas, las asignaciones bajaron de 56 081 a 2 946 y la retención medida de 17.37 a 0.92 MB; al destruir el resultado se liberó todo el heap medido. Se excluyen WebView, superficies de imágenes, asignaciones nativas y latencia de servidores reales. Estos resultados son orientativos, no umbrales universales de tiempo.

### Consola y crashes

- Ejecutar `cargo test -p cubiclauncher --lib services::launcher::tests` y `cargo test -p cubiclauncher --lib services::instance_manager::manager::tests`.
- [ ] Con "abrir consola al iniciar" desactivado, lanzar sin el Java requerido: debe aparecer el modal de Java, sin ventana de logs ni evento de crash.
- [ ] Probar un fallo anterior a la creacion del proceso (por ejemplo, ejecutable Java invalido): debe conservarse el error de la instancia sin abrir logs ni emitir un crash.
- [ ] Forzar el cierre desde el launcher: no debe abrirse una consola ni generarse un evento o snapshot de crash. Una consola ya abierta puede permanecer visible.
- [ ] Provocar un crash real de un proceso iniciado: debe abrirse la consola y conservarse el snapshot. La salida normal no debe abrirla.
- [ ] Volver a lanzar una instancia que se cerro por la fuerza y provocar un crash: debe detectarse como un nuevo crash.
- [ ] Verificar que la apertura manual y la preferencia "abrir consola al iniciar" siguen funcionando.

### Descarga de versiones

#### Arranque y carga bajo demanda

```bash
bun test --conditions=browser ./tests/frontend/ui/launcherStartup.test.mjs ./tests/frontend/ui/startupPage.test.mjs
```

La prueba del servicio verifica que las consolas solo procesan cambios de ajustes
y tema, sin inicializar los estados de descargas ni cargar instancias. La lista
de versiones instaladas se actualiza al finalizar una descarga aunque nunca se
haya abierto el descargador; las finalizaciones durante una lectura se agrupan
en una lectura posterior, descartando el resultado antiguo.

La prueba de página monta la ruta y sus contenedores reales en un navegador
headless, con llamadas nativas y paneles finales instrumentados. Comprueba que
Ajustes y Descargador se importan al abrirse, que reabrir el descargador conserva
su montaje y que una consola no inicia Discord, el actualizador ni drag-and-drop.
También comprueba el desmontaje durante un arranque pendiente. Se omite si no
encuentra un navegador compatible.

- [ ] Arrancar sin abrir paneles: no debe solicitarse el catálogo de versiones
  disponibles por el descargador. Abrirlo por primera vez, cerrarlo y reabrirlo;
  comprobar filtros, selección y descarga.
- [ ] Instalar una versión sin haber abierto el descargador; comprobar que está
  disponible al crear una instancia y al abrir el descargador posteriormente.
- [ ] Abrir Ajustes desde la barra lateral y desde el tutorial; cerrar y reabrir,
  cambiar una preferencia y comprobar su persistencia.
- [ ] Abrir varias consolas: deben conservar idioma, tema, escala, historial y
  actualizaciones de configuración sin cargar instancias/versiones ni comprobar
  actualizaciones del launcher. Repetir con la ventana principal cerrada al jugar.
- [ ] Comparar arranque/reposo y apertura de consolas en builds de producción con
  la misma configuración. Medir el conjunto Rust/WebView y separar Minecraft;
  estas pruebas funcionales no cuantifican la reducción de RAM o CPU.

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

#### Memoria y ciclo de vida de la market

- Ejecutar `bun test --conditions=browser ./tests/frontend/marketplace` y `cargo test -p cubiclauncher --lib market_`.
- El escenario sintético recorre 2000 resultados en cinco sesiones: retiene como máximo 15 páginas / 300 proyectos por sesión y cero páginas tras destruirla. Imprime el heap de JavaScriptCore tras cada cierre y GC; la primera sesión incluye calentamiento y las cifras no equivalen al RSS ni incluyen DOM, imágenes o WebView.
- Para un perfil más largo: `NODE_ENV=production MARKET_MEMORY_SESSIONS=50 MARKET_MEMORY_PROFILE=1 bun test --conditions=browser --conditions=production ./tests/frontend/marketplace/marketState.test.mjs -t "long browsing"`. La fixture limpia también historiales de mocks antes de medir: estos retienen payloads y frames de llamadas, y contaminarían el perfil. Las diferencias de tipos de objetos ayudan a distinguir calentamiento del motor de estados retenidos por sesión.
- Referencia sintética en Linux/Bun 1.3.14: en 50 sesiones (100 000 resultados procesados), las últimas diez mediciones tras cerrar y GC quedaron entre 10 403 120 y 10 412 396 bytes de heap JSC (~9.93 MiB). El conteo final no aumentó en mapas, proxies ni entornos léxicos respecto al primer cierre. Son mediciones del harness, no un umbral de RAM del launcher ni una sustitución de los snapshots del WebView.
- Las cachés nativas conservan hasta 4 MiB de JSON serializado y claves por proveedor, 200 entradas y 1 MiB por entrada. Las respuestas mayores siguen disponibles, pero no se cachean. Expiran a los cinco minutos; un barrido cada minuto también libera entradas durante la inactividad. El presupuesto excluye estructuras auxiliares y respuestas en vuelo.
- La caché Markdown admite hasta 18 entradas / 2 MiB estimados de cadenas UTF-16, incluyendo claves construidas a partir del texto limitado.
- Las consultas de lectura cancelables comparten cuatro slots nativos. Cada petición tiene límite temporal y registro con limpieza al finalizar; las cancelaciones anticipadas se conservan temporalmente en una tabla limitada a 128 entradas para resolver carreras entre mensajes IPC.
- [ ] Recorrer más de 300 resultados y volver arriba: las páginas descartadas se recuperan sin reiniciar el scroll. Abrir/cerrar un detalle debe mantener la posición.
- [ ] Repetir apertura/cierre de la market y de sus selectores mientras hay consultas lentas, escaneos o resolución de dependencias. No deben quedar menús, listeners, observadores ni tarjetas desconectadas retenidas en snapshots del WebView.
- [ ] Cerrar durante una descarga confirmada: la descarga debe continuar; su finalización no debe reactivar el estado desmontado. Volver a abrir y comprobar los archivos instalados.
- [ ] Comparar snapshots después de varias sesiones equivalentes y tras expirar cachés. Separar heap JS, imágenes y cachés nativas del RSS total del proceso.

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

- Ejecutar `bun test --conditions=browser ./tests/frontend/themes` y `cargo test -p cubiclauncher --lib theme` para comprobar cargas concurrentes, limpieza de recursos, avisos, importacion con rollback, watcher y cache de tema activo.
- [ ] Arrastrar un `.mrpack` o `.zip` al launcher e importarlo.
- [ ] Cambiar de tema y verificar que apliquen las variables CSS.
- [ ] Comprobar colores de logs, descargas, notificaciones y overlays con los temas claros/oscuros y un tema custom. Los botones primarios deben usar `--accent-text`.
- [ ] Cambiar `--sidebar-row-height`, `--version-row-height`, `--resource-row-height` y las medidas `--market-*` durante el desplazamiento: filas, columnas y altura total deben actualizarse sin solapamientos ni huecos por desincronización. Repetir con `rem` y cambiando `--font-size-base`.
- [ ] En V2, cargar un `Inject.css` con `.market-item { padding: 20px; }` sin `!important`: debe ganar al estilo encapsulado de Svelte. Probar también variables de medidas en ese CSS y volver a un tema incluido para comprobar que se restauran los valores predeterminados.
- [ ] Comprobar `--modal-width`, `--modal-padding`, `--icon-scale`, fuentes monoespaciadas y brillo del fondo, también con desenfoque y animaciones desactivados. Variables predeterminadas: [reset.css](src/styles/shared/reset.css).
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
