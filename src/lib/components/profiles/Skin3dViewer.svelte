<script lang="ts">
	import { onMount, untrack } from "svelte";
	import type { Render, IdleAnimation, WalkingAnimation } from "skin3d";
	import { launcherStore } from "$lib/state/state.svelte";
	import {
		EYE_GAP,
		EYE_STEPS,
		eyePatchesOf,
		moveOf,
		poseAt,
		poseKeyOf,
		readEyeCopy,
		type LiveEyes,
	} from "$lib/utils/freshEyes";

	const IDLE_MS = 5000;

	interface Props {
		skinUrl: string;
		capeUrl?: string | null;
		model?: "default" | "slim";
		animated?: boolean;
		interactive?: boolean;
		quality?: "low" | "high";
		/** Si la figura gira sola. */
		rotate?: boolean;
		/** Si camina en lugar de estar quieta respirando. */
		walk?: boolean;
		/** Si mueve los ojos de Fresh Moves, cuando la skin lleva su copia. */
		eyes?: boolean;
		/** Avisa, al cargar cada skin, de si lleva ojos de Fresh Moves que se pueden mover. */
		onEyesFound?: (found: boolean) => void;
	}

	let {
		skinUrl,
		capeUrl = null,
		model = "default",
		animated = true,
		interactive = true,
		quality = "low",
		rotate = true,
		walk = false,
		eyes = false,
		onEyesFound,
	}: Props = $props();

	let container: HTMLElement;
	let viewer = $state<Render | null>(null);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let IdleAnimationClass: typeof IdleAnimation | null = null;
	let WalkingAnimationClass: typeof WalkingAnimation | null = null;

	let isIntersecting = $state(false);
	let isTabVisible = $state(true);
	let pendingRaf: number | null = null;
	let idleTimeout: ReturnType<typeof setTimeout> | null = null;
	let isInteracting = false;
	let isIdle = false;
	let skinLoadGeneration = 0;
	const isVisible = $derived(isIntersecting && isTabVisible);

	const shouldAnimate = $derived(
		animated && !launcherStore.settings.disable_skin3d_animations,
	);

	function scheduleRender() {
		const v = viewer;
		if (
			!v ||
			v.disposed ||
			!isVisible ||
			!v.renderPaused ||
			pendingRaf !== null
		)
			return;
		pendingRaf = requestAnimationFrame(() => {
			pendingRaf = null;
			if (!v.disposed && isVisible && v.renderPaused) v.render();
		});
	}

	function syncRenderPause() {
		const v = viewer;
		if (!v || v.disposed) return;
		const shouldPause = !isVisible || !shouldAnimate || isIdle;
		v.renderPaused = shouldPause;
		if ((!isVisible || !shouldPause) && pendingRaf !== null) {
			cancelAnimationFrame(pendingRaf);
			pendingRaf = null;
		}
	}

	function stopIdleTimer() {
		if (idleTimeout !== null) {
			clearTimeout(idleTimeout);
			idleTimeout = null;
		}
	}

	function resetIdleTimer() {
		stopIdleTimer();
		isIdle = false;
		if (shouldAnimate && isVisible && !isInteracting) {
			idleTimeout = setTimeout(() => {
				isIdle = true;
				syncRenderPause();
			}, IDLE_MS);
		}
	}

	function wakeFromIdle() {
		resetIdleTimer();
		syncRenderPause();
	}

	function handleVisibilityChange() {
		isTabVisible = !document.hidden;
	}

	/** Lo más cerca que llega la cámara al punto que mira: no entra en la figura. */
	const NEAREST = 10;

	/**
	 * A qué altura gira y hacia dónde acerca la vista: el pecho (en skin3d el torso va de -4 a 8,
	 * la figura de -16 a 16). El centro de la figura queda a la altura de la cintura.
	 */
	const CHEST_Y = 4;

	/* Ojos de Fresh Moves. */

	/** Los ojos de la skin cargada, y la skin tal como llegó, para volver a ella. */
	let liveEyes = $state<LiveEyes | null>(null);
	let plainSkin: HTMLCanvasElement | null = null;
	/** La cara de la skin a `EYE_STEPS` por píxel, sin mover: cada momento se pinta encima. */
	let restingFace: ImageData | null = null;
	let eyesTimer: ReturnType<typeof setTimeout> | null = null;
	let eyesFrame: number | null = null;

	/** Lee la copia de los ojos de la skin recién cargada; sin copia, o sin poder leerla, nada. */
	function readEyes(v: Render) {
		stopEyes(v);
		liveEyes = null;
		plainSkin = null;
		try {
			const canvas = v.skinCanvas;
			const context = canvas.getContext("2d", {
				willReadFrequently: true,
			});
			if (!context) return;
			const { data } = context.getImageData(
				0,
				0,
				canvas.width,
				canvas.height,
			);
			const found = readEyeCopy(data, canvas.width, canvas.height);
			if (found) {
				plainSkin = document.createElement("canvas");
				plainSkin.width = canvas.width;
				plainSkin.height = canvas.height;
				plainSkin.getContext("2d")!.drawImage(canvas, 0, 0);
			}
			liveEyes = found;
		} catch {
			// Una imagen de otro origen sin CORS no se deja leer: sin ojos.
			liveEyes = null;
		} finally {
			onEyesFound?.(liveEyes !== null);
		}
	}

	/**
	 * Empieza a mover los ojos: la skin pasa a la vista `EYE_STEPS` veces más grande, para que
	 * la pupila se deslice por partes de un píxel como en mosaicary.com, y en cada momento de
	 * un movimiento solo se vuelve a pintar la cara. Entre movimientos no se pinta nada, y
	 * con la vista en reposo se dibuja un único fotograma por cambio.
	 */
	function startEyes(v: Render, found: LiveEyes) {
		if (!plainSkin || eyesTimer !== null || eyesFrame !== null) return;
		const big = document.createElement("canvas");
		big.width = plainSkin.width * EYE_STEPS;
		big.height = plainSkin.height * EYE_STEPS;
		const bigContext = big.getContext("2d")!;
		bigContext.imageSmoothingEnabled = false;
		bigContext.drawImage(plainSkin, 0, 0, big.width, big.height);
		v.loadSkin(big, { model });
		const context = v.skinCanvas.getContext("2d", {
			willReadFrequently: true,
		});
		if (!context) return;
		const faceAt = 8 * EYE_STEPS;
		restingFace = context.getImageData(faceAt, faceAt, faceAt, faceAt);
		const rows = found.rows.length;
		let shown = poseKeyOf({ look: 0, lid: 0 }, rows);

		const paint = (pose: { look: number; lid: number }) => {
			const key = poseKeyOf(pose, rows);
			if (key === shown || !restingFace || v.disposed) return;
			shown = key;
			context.putImageData(restingFace, faceAt, faceAt);
			for (const patch of eyePatchesOf(found, pose)) {
				const [r, g, b] = patch.colour;
				context.fillStyle = `rgb(${r}, ${g}, ${b})`;
				const x0 = Math.round(patch.x * EYE_STEPS);
				const y0 = Math.round(patch.y * EYE_STEPS);
				context.fillRect(
					faceAt + x0,
					faceAt + y0,
					Math.round((patch.x + patch.w) * EYE_STEPS) - x0,
					Math.round((patch.y + patch.h) * EYE_STEPS) - y0,
				);
			}
			// skin3d no expone la textura en sus tipos; es la de skinCanvas.
			const texture = (
				v as unknown as { skinTexture: { needsUpdate: boolean } | null }
			).skinTexture;
			if (texture) texture.needsUpdate = true;
			scheduleRender();
		};

		const next = () => {
			eyesTimer = setTimeout(
				() => {
					eyesTimer = null;
					// Fuera de la vista, el siguiente movimiento espera.
					if (!isVisible) return next();
					const keys = moveOf(rows);
					const end = keys[keys.length - 1]!.at;
					let start = -1;
					const step = (now: number) => {
						if (start < 0) start = now;
						const time = now - start;
						paint(poseAt(keys, Math.min(time, end)));
						if (time < end) {
							eyesFrame = requestAnimationFrame(step);
						} else {
							eyesFrame = null;
							next();
						}
					};
					eyesFrame = requestAnimationFrame(step);
				},
				EYE_GAP.least + Math.random() * EYE_GAP.more,
			);
		};
		next();
	}

	/** Deja los ojos quietos y la skin como llegó. */
	function stopEyes(v: Render | null) {
		if (eyesTimer !== null) clearTimeout(eyesTimer);
		if (eyesFrame !== null) cancelAnimationFrame(eyesFrame);
		eyesTimer = null;
		eyesFrame = null;
		if (restingFace && plainSkin && v && !v.disposed) {
			v.loadSkin(plainSkin, { model });
			scheduleRender();
		}
		restingFace = null;
	}

	function bustCache(url: string): string {
		if (url.startsWith("data:")) return url;
		const sep = url.includes("?") ? "&" : "?";
		// Use a small per-component generation counter instead of Date.now() so
		// the browser image cache is not flooded with unique URLs on every render.
		return `${url}${sep}_skin3d=${++skinLoadGeneration}`;
	}

	onMount(() => {
		let mounted = true;
		let resizeObserver: ResizeObserver | null = null;
		let intersectionObserver: IntersectionObserver | null = null;
		let controlsStartHandler: (() => void) | null = null;
		let controlsChangeHandler: (() => void) | null = null;
		let controlsEndHandler: (() => void) | null = null;
		let activityHandler: (() => void) | null = null;

		async function init() {
			const { Render, IdleAnimation, WalkingAnimation } =
				await import("skin3d");
			if (!mounted) return;

			IdleAnimationClass = IdleAnimation;
			WalkingAnimationClass = WalkingAnimation;

			const width = Math.max(1, container.clientWidth);
			const height = container.clientHeight || width;

			const instance = new Render({
				width,
				height,
				enableControls: true,
				zoom: 0.7,
				// Keep skin3d's original visual quality in the main preview.
				enableFXAA: quality === "high",
				maxPixelRatio: quality === "high" ? 2 : 1,
				pixelRatio: quality === "high" ? "match-device" : 1,
				// Wait for visibility before starting any animation work.
				renderPaused: true,
			});

			if (!interactive) {
				instance.controls.enabled = false;
			}
			instance.controls.minDistance = NEAREST;
			// Sin desplazar con el botón derecho: el centro de giro se queda en el pecho.
			instance.controls.enablePan = false;
			// El centro de giro en el pecho; la cámara sube lo mismo, y la vista no se inclina.
			instance.controls.target.set(0, CHEST_Y, 0);
			instance.camera.position.y += CHEST_Y;
			instance.controls.update();

			// Reducir FPS del loop animado a ~30 cuando no hay interacción
			const instanceAny = instance as unknown as {
				draw: () => void;
				animationID: number | null;
			};
			const originalDraw = instanceAny.draw.bind(instance);
			let lastDrawTime = performance.now();
			const targetFrameInterval = 1000 / 30;
			instanceAny.draw = function () {
				if (instance.disposed || instance.renderPaused) return;
				const now = performance.now();
				const elapsed = now - lastDrawTime;
				if (!isInteracting && elapsed < targetFrameInterval) {
					instanceAny.animationID = requestAnimationFrame(() =>
						instanceAny.draw(),
					);
					return;
				}
				// Carry timing remainder forward to avoid drifting below 30 FPS.
				lastDrawTime = isInteracting
					? now
					: now - (elapsed % targetFrameInterval);
				originalDraw();
			};

			// eslint-disable-next-line svelte/no-dom-manipulating
			container.appendChild(instance.canvas);

			activityHandler = () => wakeFromIdle();
			const activityEvents = [
				"pointerenter",
				"pointerdown",
				"pointermove",
				"wheel",
				"touchstart",
			];
			for (const event of activityEvents) {
				container.addEventListener(event, activityHandler, {
					passive: true,
				});
			}

			if (interactive) {
				controlsStartHandler = () => {
					isInteracting = true;
					wakeFromIdle();
					scheduleRender();
				};
				controlsChangeHandler = () => scheduleRender();
				controlsEndHandler = () => {
					isInteracting = false;
					wakeFromIdle();
				};
				instance.controls.addEventListener(
					"start",
					controlsStartHandler,
				);
				instance.controls.addEventListener(
					"change",
					controlsChangeHandler,
				);
				instance.controls.addEventListener("end", controlsEndHandler);
			}

			resizeObserver = new ResizeObserver(() => {
				if (instance.disposed) return;
				const width = container.clientWidth;
				const height = container.clientHeight;
				if (
					width > 0 &&
					height > 0 &&
					(width !== instance.width || height !== instance.height)
				) {
					// Resize GPU buffers once, only when dimensions actually change.
					instance.setSize(width, height);
					scheduleRender();
				}
			});
			resizeObserver.observe(container);

			intersectionObserver = new IntersectionObserver((entries) => {
				isIntersecting = entries[0]?.isIntersecting ?? false;
			});
			intersectionObserver.observe(container);

			document.addEventListener(
				"visibilitychange",
				handleVisibilityChange,
			);
			isTabVisible = !document.hidden;

			viewer = instance;
		}

		init().catch((e) => {
			if (mounted) error = String(e);
		});

		return () => {
			mounted = false;
			stopIdleTimer();
			document.removeEventListener(
				"visibilitychange",
				handleVisibilityChange,
			);
			if (activityHandler && container) {
				const activityEvents = [
					"pointerenter",
					"pointerdown",
					"pointermove",
					"wheel",
					"touchstart",
				];
				for (const event of activityEvents) {
					container.removeEventListener(event, activityHandler);
				}
			}
			if (viewer && !viewer.disposed) {
				if (controlsStartHandler) {
					viewer.controls.removeEventListener(
						"start",
						controlsStartHandler,
					);
				}
				if (controlsChangeHandler) {
					viewer.controls.removeEventListener(
						"change",
						controlsChangeHandler,
					);
				}
				if (controlsEndHandler) {
					viewer.controls.removeEventListener(
						"end",
						controlsEndHandler,
					);
				}
			}
			resizeObserver?.disconnect();
			intersectionObserver?.disconnect();
			if (pendingRaf !== null) {
				cancelAnimationFrame(pendingRaf);
				pendingRaf = null;
			}
			if (eyesTimer !== null) clearTimeout(eyesTimer);
			if (eyesFrame !== null) cancelAnimationFrame(eyesFrame);
			viewer?.dispose();
			viewer = null;
			IdleAnimationClass = null;
			WalkingAnimationClass = null;
		};
	});

	$effect(() => {
		const v = viewer;
		if (!v || v.disposed) return;

		const Animation = walk ? WalkingAnimationClass : IdleAnimationClass;
		v.animation = shouldAnimate && Animation ? new Animation() : null;
		// Un interruptor recién tocado se ve en marcha, aunque la vista estuviera en reposo.
		untrack(() => wakeFromIdle());
	});

	// Los ojos se mueven si se piden, la skin los lleva y las animaciones 3D están activas.
	$effect(() => {
		const v = viewer;
		const found = liveEyes;
		if (!v || v.disposed || !found) return;
		if (eyes && shouldAnimate) untrack(() => startEyes(v, found));
		else untrack(() => stopEyes(v));
	});

	// Aparte de la animación, para que pararlo y reanudarlo no la reinicie.
	$effect(() => {
		const v = viewer;
		if (!v || v.disposed) return;
		v.autoRotate = shouldAnimate && rotate;
	});

	// Visibility changes should resume the current pose, not recreate animation.
	$effect(() => {
		resetIdleTimer();
		syncRenderPause();
		// Redraw static previews on return, including textures loaded while hidden.
		scheduleRender();
	});

	$effect(() => {
		const v = viewer;
		if (!v || v.disposed) return;

		const skin = bustCache(skinUrl);
		const cape = capeUrl ? bustCache(capeUrl) : null;
		const m = model;

		untrack(() => {
			stopEyes(null);
			liveEyes = null;
		});
		// Evita que se vea la textura anterior mientras llega la nueva
		v.resetSkin();
		v.resetCape();

		loading = true;
		error = null;

		const skinPromise = v.loadSkin(skin, { model: m });
		const capePromise = cape ? v.loadCape(cape) : Promise.resolve();

		Promise.all([skinPromise, capePromise])
			.then(() => {
				if (!v.disposed) untrack(() => readEyes(v));
			})
			.catch((e) => {
				error = String(e);
			})
			.finally(() => {
				loading = false;
				scheduleRender();
			});
	});
</script>

<div bind:this={container} class="skin-3d-viewer" class:loading>
	{#if loading}
		<div class="loader">
			<span class="spinner"></span>
		</div>
	{/if}
	{#if error}
		<div class="error">{error}</div>
	{/if}
</div>

<style>
	.skin-3d-viewer {
		min-width: 180px;
		min-height: 260px;
		width: 100%;
		height: 100%;
		position: relative;
		overflow: hidden;
	}

	.skin-3d-viewer :global(canvas) {
		display: block;
		width: 100%;
		height: 100%;
		border-radius: var(--border-radius);
		transition: opacity 150ms ease;
	}

	.skin-3d-viewer.loading :global(canvas) {
		opacity: 0.2;
	}

	.loader {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		pointer-events: none;
		background: transparent;
		z-index: 1;
	}

	.spinner {
		width: 24px;
		height: 24px;
		border: 3px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.error {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 12px;
		text-align: center;
		font-size: 0.75rem;
		color: var(--error, #ff6b6b);
		background: var(--bg-card);
		border-radius: var(--border-radius);
		pointer-events: none;
		z-index: 2;
	}
</style>
