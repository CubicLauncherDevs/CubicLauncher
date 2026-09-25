type Translate = (
	key: string,
	params?: Record<string, string | number>,
) => string;

const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/**
 * Patrones indexados por la "forma" del resultado: cada bit indica si ese
 * componente se muestra (4 = días, 2 = horas, 1 = minutos).
 *
 * Guardar los patrones ya combinados permite resolver el texto con una única
 * llamada a `t` (una búsqueda en el diccionario + un reemplazo de placeholders)
 * en lugar de componer y unir fragmentos.
 */
const PATTERN_KEYS = [
	"instanceView.playtimeSeconds", // 0: se resuelve antes, nunca se indexa
	"instanceView.playtimeMinutes", // 1
	"instanceView.playtimeHours", // 2
	"instanceView.playtimeHoursMinutes", // 3
	"instanceView.playtimeDays", // 4
	"instanceView.playtimeDaysMinutes", // 5
	"instanceView.playtimeDaysHours", // 6
	"instanceView.playtimeDaysHoursMinutes", // 7
] as const;

/**
 * `true` cuando la instancia ya acumula tiempo jugado que mostrar. Se usa para
 * ocultar la fila en lugar de enseñar un "0 s" en instancias sin estrenar.
 */
export function hasPlaytime(seconds: number): boolean {
	return Number.isFinite(seconds) && seconds >= 1;
}

/**
 * Formatea el tiempo jugado acumulado (en segundos) al estilo Prism:
 * unidades de mayor a menor ("3 d 4 h 26 min"), omitiendo los componentes
 * que valen cero ("4 h", "26 min", "45 s").
 *
 * Recibe `t` por parámetro para poder probarse sin depender del estado global
 * de i18n.
 */
export function formatPlaytime(seconds: number, t: Translate): string {
	// NaN, Infinity y negativos se tratan como 0.
	const safe =
		Number.isFinite(seconds) && seconds > 0 ? Math.floor(seconds) : 0;
	if (safe < MINUTE) {
		return t(PATTERN_KEYS[0], { seconds: safe });
	}
	// Los restos quedan por debajo de 24 y de 60, así que truncar con `| 0`
	// nunca desborda (a diferencia de operar sobre los segundos totales).
	const days = Math.floor(safe / DAY);
	const hours = ((safe % DAY) / HOUR) | 0;
	const minutes = ((safe % HOUR) / MINUTE) | 0;
	const shape =
		(days > 0 ? 4 : 0) | (hours > 0 ? 2 : 0) | (minutes > 0 ? 1 : 0);
	return t(PATTERN_KEYS[shape], { days, hours, minutes });
}
