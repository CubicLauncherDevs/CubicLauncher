<script lang="ts" generics="T">
	import { onMount } from "svelte";
	import type { Snippet } from "svelte";

	interface Props {
		items: T[];
		itemHeight: number;
		children: Snippet<[T, number]>;
		class?: string;
		padding?: number;
		onNearEnd?: () => void;
	}

	let {
		items,
		itemHeight,
		children,
		class: className = "",
		padding = 20,
		onNearEnd,
	}: Props = $props();

	let container: HTMLDivElement = $state() as HTMLDivElement;
	let scrollTop = $state(0);
	let containerHeight = $state(0);
	let ticking = false;

	const totalHeight = $derived(items.length * itemHeight + padding);

	const buffer = 5;

	const startIndex = $derived(Math.max(0, Math.floor(scrollTop / itemHeight) - buffer));
	const endIndex = $derived(
		Math.min(items.length - 1, Math.floor((scrollTop + containerHeight) / itemHeight) + buffer),
	);

	const visibleSlice = $derived(items.slice(startIndex, endIndex + 1));

	function handleScroll(e: Event) {
		const target = e.target as HTMLDivElement;
		if (!ticking) {
			requestAnimationFrame(() => {
				scrollTop = target.scrollTop;
				if (target.scrollHeight - scrollTop - containerHeight < 500) {
					onNearEnd?.();
				}
				ticking = false;
			});
			ticking = true;
		}
	}

	onMount(() => {
		const resizeObserver = new ResizeObserver((entries) => {
			for (let entry of entries) {
				containerHeight = entry.contentRect.height;
			}
		});
		resizeObserver.observe(container);
		return () => resizeObserver.disconnect();
	});
</script>

<div
	bind:this={container}
	class="virtual-list-container {className}"
	onscroll={handleScroll}
	style="position: relative; overflow-y: auto; height: 100%;"
>
	<div
		class="virtual-list-phantom"
		style="height: {totalHeight}px; width: 100%; pointer-events: none;"
	></div>
	<div
		class="virtual-list-content"
		style="position: absolute; top: 0; left: 0; width: 100%;"
	>
		{#each visibleSlice as item, idx (startIndex + idx)}
			{@const index = startIndex + idx}
			<div
				class="virtual-list-item-wrapper"
				style="position: absolute; transform: translateY({index * itemHeight}px); left: 0; width: 100%; height: {itemHeight}px;"
			>
				{@render children(item, index)}
			</div>
		{/each}
	</div>
</div>

<style>
	.virtual-list-container {
		scrollbar-width: thin;
		scrollbar-color: var(--border) transparent;
	}

	.virtual-list-container:global(::-webkit-scrollbar) {
		width: 6px;
	}

	.virtual-list-container:global(::-webkit-scrollbar-thumb) {
		background: var(--border);
		border-radius: 10px;
	}
</style>
