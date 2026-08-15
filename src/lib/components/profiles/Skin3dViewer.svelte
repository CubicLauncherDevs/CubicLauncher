<script lang="ts">
	import { onMount } from "svelte";
	import type { Render } from "skin3d";

	interface Props {
		skinUrl: string;
		capeUrl?: string | null;
		model?: "default" | "slim";
	}

	let { skinUrl, capeUrl = null, model = "default" }: Props = $props();

	let container: HTMLElement;
	let viewer = $state<Render | null>(null);

	onMount(() => {
		let mounted = true;
		let resizeObserver: ResizeObserver | null = null;

		async function init() {
			const { Render, IdleAnimation } = await import("skin3d");
			if (!mounted) return;

			const width = container.clientWidth;
			const height = container.clientHeight || width;

			const instance = new Render({
				width,
				height,
				enableControls: true,
			});

			instance.autoRotate = true;
			instance.animation = new IdleAnimation();
			// eslint-disable-next-line svelte/no-dom-manipulating
			container.appendChild(instance.canvas);

			resizeObserver = new ResizeObserver(() => {
				if (instance && !instance.disposed) {
					instance.width = container.clientWidth;
					instance.height = container.clientHeight;
				}
			});
			resizeObserver.observe(container);

			viewer = instance;
		}

		init();

		return () => {
			mounted = false;
			resizeObserver?.disconnect();
			viewer?.dispose();
		};
	});

	$effect(() => {
		const v = viewer;
		if (!v || v.disposed) return;

		v.loadSkin(skinUrl, { model });

		if (capeUrl) {
			v.loadCape(capeUrl);
		} else {
			v.loadCape(null);
		}
	});
</script>

<div bind:this={container} class="skin-3d-viewer"></div>

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
	}
</style>
