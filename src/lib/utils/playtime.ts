type Translate = (
	key: string,
	params?: Record<string, string | number>,
) => string;

/**
 * Formatea el tiempo jugado acumulado (en segundos) al estilo Prism:
 * horas + minutos para sesiones largas, y segundos para totales pequeños.
 *
 * Recibe `t` por parámetro para poder probarse sin depender del estado global
 * de i18n.
 */
export function formatPlaytime(seconds: number, t: Translate): string {
	const safe =
		Number.isFinite(seconds) && seconds > 0 ? Math.floor(seconds) : 0;
	if (safe < 60) {
		return t("instanceView.playtimeSeconds", { seconds: safe });
	}
	const totalMinutes = Math.floor(safe / 60);
	const hours = Math.floor(totalMinutes / 60);
	const minutes = totalMinutes % 60;
	if (hours > 0) {
		return t("instanceView.playtimeHoursMinutes", { hours, minutes });
	}
	return t("instanceView.playtimeMinutes", { minutes });
}
