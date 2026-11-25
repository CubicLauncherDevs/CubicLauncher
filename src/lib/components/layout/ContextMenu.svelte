<script lang="ts">
	import { fly } from "svelte/transition";
	import { onMount } from "svelte";
	import Icon from "#lib/icons/Icon.svelte";
	import { animDuration } from "#lib/utils/animations.js";

	export interface ContextMenuItem {
		label: string;
		iconName?: string;
		action?: () => void | Promise<void>;
		variant?: "default" | "danger";
		separator?: boolean;
		disabled?: boolean;
	}

	let {
		open = $bindable(false),
		x = 0,
		y = 0,
		items = [] as ContextMenuItem[],
	}: {
		open: boolean;
		x: number;
		y: number;
		items: ContextMenuItem[];
	} = $props();

	let containerEl = $state<HTMLDivElement>();
	let adjustedX = $state(0);
	let adjustedY = $state(0);

	$effect(() => {
		if (!open || !containerEl) return;

		const rect = containerEl.getBoundingClientRect();
		const padding = 8;
		const maxX = window.innerWidth - rect.width - padding;
		const maxY = window.innerHeight - rect.height - padding;

		adjustedX = Math.max(padding, Math.min(x, maxX));
		adjustedY = Math.max(padding, Math.min(y, maxY));
	});

	function portal(el: HTMLElement) {
		document.body.appendChild(el);
		return {
			destroy() {
				el.remove();
			},
		};
	}

	function handleClickOutside(event: MouseEvent) {
		if (event.defaultPrevented) return;
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
		window.addEventListener("contextmenu", handleClickOutside);
		window.addEventListener("keydown", handleKeydown);
		return () => {
			window.removeEventListener("click", handleClickOutside);
			window.removeEventListener("contextmenu", handleClickOutside);
			window.removeEventListener("keydown", handleKeydown);
		};
	});

	const flyDuration = $derived(animDuration(120));
</script>

{#if open}
	<div use:portal>
		<div
			bind:this={containerEl}
			class="ctx-menu"
			style="left: {adjustedX}px; top: {adjustedY}px;"
			transition:fly={{ y: -4, duration: flyDuration }}
			role="menu"
		>
			{#each items as item, index (item.label + index)}
				{#if item.separator}
					<div class="ctx-separator"></div>
				{:else}
					<button
						type="button"
						class="ctx-item"
						class:danger={item.variant === "danger"}
						class:disabled={item.disabled}
						disabled={item.disabled}
						role="menuitem"
						onclick={() => {
							item.action?.();
							open = false;
						}}
					>
						{#if item.iconName}
							<span class="ctx-icon"
								><Icon name={item.iconName} size={14} /></span
							>
						{/if}
						<span class="ctx-label">{item.label}</span>
					</button>
				{/if}
			{/each}
		</div>
	</div>
{/if}

<style>
	.ctx-menu {
		position: fixed;
		z-index: 9999;
		min-width: 180px;
		max-width: 240px;
		max-height: min(60vh, 420px);
		overflow-y: auto;
		background: var(--bg-surface, #1e1e1e);
		border: var(--border-width) solid var(--border);
		border-radius: var(--border-radius);
		padding: 5px;
		box-shadow: var(--shadow-floating);
		backdrop-filter: blur(var(--backdrop-blur-dropdown, 4px));
	}

	.ctx-item {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 7px 10px;
		background: transparent;
		border: none;
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
		font-size: var(--font-size-sm);
		font-weight: 500;
		line-height: 1.3;
		text-align: left;
		cursor: pointer;
		transition: background-color 0.12s ease;
	}

	.ctx-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		width: 16px;
		opacity: 0.85;
		transition: opacity 0.12s ease;
	}

	.ctx-label {
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}

	.ctx-item:hover:not(:disabled) {
		background: var(--surface-hover);
	}

	.ctx-item:hover:not(:disabled) .ctx-icon {
		opacity: 1;
	}

	.ctx-item.danger {
		color: var(--color-error, #ef4444);
	}

	.ctx-item.danger:hover:not(:disabled) {
		background: rgba(var(--color-error-rgb), 0.12);
	}

	.ctx-item:disabled,
	.ctx-item.disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.ctx-separator {
		height: 1px;
		margin: 4px 6px;
		background: var(--border);
	}
</style>
