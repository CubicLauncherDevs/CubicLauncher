<script lang="ts">
	import { t } from "$lib/i18n";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";
	import DownloadIcon from "$lib/icons/DownloadIcon.svelte";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";

	let {
		open = $bindable(false),
		activeCount = 0,
		doneCount = 0,
	}: {
		open?: boolean;
		activeCount?: number;
		doneCount?: number;
	} = $props();
</script>

<button
	type="button"
	class="sd-header"
	class:expanded={open}
	onclick={() => (open = !open)}
	aria-expanded={open}
>
	<span class="sd-header-left">
		{#if activeCount > 0}
			<span class="sd-spinner"></span>
			<span class="sd-label"
				>{activeCount} {t("sidebar.downloading")}</span
			>
		{:else if doneCount > 0}
			<CheckIcon size={12} color="var(--color-success)" />
			<span class="sd-label"
				>{doneCount} {t("sidebar.completed")}</span
			>
		{:else}
			<DownloadIcon size={18} />
			<span class="sd-label">{t("sidebar.noDownloads")}</span>
		{/if}
	</span>
	<ChevronDownIcon
		size={16}
		class={"sd-chevron" + (open ? " open" : "")}
	/>
</button>

<style>
	.sd-header {
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

	.sd-header:hover {
		background: rgba(255, 255, 255, 0.03);
	}

	.sd-header.expanded {
		border-bottom: 1px solid var(--border);
		border-radius: var(--border-radius-sm) var(--border-radius-sm) 0 0;
	}

	.sd-header-left {
		display: flex;
		align-items: center;
		gap: 7px;
		min-width: 0;
		flex: 1;
	}

	.sd-label {
		font-size: 0.75rem;
		font-weight: 700;
		color: var(--text-primary);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
	}

	.sd-spinner {
		width: 10px;
		height: 10px;
		border: 1.5px solid var(--border);
		border-top-color: var(--text-muted);
		border-radius: 50%;
		animation: sd-spin 0.7s linear infinite;
		will-change: transform;
		flex-shrink: 0;
	}

	@keyframes sd-spin {
		to {
			transform: rotate(360deg);
		}
	}

	:global(.sd-chevron) {
		color: var(--accent);
		flex-shrink: 0;
		transition: transform 0.2s;
	}

	:global(.sd-chevron.open) {
		transform: rotate(180deg);
	}

	@media (max-width: 650px) {
		.sd-header {
			justify-content: center;
			padding: 8px 4px;
		}

		:global(.sd-header .sd-chevron) {
			display: none;
		}
	}
</style>