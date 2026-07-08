<script lang="ts">
	import { t } from "$lib/i18n";
	import { slide } from "svelte/transition";
	import Loading from "$lib/icons/Loading.svelte";
	import type {
		MarketFilters,
		MarketSort,
		MarketSource,
	} from "$lib/state/marketState.svelte";

	interface Props {
		filters: MarketFilters;
		loading: boolean;
		onSourceChange: (source: MarketSource) => void;
		onQueryChange: (query: string) => void;
		onSortChange: (sort: MarketSort) => void;
		onCategoryChange: (category: string | null) => void;
		onRefresh: () => void;
	}

	let {
		filters,
		loading,
		onSourceChange,
		onQueryChange,
		onSortChange,
		onCategoryChange,
		onRefresh,
	}: Props = $props();

	const sorts: { value: MarketSort; label: string; icon: string }[] = [
		{ value: "downloads", label: t("market.filter.sortDownloads"), icon: "↓" },
		{ value: "relevance", label: t("market.filter.sortRelevance"), icon: "◎" },
		{ value: "newest", label: t("market.filter.sortNewest"), icon: "★" },
		{ value: "updated", label: t("market.filter.sortUpdated"), icon: "↻" },
	];

	const categories = [
		"adventure",
		"magic",
		"utility",
		"optimization",
		"equipment",
		"worldgen",
		"food",
		"library",
		"decoration",
		"storage",
	];
</script>

<div class="market-filter-panel">
	<div class="filter-row filter-tabs">
		<button
			type="button"
			class="filter-tab"
			class:active={filters.source === "remote"}
			onclick={() => onSourceChange("remote")}
		>
			{t("market.filter.tabMarket")}
		</button>
		<button
			type="button"
			class="filter-tab"
			class:active={filters.source === "local"}
			onclick={() => onSourceChange("local")}
		>
			{t("market.filter.tabLocal")}
		</button>
	</div>

	<div class="filter-row search-row">
		<span class="search-icon">
			<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<circle cx="11" cy="11" r="8" />
				<path d="m21 21-4.35-4.35" />
			</svg>
		</span>
		<input
			type="text"
			class="search-input"
			placeholder={filters.source === "local"
				? t("market.filter.searchLocal")
				: t("market.filter.searchMarket")}
			value={filters.query}
			oninput={(e) => onQueryChange(e.currentTarget.value)}
		/>
		{#if filters.query}
			<button
				type="button"
				class="search-clear"
				onclick={() => onQueryChange("")}
			>
				×
			</button>
		{/if}
		<button
			type="button"
			class="refresh-btn"
			disabled={loading}
			onclick={onRefresh}
			aria-label={t("market.filter.refresh")}
		>
			{#if loading}
				<Loading class="filter-refresh-spinner" />
			{:else}
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
					<path d="M3 3v5h5" />
					<path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
					<path d="M16 16h5v5" />
				</svg>
			{/if}
		</button>
	</div>

	{#if filters.source === "remote"}
		<div transition:slide={{ duration: 150 }}>
			<div class="filter-section">
				<span class="filter-label">{t("market.filter.sortBy")}</span>
				<div class="filter-chips">
					{#each sorts as sort (sort.value)}
						<button
							type="button"
							class="filter-chip"
							class:active={filters.sort === sort.value}
							onclick={() => onSortChange(sort.value)}
						>
							<span class="chip-icon">{sort.icon}</span>
							{sort.label}
						</button>
					{/each}
				</div>
			</div>

			<div class="filter-section">
				<span class="filter-label">{t("market.filter.category")}</span>
				<div class="filter-chips">
					<button
						type="button"
						class="filter-chip"
						class:active={filters.category === null}
						onclick={() => onCategoryChange(null)}
					>
						{t("market.filter.allCategories")}
					</button>
					{#each categories as category (category)}
						<button
							type="button"
							class="filter-chip"
							class:active={filters.category === category}
							onclick={() => onCategoryChange(category)}
						>
							{category}
						</button>
					{/each}
				</div>
			</div>

			<div class="filter-section filter-info">
				<span class="filter-pill">Minecraft {filters.gameVersion}</span>
				<span class="filter-pill">{filters.loader}</span>
			</div>
		</div>
	{/if}
</div>

<style>
	.market-filter-panel {
		padding: 12px 14px;
		background: var(--bg-sidebar);
		border-bottom: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 10px;
		flex-shrink: 0;
	}

	.filter-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.filter-tabs {
		background: rgba(255, 255, 255, 0.04);
		border-radius: var(--border-radius-sm);
		padding: 2px;
		gap: 2px;
	}

	.filter-tab {
		flex: 1;
		padding: 5px 10px;
		background: transparent;
		border: none;
		color: var(--text-secondary);
		font-size: 0.72rem;
		font-weight: 700;
		cursor: pointer;
		border-radius: calc(var(--border-radius-sm) - 1px);
		transition: all 0.15s;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.filter-tab:hover {
		color: var(--text-primary);
		background: rgba(255, 255, 255, 0.04);
	}

	.filter-tab.active {
		background: var(--accent);
		color: var(--bg-main);
	}

	.search-row {
		position: relative;
		gap: 6px;
	}

	.search-icon {
		position: absolute;
		left: 10px;
		color: var(--text-secondary);
		opacity: 0.5;
		pointer-events: none;
		display: flex;
		align-items: center;
	}

	.search-input {
		flex: 1;
		padding: 7px 66px 7px 34px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
		font-size: 0.85rem;
		outline: none;
		transition: all 0.2s;
		font-family: inherit;
	}

	.search-input:focus {
		border-color: var(--accent);
		background: rgba(255, 255, 255, 0.08);
	}

	.search-input::placeholder {
		color: var(--text-secondary);
		opacity: 0.6;
	}

	.search-clear,
	.refresh-btn {
		width: 28px;
		height: 28px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s;
	}

	.search-clear {
		position: absolute;
		right: 36px;
		font-size: 1.1rem;
		line-height: 1;
	}

	.search-clear:hover,
	.refresh-btn:hover:not(:disabled) {
		color: var(--text-primary);
		background: rgba(255, 255, 255, 0.1);
	}

	.refresh-btn:disabled {
		opacity: 0.5;
		cursor: wait;
	}

	:global(.filter-refresh-spinner) {
		width: 14px;
		height: 14px;
	}

	.filter-section {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.filter-label {
		font-size: 0.65rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.8px;
	}

	.filter-chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.filter-chip {
		padding: 3px 10px;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid var(--border);
		border-radius: 20px;
		color: var(--text-secondary);
		font-size: 0.7rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s;
		font-family: inherit;
		text-transform: capitalize;
	}

	.filter-chip:hover {
		background: rgba(255, 255, 255, 0.08);
		color: var(--text-primary);
	}

	.filter-chip.active {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--bg-main);
	}

	.chip-icon {
		opacity: 0.8;
		margin-right: 2px;
	}

	.filter-info {
		flex-direction: row;
		gap: 8px;
		margin-top: 2px;
	}

	.filter-pill {
		padding: 3px 8px;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font-size: 0.68rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}
</style>
