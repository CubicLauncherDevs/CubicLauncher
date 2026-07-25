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
		if (viewport) renderer.attach(viewport);
		return () => renderer.detach();
	});
</script>

<div
	class="log-viewport"
	bind:this={viewport}
	onscroll={() => renderer.handleScroll(onScrollState)}
>
	<div class="log-lines"></div>
</div>

<style>
	.log-viewport {
		flex: 1;
		overflow-y: auto;
		contain: layout style;
	}

	.log-viewport::-webkit-scrollbar {
		width: 4px;
	}

	.log-viewport::-webkit-scrollbar-thumb {
		background: var(--bg-item-active, #333);
		border-radius: 4px;
	}
</style>
