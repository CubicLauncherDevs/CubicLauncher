<script lang="ts">
	import { onMount } from "svelte";
	import type { Render, IdleAnimation } from "skin3d";
	import { launcherStore } from "$lib/state/state.svelte";

	const IDLE_MS = 5000;

	interface Props {
		skinUrl: string;
		capeUrl?: string | null;
		model?: "default" | "slim";
		animated?: boolean;
		interactive?: boolean;
		quality?: "low" | "high";
	}

	let {
		skinUrl,
		capeUrl = null,
		model = "default",
		animated = true,
		interactive = true,
		quality = "low",
	}: Props = $props();

	let container: HTMLElement;
	let viewer = $state<Render | null>(null);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let IdleAnimationClass: typeof IdleAnimation | null = null;

	let isIntersecting = $state(true);
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
		if (!v || v.disposed || !v.renderPaused || pendingRaf !== null) return;
		pendingRaf = requestAnimationFrame(() => {
			pendingRaf = null;
			v.render();
		});
	}

	function syncRenderPause() {
		const v = viewer;
		if (!v || v.disposed) return;
		const shouldPause = !isVisible || !shouldAnimate || isIdle;
		const wasPaused = v.renderPaused;
		v.renderPaused = shouldPause;
		if (shouldPause && !wasPaused) {
			scheduleRender();
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
		if (shouldAnimate && isVisible) {
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
		resetIdleTimer();
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
			const { Render, IdleAnimation } = await import("skin3d");
			if (!mounted) return;

			IdleAnimationClass = IdleAnimation;

			const width = container.clientWidth;
			const height = container.clientHeight || width;

			const instance = new Render({
				width,
				height,
				enableControls: true,
				zoom: 0.7,
				enableFXAA: false,
				maxPixelRatio: quality === "high" ? 2 : 1,
				pixelRatio: quality === "high" ? "match-device" : 1,
				renderPaused: !shouldAnimate,
			});

			// Default skin3d lighting (ambient 3.0 / camera 0.6) washes out skins.
			// A balanced ambient + stronger camera light keeps colors closer to the
			// original texture while preserving shape.
			instance.globalLight.intensity = 2.0;
			instance.cameraLight.intensity = 1.2;

			instance.autoRotate = shouldAnimate;
			instance.animation = shouldAnimate ? new IdleAnimation() : null;
			if (!interactive) {
				instance.controls.enabled = false;
			}

			// Reducir FPS del loop animado a ~30 cuando no hay interacción
			const instanceAny = instance as unknown as {
				draw: () => void;
				animationID: number | null;
			};
			const originalDraw = instanceAny.draw.bind(instance);
			let lastDrawTime = performance.now();
			const targetFrameInterval = 1000 / 30;
			instanceAny.draw = function () {
				const now = performance.now();
				if (
					!isInteracting &&
					now - lastDrawTime < targetFrameInterval
				) {
					instanceAny.animationID = requestAnimationFrame(() =>
						instanceAny.draw(),
					);
					return;
				}
				lastDrawTime = now;
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
					scheduleRender();
				};
				controlsChangeHandler = () => scheduleRender();
				controlsEndHandler = () => {
					isInteracting = false;
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
				if (instance && !instance.disposed) {
					instance.width = container.clientWidth;
					instance.height = container.clientHeight;
					if (instance.renderPaused) scheduleRender();
				}
			});
			resizeObserver.observe(container);

			intersectionObserver = new IntersectionObserver((entries) => {
				isIntersecting = entries[0]?.isIntersecting ?? true;
				syncRenderPause();
				resetIdleTimer();
			});
			intersectionObserver.observe(container);

			document.addEventListener(
				"visibilitychange",
				handleVisibilityChange,
			);
			isTabVisible = !document.hidden;

			viewer = instance;
			syncRenderPause();
			resetIdleTimer();
		}

		init();

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
			viewer?.dispose();
			viewer = null;
			IdleAnimationClass = null;
		};
	});

	$effect(() => {
		const v = viewer;
		if (!v || v.disposed) return;

		v.autoRotate = shouldAnimate;
		v.animation =
			shouldAnimate && IdleAnimationClass
				? new IdleAnimationClass()
				: null;
		syncRenderPause();
		resetIdleTimer();
	});

	$effect(() => {
		const v = viewer;
		if (!v || v.disposed) return;

		const skin = bustCache(skinUrl);
		const cape = capeUrl ? bustCache(capeUrl) : null;
		const m = model;

		// Evita que se vea la textura anterior mientras llega la nueva
		v.resetSkin();
		v.resetCape();

		loading = true;
		error = null;

		const skinPromise = v.loadSkin(skin, { model: m });
		const capePromise = cape ? v.loadCape(cape) : Promise.resolve();

		Promise.all([skinPromise, capePromise])
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
