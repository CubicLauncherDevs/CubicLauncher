/**
 * Cuándo se dibuja un visor 3D. Sin animación no hay bucle: solo se dibuja lo que se pide. Con
 * animación, entre un fotograma y el siguiente la página duerme en un temporizador en lugar de
 * despertar en cada refresco de la pantalla para no hacer nada.
 */

/** Lo que el bucle necesita del navegador; las pruebas le dan un reloj propio. */
export interface FrameClock {
	now(): number;
	requestFrame(callback: () => void): number;
	cancelFrame(id: number): void;
	setTimer(callback: () => void, ms: number): number;
	clearTimer(id: number): void;
}

export interface FrameLoop {
	/** Fotogramas por segundo de la animación; 0 la detiene y no queda nada programado. */
	setRate(fps: number): void;
	/** Dibuja una vez en el próximo refresco, sin avanzar la animación. */
	request(): void;
	dispose(): void;
}

/** Tras una pausa, la animación sigue donde estaba en lugar de dar un salto. */
const LONGEST_STEP_S = 0.1;

/** El refresco que toca llega un poco tarde; pedirlo antes evita perder un fotograma. */
const FRAME_SLACK_MS = 4;

const browserClock: FrameClock = {
	now: () => performance.now(),
	requestFrame: (callback) => requestAnimationFrame(callback),
	cancelFrame: (id) => cancelAnimationFrame(id),
	setTimer: (callback, ms) => setTimeout(callback, ms) as unknown as number,
	clearTimer: (id) => clearTimeout(id),
};

/**
 * @param draw recibe los segundos que avanza la animación; 0 cuando solo hay que volver a dibujar.
 */
export function createFrameLoop(
	draw: (step: number) => void,
	clock: FrameClock = browserClock,
): FrameLoop {
	let interval = 0;
	let frame: number | null = null;
	let timer: number | null = null;
	let wanted = false;
	let lastStep: number | null = null;
	let due = 0;
	let disposed = false;

	function needFrame() {
		if (frame === null) frame = clock.requestFrame(onFrame);
	}

	function waitForDue(now: number) {
		if (timer !== null || interval === 0) return;
		const wait = due - now - FRAME_SLACK_MS;
		if (wait <= 0) {
			needFrame();
			return;
		}
		timer = clock.setTimer(() => {
			timer = null;
			needFrame();
		}, wait);
	}

	function onFrame() {
		frame = null;
		if (disposed) return;
		const now = clock.now();
		const stepping = interval > 0 && now >= due - FRAME_SLACK_MS;
		if (stepping) {
			const elapsed = lastStep === null ? 0 : (now - lastStep) / 1000;
			lastStep = now;
			// Conserva el resto para no quedar por debajo del ritmo pedido.
			due = Math.max(due + interval, now + interval / 2);
			wanted = false;
			draw(Math.min(elapsed, LONGEST_STEP_S));
		} else if (wanted) {
			wanted = false;
			draw(0);
		}
		waitForDue(now);
	}

	return {
		setRate(fps) {
			const next = fps > 0 ? 1000 / fps : 0;
			if (next === interval) return;
			interval = next;
			if (timer !== null) {
				clock.clearTimer(timer);
				timer = null;
			}
			if (interval === 0) {
				lastStep = null;
				if (frame !== null && !wanted) {
					clock.cancelFrame(frame);
					frame = null;
				}
				return;
			}
			due = clock.now();
			needFrame();
		},
		request() {
			wanted = true;
			needFrame();
		},
		dispose() {
			disposed = true;
			if (frame !== null) clock.cancelFrame(frame);
			if (timer !== null) clock.clearTimer(timer);
			frame = null;
			timer = null;
		},
	};
}
