<script lang="ts">
	import { onMount } from "svelte";
	import type { LogRenderer } from "./LogRenderer";

	interface Props {
		renderer: LogRenderer;
		onScrollState?: (state: {
			isAtBottom: boolean;
			unseenCount: number;
		}) => void;
	}

	let { renderer, onScrollState }: Props = $props();
	let viewport: HTMLDivElement | undefined = $state();

	onMount(() => {
		if (!viewport) return;
		renderer.attach(viewport);
		const handler = () => renderer.handleScroll(onScrollState);
		viewport.addEventListener("scroll", handler, { passive: true });
		return () => {
			renderer.detach();
			viewport?.removeEventListener("scroll", handler);
		};
	});
</script>

<div class="log-viewport" bind:this={viewport}>
	<div class="log-lines"></div>
</div>

<style>
	.log-viewport {
		flex: 1;
		overflow-y: auto;
		contain: layout style;
		background: var(--bg-input);
		padding: 4px 0;
	}

	.log-viewport::-webkit-scrollbar {
		width: 5px;
	}

	.log-viewport::-webkit-scrollbar-track {
		background: var(--scrollbar-track, transparent);
	}

	.log-viewport::-webkit-scrollbar-thumb {
		background: var(--scrollbar-thumb);
		border-radius: 10px;
	}
</style>
