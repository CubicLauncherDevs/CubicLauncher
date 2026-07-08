<script lang="ts">
	import { fly } from "svelte/transition";
	import { onMount } from "svelte";

	export interface ContextMenuItem {
		label: string;
		action: () => void;
	}

	let {
		open = $bindable(false),
		x = 0,
		y = 0,
		items = [] as ContextMenuItem[],
	} = $props<{
		open: boolean;
		x: number;
		y: number;
		items: ContextMenuItem[];
	}>();

	let containerEl = $state<HTMLDivElement>();

	function portal(el: HTMLElement) {
		document.body.appendChild(el);
		return {
			destroy() {
				el.remove();
			},
		};
	}

	function handleClickOutside(event: MouseEvent) {
		if (containerEl && !containerEl.contains(event.target as Node)) {
			open = false;
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") {
			open = false;
		}
	}

	onMount(() => {
		window.addEventListener("click", handleClickOutside);
		window.addEventListener("keydown", handleKeydown);
		return () => {
			window.removeEventListener("click", handleClickOutside);
			window.removeEventListener("keydown", handleKeydown);
		};
	});
</script>

{#if open}
	<div use:portal>
		<div
			bind:this={containerEl}
			class="ctx-menu"
			style="left: {x}px; top: {y}px;"
			transition:fly={{ y: -4, duration: 120 }}
		>
			{#each items as item (item.label)}
				<button
					type="button"
					class="ctx-item"
					onclick={() => {
						item.action();
						open = false;
					}}
				>
					{item.label}
				</button>
			{/each}
		</div>
	</div>
{/if}

<style>
	.ctx-menu {
		position: fixed;
		z-index: 9999;
		min-width: 180px;
		background: var(--bg-surface, #1e1e1e);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm, 6px);
		padding: 4px;
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
	}

	.ctx-item {
		display: block;
		width: 100%;
		padding: 8px 12px;
		background: transparent;
		border: none;
		color: var(--text-primary);
		font-size: 0.8rem;
		font-weight: 500;
		text-align: left;
		cursor: pointer;
		border-radius: 4px;
		transition: background 0.1s ease;
	}

	.ctx-item:hover {
		background: var(--bg-item-active, rgba(255, 255, 255, 0.06));
	}

	.ctx-item:not(:last-child) {
		margin-bottom: 2px;
	}
</style>
