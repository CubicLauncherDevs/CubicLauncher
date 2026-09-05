<script lang="ts">
	import { onMount } from "svelte";
	import type { Render, IdleAnimation } from "skin3d";
	import { launcherStore } from "$lib/state/state.svelte";

	interface Props {
		skinUrl: string;
		capeUrl?: string | null;
		model?: "default" | "slim";
		animated?: boolean;
	}

	let {
		skinUrl,
		capeUrl = null,
		model = "default",
		animated = true,
	}: Props = $props();

	let container: HTMLElement;
	let viewer = $state<Render | null>(null);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let IdleAnimationClass: typeof IdleAnimation | null = null;

	const shouldAnimate = $derived(
		animated && !launcherStore.settings.disable_skin3d_animations,
	);

	function bustCache(url: string): string {
		if (url.startsWith("data:")) return url;
		const sep = url.includes("?") ? "&" : "?";
		return `${url}${sep}_skin3d=${Date.now()}`;
	}

	onMount(() => {
		let mounted = true;
		let resizeObserver: ResizeObserver | null = null;
		let intersectionObserver: IntersectionObserver | null = null;

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
			});

			instance.autoRotate = shouldAnimate;
			instance.animation = shouldAnimate ? new IdleAnimation() : null;
			// eslint-disable-next-line svelte/no-dom-manipulating
			container.appendChild(instance.canvas);

			resizeObserver = new ResizeObserver(() => {
				if (instance && !instance.disposed) {
					instance.width = container.clientWidth;
					instance.height = container.clientHeight;
				}
			});
			resizeObserver.observe(container);

			intersectionObserver = new IntersectionObserver((entries) => {
				if (instance && !instance.disposed) {
					instance.renderPaused = !entries[0]?.isIntersecting;
				}
			});
			intersectionObserver.observe(container);

			viewer = instance;
		}

		init();

		return () => {
			mounted = false;
			resizeObserver?.disconnect();
			intersectionObserver?.disconnect();
			viewer?.dispose();
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
