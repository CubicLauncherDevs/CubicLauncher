<script lang="ts" generics="T">
	import { onMount, onDestroy, untrack } from "svelte";
	import type { Snippet } from "svelte";
	import { observeThemeMetrics } from "$lib/utils/themeMetrics";

	interface Props {
		items: T[];
		itemHeight: number;
		itemHeightVar?: string;
		children: Snippet<[T, number]>;
		class?: string;
		padding?: number;
		onNearEnd?: () => void;
		keyFn?: (item: T) => string | number;
		hideScrollbar?: boolean;
		overscan?: number;
		onRangeNeeded?: (first: number, last: number) => void;
		active?: boolean;
		resetKey?: number;
	}

	let {
		items,
		itemHeight,
		itemHeightVar,
		children,
		class: className = "",
		padding = 20,
		onNearEnd,
		keyFn,
		hideScrollbar = true,
		overscan = 5,
		onRangeNeeded,
		active = true,
		resetKey = 0,
	}: Props = $props();

	let container: HTMLDivElement = $state() as HTMLDivElement;
	let scrollTop = $state(0);
	let containerHeight = $state(0);
	let ticking = false;
	let disposed = false;
	let frame: number | undefined;
	$effect(() => {
		void resetKey;
		if (container) {
			container.scrollTop = 0;
			scrollTop = 0;
		}
	});

	let measuredHeight = $state(0);
	const rowHeight = $derived(measuredHeight || itemHeight);
	$effect(() => {
		if (!container || !itemHeightVar) {
			measuredHeight = 0;
			return;
		}
		return observeThemeMetrics(
			container,
			{
				row: { variable: itemHeightVar, fallback: itemHeight },
			},
			({ row }) => {
				measuredHeight = row;
			},
		);
	});

	const totalHeight = $derived(items.length * rowHeight + padding);

	const startIndex = $derived(
		Math.max(
			0,
			Math.min(items.length - 1, Math.floor(scrollTop / rowHeight)) -
				overscan,
		),
	);
	const endIndex = $derived(
		Math.min(
			items.length - 1,
			Math.floor((scrollTop + containerHeight) / rowHeight) + overscan,
		),
	);

	const visibleSlice = $derived(items.slice(startIndex, endIndex + 1));
	$effect(() => {
		if (!active || !onRangeNeeded || !items.length) return;
		const first = startIndex;
		const last = endIndex;
		// Metadata revisions can change without the item count changing.
		void items;
		untrack(() => onRangeNeeded?.(first, last));
	});

	function handleScroll(e: Event) {
		const target = e.target as HTMLDivElement;
		if (!ticking) {
			frame = requestAnimationFrame(() => {
				frame = undefined;
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
		if (frame !== undefined) cancelAnimationFrame(frame);
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
					rowHeight}px); left: 0; width: 100%; height: {rowHeight}px; --virtual-row-height: {rowHeight}px;"
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
