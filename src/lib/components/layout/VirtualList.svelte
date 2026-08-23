<script lang="ts" generics="T">
	import { onMount, onDestroy } from "svelte";
	import type { Snippet } from "svelte";

	interface Props {
		items: T[];
		itemHeight: number;
		children: Snippet<[T, number]>;
		class?: string;
		padding?: number;
		onNearEnd?: () => void;
		keyFn?: (item: T) => string | number;
		hideScrollbar?: boolean;
		overscan?: number;
	}

	let {
		items,
		itemHeight,
		children,
		class: className = "",
		padding = 20,
		onNearEnd,
		keyFn,
		hideScrollbar = true,
		overscan = 5,
	}: Props = $props();

	let container: HTMLDivElement = $state() as HTMLDivElement;
	let scrollTop = $state(0);
	let containerHeight = $state(0);
	let ticking = false;
	let disposed = false;

	const totalHeight = $derived(items.length * itemHeight + padding);

	const startIndex = $derived(
		Math.max(0, Math.floor(scrollTop / itemHeight) - overscan),
	);
	const endIndex = $derived(
		Math.min(
			items.length - 1,
			Math.floor((scrollTop + containerHeight) / itemHeight) + overscan,
		),
	);

	const visibleSlice = $derived(items.slice(startIndex, endIndex + 1));

	function handleScroll(e: Event) {
		const target = e.target as HTMLDivElement;
		if (!ticking) {
			requestAnimationFrame(() => {
				if (disposed) return;
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

	onDestroy(() => {
		disposed = true;
	});
</script>

<div
	bind:this={container}
	class="virtual-list-container {className}"
	class:hide-scrollbar={hideScrollbar}
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
		{#each visibleSlice as item, idx (keyFn ? keyFn(item) : startIndex + idx)}
			{@const index = startIndex + idx}
			<div
				class="virtual-list-item-wrapper"
				style="position: absolute; transform: translateY({index *
					itemHeight}px); left: 0; width: 100%; height: {itemHeight}px;"
			>
				{@render children(item, index)}
			</div>
		{/each}
	</div>
</div>

<style>
	.virtual-list-container.hide-scrollbar {
		scrollbar-width: none;
	}

	.virtual-list-container.hide-scrollbar::-webkit-scrollbar {
		display: none;
	}
</style>
