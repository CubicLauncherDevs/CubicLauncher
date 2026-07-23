<script lang="ts">
	import { t } from "$lib/i18n";
	import { slide } from "svelte/transition";
	import { launcherStore } from "$lib/state/state.svelte";
	import { saveSettings } from "$lib/api/launcherService";
	import Loading from "$lib/icons/Loading.svelte";
	import Lupa from "$lib/icons/Lupa.svelte";
	import CloseIcon from "$lib/icons/CloseIcon.svelte";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";
	import type {
		MarketFilters,
		MarketSort,
		LocalSort,
	} from "$lib/state/marketState.svelte";
	import type { MarketSource, ContentType } from "$lib/types/market";

	interface Props {
		filters: MarketFilters;
		contentType: ContentType;
		loading: boolean;
		onSourceChange: (source: MarketSource) => void;
		onQueryChange: (query: string) => void;
		onSortChange: (sort: MarketSort) => void;
		onCategoryChange: (category: string | null) => void;
		onLocalSortChange?: (sort: LocalSort) => void;
		onRefresh: () => void;
	}

	let {
		filters,
		contentType = "mods",
		loading,
		onSourceChange,
		onQueryChange,
		onSortChange,
		onCategoryChange,
		onLocalSortChange,
		onRefresh,
	}: Props = $props();

	const isModContent = $derived(contentType === "mods");
	const collapsed = $derived(launcherStore.settings.market_filter_collapsed);

	function toggleCollapsed() {
		launcherStore.settings.market_filter_collapsed = !collapsed;
		saveSettings().catch(console.error);
	}

	const sources = $derived<{ value: MarketSource; label: string }[]>([
		{ value: "modrinth", label: t("market.filter.tabModrinth") },
		...(isModContent
			? [
					{
						value: "curseforge" as MarketSource,
						label: t("market.filter.tabCurseForge"),
					},
				]
			: []),
		{ value: "local", label: t("market.filter.tabLocal") },
	]);

	const sorts: { value: MarketSort; label: string; icon: string }[] = [
		{
			value: "downloads",
			label: t("market.filter.sortDownloads"),
			icon: "↓",
		},
		{
			value: "relevance",
			label: t("market.filter.sortRelevance"),
			icon: "◎",
		},
		{ value: "newest", label: t("market.filter.sortNewest"), icon: "★" },
	];

	const localSorts: { value: LocalSort; label: string; icon: string }[] = [
		{ value: "name-asc", label: "Name A-Z", icon: "A" },
		{ value: "name-desc", label: "Name Z-A", icon: "Z" },
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
	<div class="filter-header">
		<span class="filter-title">{t("market.filter.title")}</span>
		<button
			type="button"
			class="collapse-btn"
			class:collapsed
			onclick={toggleCollapsed}
			aria-label={collapsed
				? t("market.filter.showFilters")
				: t("market.filter.hideFilters")}
		>
			<ChevronDownIcon size={16} />
		</button>
	</div>

	<div class="filter-row filter-tabs">
		{#each sources as source (source.value)}
			<button
				type="button"
				class="filter-tab"
				class:active={filters.source === source.value}
				onclick={() => onSourceChange(source.value)}
			>
				{source.label}
			</button>
		{/each}
	</div>

	<div class="filter-row search-row">
		<span class="search-icon">
			<Lupa width="15" height="15" />
		</span>
		<input
			type="text"
			class="search-input"
			placeholder={filters.source === "local"
				? t("market.filter.searchLocal")
				: filters.source === "curseforge"
					? t("market.filter.searchCurseForge")
					: t("market.filter.searchModrinth")}
			value={filters.query}
			oninput={(e) => onQueryChange(e.currentTarget.value)}
		/>
		{#if filters.query}
			<button
				type="button"
				class="search-clear"
				onclick={() => onQueryChange("")}
			>
				<CloseIcon size={14} />
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
				<svg
					width="14"
					height="14"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<path
						d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"
					/>
					<path d="M3 3v5h5" />
					<path
						d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"
					/>
					<path d="M16 16h5v5" />
				</svg>
			{/if}
		</button>
	</div>

	{#if !collapsed}
		<div class="filter-advanced" transition:slide={{ duration: 180 }}>
			{#if filters.source === "local"}
				<div class="filter-section">
					<span class="filter-label">{t("market.filter.sortBy")}</span
					>
					<div class="filter-chips">
						{#each localSorts as sort (sort.value)}
							<button
								type="button"
								class="filter-chip"
								class:active={filters.localSort === sort.value}
								onclick={() => onLocalSortChange?.(sort.value)}
							>
								<span class="chip-icon">{sort.icon}</span>
								{sort.label}
							</button>
						{/each}
					</div>
				</div>
			{:else}
				{#if isModContent}
					<div class="filter-section">
						<span class="filter-label"
							>{t("market.filter.sortBy")}</span
						>
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
						<span class="filter-label"
							>{t("market.filter.category")}</span
						>
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
				{/if}

				<div class="filter-section filter-info">
					<span class="filter-pill"
						>Minecraft {filters.gameVersion}</span
					>
					{#if isModContent}
						<span class="filter-pill">{filters.loader}</span>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.market-filter-panel {
		padding: 12px 14px;
		background: var(--bg-card-gradient), var(--bg-card);
		border-bottom: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 10px;
		flex-shrink: 0;
	}

	.filter-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}

	.filter-title {
		font-size: 0.8rem;
		font-weight: 800;
		color: var(--text-primary);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.collapse-btn {
		width: 26px;
		height: 26px;
		padding: 0;
		background: var(--surface-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.2s ease;
	}

	.collapse-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
		border-color: var(--accent);
	}

	.collapse-btn :global(svg) {
		transition: transform 0.2s ease;
	}

	.collapse-btn.collapsed :global(svg) {
		transform: rotate(-90deg);
	}

	.filter-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.filter-tabs {
		background: var(--surface-selected);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		padding: 3px;
		gap: 3px;
	}

	.filter-tab {
		flex: 1;
		padding: 5px 8px;
		background: transparent;
		border: none;
		color: var(--text-secondary);
		font-size: 0.7rem;
		font-weight: 700;
		cursor: pointer;
		border-radius: calc(var(--border-radius) - 2px);
		transition: all 0.15s ease;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		white-space: nowrap;
	}

	.filter-tab:hover {
		color: var(--text-primary);
		background: var(--surface-hover);
	}

	.filter-tab.active {
		background: var(--accent);
		color: var(--accent-text);
		box-shadow: var(--shadow-sm);
	}

	.search-row {
		position: relative;
		gap: 6px;
	}

	.search-icon {
		position: absolute;
		left: 10px;
		color: var(--text-secondary);
		opacity: 0.6;
		pointer-events: none;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.search-input {
		flex: 1;
		padding: 7px 66px 7px 34px;
		background: var(--surface-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
		font-size: 0.85rem;
		outline: none;
		transition: all 0.2s ease;
		font-family: inherit;
	}

	.search-input:focus {
		border-color: var(--accent);
		background: rgba(var(--surface-rgb), 0.06);
	}

	.search-input::placeholder {
		color: var(--text-secondary);
		opacity: 0.55;
	}

	.search-clear,
	.refresh-btn {
		width: 28px;
		height: 28px;
		background: var(--surface-card);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s ease;
	}

	.search-clear {
		position: absolute;
		right: 36px;
	}

	.search-clear:hover,
	.refresh-btn:hover:not(:disabled) {
		color: var(--text-primary);
		background: var(--surface-hover);
		border-color: var(--accent);
	}

	.refresh-btn:disabled {
		opacity: 0.5;
		cursor: wait;
	}

	:global(.filter-refresh-spinner) {
		width: 14px;
		height: 14px;
	}

	.filter-advanced {
		display: flex;
		flex-direction: column;
		gap: 12px;
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
		padding: 4px 11px;
		background: var(--surface-card);
		border: 1px solid var(--border);
		border-radius: 20px;
		color: var(--text-secondary);
		font-size: 0.7rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s ease;
		font-family: inherit;
		text-transform: capitalize;
	}

	.filter-chip:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
		border-color: var(--text-tertiary);
	}

	.filter-chip.active {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--accent-text);
		box-shadow: var(--shadow-sm);
	}

	.chip-icon {
		opacity: 0.85;
		margin-right: 2px;
	}

	.filter-info {
		flex-direction: row;
		gap: 8px;
		margin-top: 2px;
	}

	.filter-pill {
		padding: 3px 8px;
		background: var(--surface-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font-size: 0.68rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}
</style>
