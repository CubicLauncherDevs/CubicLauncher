<script lang="ts">
	import { t } from "$lib/i18n";
	import { slide } from "svelte/transition";
	import { launcherStore } from "$lib/state/state.svelte";
	import { animDuration } from "$lib/utils/animations";
	import { saveSettings } from "$lib/api/launcherService";
	import Lupa from "$lib/icons/Lupa.svelte";
	import CloseIcon from "$lib/icons/CloseIcon.svelte";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";
	import type {
		MarketFilters,
		MarketSort,
		LocalSort,
		LocalSourceFilter,
	} from "$lib/state/marketState.svelte";
	import type { MarketSource, ContentType } from "$lib/types/market";

	interface Props {
		filters: MarketFilters;
		contentType: ContentType;
		onSourceChange: (source: MarketSource) => void;
		onQueryChange: (query: string) => void;
		onSortChange: (sort: MarketSort) => void;
		onCategoryChange: (category: string | null) => void;
		onLocalSortChange?: (sort: LocalSort) => void;
		onLocalSourceChange?: (source: LocalSourceFilter) => void;
	}

	let {
		filters,
		contentType = "mods",
		onSourceChange,
		onQueryChange,
		onSortChange,
		onCategoryChange,
		onLocalSortChange,
		onLocalSourceChange,
	}: Props = $props();

	const isModContent = $derived(contentType === "mods");
	const collapsed = $derived(launcherStore.settings.market_filter_collapsed);

	function toggleCollapsed() {
		launcherStore.settings.market_filter_collapsed = !collapsed;
		saveSettings().catch(console.error);
	}

	const slideDuration = $derived(animDuration(180));

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

	const localSources: {
		value: LocalSourceFilter;
		label: string;
	}[] = [
		{ value: "all", label: t("market.filter.sourceAll") },
		{ value: "modrinth", label: t("market.filter.sourceModrinth") },
		{ value: "curseforge", label: t("market.filter.sourceCurseForge") },
		{ value: "local", label: t("market.filter.sourceLocal") },
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
		<button
			type="button"
			class="search-clear"
			aria-label={t("market.filter.clearSearch")}
			title={t("market.filter.clearSearch")}
			disabled={!filters.query}
			onclick={() => onQueryChange("")}
		>
			<CloseIcon size={14} />
		</button>
	</div>

	{#if !collapsed}
		<div
			class="filter-advanced"
			transition:slide={{ duration: slideDuration }}
		>
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
				<div class="filter-section">
					<span class="filter-label">{t("market.filter.source")}</span
					>
					<div class="filter-chips">
						{#each localSources as source (source.value)}
							<button
								type="button"
								class="filter-chip"
								class:active={filters.localSource ===
									source.value}
								onclick={() =>
									onLocalSourceChange?.(source.value)}
							>
								{source.label}
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

	.collapse-btn :global(.icon-svg) {
		transition: transform 0.2s ease;
	}

	.collapse-btn.collapsed :global(.icon-svg) {
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
		gap: 0;
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 var(--surface-selected);
		overflow: hidden;
	}

	.search-row:focus-within {
		border-color: var(--accent);
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
		min-width: 0;
		padding: 7px 10px 7px 34px;
		background: transparent;
		border: none;
		border-radius: 0;
		color: var(--text-primary);
		font-size: 0.85rem;
		outline: none;
		font-family: inherit;
	}

	.search-input::placeholder {
		color: var(--text-secondary);
		opacity: 0.55;
	}

	.search-clear {
		width: 36px;
		flex-shrink: 0;
		align-self: stretch;
		padding: 0;
		background: transparent;
		border: none;
		border-left: 1px solid var(--border-color);
		color: var(--text-secondary);
		border-radius: 0;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			color 0.15s ease,
			background 0.15s ease;
	}

	.search-clear:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}

	.search-clear:hover:not(:disabled) {
		color: var(--text-primary);
		background: var(--surface-selected);
	}

	.search-clear:disabled {
		color: var(--text-muted);
		cursor: not-allowed;
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
