<script lang="ts">
	import type { Snippet } from "svelte";
	import Icon from "$lib/icons/Icon.svelte";

	interface Props {
		label: string;
		ariaLabel?: string;
		filtered?: boolean;
		children: Snippet<[{ close: () => void }]>;
	}

	let { label, ariaLabel, filtered = false, children }: Props = $props();

	let menu: HTMLDetailsElement;
	let trigger: HTMLElement;

	function close() {
		if (!menu?.open) return;
		menu.open = false;
		trigger.focus();
	}
</script>

<svelte:window
	onclick={(event) => {
		if (menu && !menu.contains(event.target as Node)) menu.open = false;
	}}
	onkeydown={(event) => {
		if (event.key === "Escape") close();
	}}
/>

<details class="log-menu" bind:this={menu}>
	<summary bind:this={trigger} class:filtered aria-label={ariaLabel}>
		{label}
		<Icon name="log:chevron-down" size={12} />
	</summary>
	<div class="menu-list">
		{@render children({ close })}
	</div>
</details>

<style>
	.log-menu {
		position: relative;
		flex-shrink: 0;
	}

	summary {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 32px;
		padding: 6px 8px;
		border: none;
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
		list-style: none;
	}

	summary::-webkit-details-marker {
		display: none;
	}

	summary:hover,
	.log-menu[open] summary {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	summary:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	summary.filtered {
		color: var(--accent);
	}

	.menu-list {
		position: absolute;
		top: calc(100% + 4px);
		right: 0;
		z-index: 20;
		width: max-content;
		max-width: calc(100vw - 28px);
		padding: 4px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		box-shadow: var(--shadow-md);
	}
</style>
