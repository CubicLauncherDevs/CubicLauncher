<script lang="ts">
	import { fly } from "svelte/transition";
	import type { Snippet } from "svelte";
	import { animDuration } from "$lib/utils/animations";

	interface Props {
		open?: boolean;
		x?: number;
		y?: number;
		placement?: "right" | "left" | "top" | "bottom";
		children?: Snippet;
	}

	let {
		open = $bindable(false),
		x = 0,
		y = 0,
		placement = "right",
		children,
	}: Props = $props();

	let containerEl = $state<HTMLDivElement>();
	let adjustedX = $state(0);
	let adjustedY = $state(0);

	const flyDuration = $derived(animDuration(120));

	const GAP = 8;
	const PADDING = 8;

	$effect(() => {
		if (!open || !containerEl) return;

		const rect = containerEl.getBoundingClientRect();
		const maxX = window.innerWidth - rect.width - PADDING;
		const maxY = window.innerHeight - rect.height - PADDING;

		let nx = x;
		let ny = y;

		if (placement === "right") {
			nx += GAP;
		} else if (placement === "left") {
			nx -= rect.width + GAP;
		} else if (placement === "bottom") {
			ny += GAP;
		} else if (placement === "top") {
			ny -= rect.height + GAP;
		}

		adjustedX = Math.max(PADDING, Math.min(nx, maxX));
		adjustedY = Math.max(PADDING, Math.min(ny, maxY));
	});

	function portal(el: HTMLElement) {
		document.body.appendChild(el);
		return {
			destroy() {
				el.remove();
			},
		};
	}
</script>

{#if open}
	<div use:portal>
		<div
			bind:this={containerEl}
			class="tooltip"
			role="tooltip"
			style="left: {adjustedX}px; top: {adjustedY}px;"
			transition:fly={{ y: -4, duration: flyDuration }}
		>
			{@render children?.()}
		</div>
	</div>
{/if}

<style>
	.tooltip {
		position: fixed;
		z-index: 9999;
		min-width: 140px;
		max-width: 220px;
		padding: 8px 10px;
		background: var(--bg-surface, #1e1e1e);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm, 6px);
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
		backdrop-filter: blur(var(--backdrop-blur-dropdown, 4px));
		pointer-events: none;
	}
</style>
