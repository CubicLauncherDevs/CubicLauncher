<script lang="ts">
	import { slide } from "svelte/transition";
	import type { Snippet } from "svelte";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";
	import { derived } from "svelte/store";

	let {
		title,
		iconSrc,
		storageKey,
		defaultOpen = true,
		children,
	}: {
		title: string;
		iconSrc?: string;
		storageKey?: string;
		defaultOpen?: boolean;
		children: Snippet;
	} = $props();

	function loadSaved(key: string | undefined, fallback: boolean): boolean {
		if (!key) return fallback;
		try {
			const saved = localStorage.getItem(key);
			if (saved !== null) return saved === "true";
		} catch {
			// localStorage not available
		}
		return fallback;
	}

	let open = $derived(loadSaved(storageKey, defaultOpen));

	$effect(() => {
		if (storageKey) {
			try {
				localStorage.setItem(storageKey, String(open));
			} catch {
				// localStorage not available
			}
		}
	});
</script>

<div class="cs-root">
	<button
		type="button"
		class="cs-header"
		class:expanded={open}
		onclick={() => (open = !open)}
		aria-expanded={open}
	>
		<span class="cs-header-left">
			{#if iconSrc}
				<span
					class={"cs-icon" + (open ? " open" : "")}
					style="mask-image: url({iconSrc}); -webkit-mask-image: url({iconSrc});"
				></span>
			{/if}
			<span class="cs-title">{title}</span>
		</span>
		<ChevronDownIcon
			size={16}
			class={"cs-chevron" + (open ? " open" : "")}
		/>
	</button>
	{#if open}
		<div class="cs-content" transition:slide={{ duration: 150 }}>
			{@render children()}
		</div>
	{/if}
</div>

<style>
	.cs-root {
		background: var(--bg-card-gradient), var(--bg-card);
	}

	.cs-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		background: none;
		border: none;
		color: inherit;
		padding: 8px 10px;
		cursor: pointer;
		border-radius: var(--border-radius-sm);
		transition: background 0.15s ease;
		user-select: none;
	}

	.cs-header:hover {
		background: rgba(255, 255, 255, 0.03);
	}

	.cs-header.expanded {
		border-bottom: 1px solid var(--border);
		border-radius: var(--border-radius-sm) var(--border-radius-sm) 0 0;
	}

	.cs-header-left {
		display: flex;
		align-items: center;
		gap: 7px;
		min-width: 0;
		flex: 1;
	}

	.cs-icon {
		display: block;
		width: 18px;
		height: 18px;
		background: var(--text-primary);
		mask-size: contain;
		mask-repeat: no-repeat;
		mask-position: center;
		-webkit-mask-size: contain;
		-webkit-mask-repeat: no-repeat;
		-webkit-mask-position: center;
		flex-shrink: 0;
		transition: transform 0.5s;
	}

	.cs-icon.open {
		transform: rotate(360deg);
	}

	.cs-title {
		font-size: 0.75rem;
		font-weight: 700;
		text-transform: uppercase;
		color: var(--text-primary);
		letter-spacing: 0.05em;
		text-align: left;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	:global(.cs-chevron) {
		color: var(--accent);
		transition: transform 0.2s;
		flex-shrink: 0;
	}

	:global(.cs-chevron.open) {
		transform: rotate(180deg);
	}

	.cs-content {
		padding: 4px 10px 10px 10px;
		overflow: hidden;
	}
</style>
