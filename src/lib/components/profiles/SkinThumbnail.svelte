<script lang="ts">
	import { requestThumbnail } from "$lib/utils/skinThumbnails";
	import {
		THUMBNAIL_CANCELLED,
		type FigureModel,
	} from "$lib/utils/thumbnailQueue";

	interface Props {
		skinUrl: string;
		model?: FigureModel;
	}

	let { skinUrl, model = "default" }: Props = $props();

	let src = $state<string | null>(null);
	let error = $state<string | null>(null);

	$effect(() => {
		const job = requestThumbnail(skinUrl, model);
		let current = true;
		src = null;
		error = null;
		job.promise
			.then((url) => {
				if (current) src = url;
			})
			.catch((e: Error) => {
				if (current && e.message !== THUMBNAIL_CANCELLED) {
					error = String(e);
				}
			});
		return () => {
			current = false;
			job.cancel();
		};
	});
</script>

<div class="skin-thumbnail">
	{#if src}
		<img {src} alt="" decoding="async" draggable="false" />
	{:else if error}
		<div class="error">{error}</div>
	{:else}
		<div class="loader"><span class="spinner"></span></div>
	{/if}
</div>

<style>
	.skin-thumbnail {
		width: 100%;
		height: 100%;
		position: relative;
		overflow: hidden;
	}

	/* La altura manda, como en la vista 3D: la figura mide lo mismo en una tarjeta ancha o estrecha. */
	img {
		display: block;
		position: absolute;
		top: 0;
		left: 50%;
		height: 100%;
		aspect-ratio: 1;
		transform: translateX(-50%);
	}

	.loader {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		pointer-events: none;
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
		pointer-events: none;
	}
</style>
