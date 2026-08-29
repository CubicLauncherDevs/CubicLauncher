import { launcherStore } from "$lib/state/state.svelte";

/**
 * Devuelve `true` si el usuario habilitó la reducción de animaciones.
 * Úsalo dentro de componentes Svelte para que la lectura sea reactiva.
 */
export function shouldReduceAnimations(): boolean {
	return launcherStore.settings.reduce_animations;
}

/**
 * Devuelve la duración de animación correspondiente.
 * @param normal duración normal en ms
 * @param reduced duración cuando están reducidas (por defecto 0 = instantáneo)
 */
export function animDuration(normal: number, reduced = 0): number {
	return shouldReduceAnimations() ? reduced : normal;
}
