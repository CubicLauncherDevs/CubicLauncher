<script lang="ts">
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import Select from "$lib/components/layout/Select.svelte";
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
		onSearch: () => void;
		onSortChange: (sort: MarketSort) => void;
		onCategoryChange: (category: string | null) => void;
		onLocalSortChange: (sort: LocalSort) => void;
		onLocalSourceChange: (source: LocalSourceFilter) => void;
		onClearFilters: () => void;
		active?: boolean;
	}

	let {
		filters,
		contentType,
		onSourceChange,
		onQueryChange,
		onSearch,
		onSortChange,
		onCategoryChange,
		onLocalSortChange,
		onLocalSourceChange,
		onClearFilters,
		active = true,
	}: Props = $props();

	let searchInput: HTMLInputElement;
	const isModContent = $derived(contentType === "mods");
	const isLocal = $derived(filters.source === "local");
	const placeholder = $derived(
		isLocal
			? t("market.filter.searchLocal")
			: filters.source === "curseforge"
				? t("market.filter.searchCurseForge")
				: t("market.filter.searchModrinth"),
	);
	const sources = $derived<{ value: MarketSource; label: string }[]>([
		{ value: "modrinth", label: "Modrinth" },
		...(isModContent
			? [{ value: "curseforge" as const, label: "CurseForge" }]
			: []),
		{ value: "local", label: t("market.filter.tabLocal") },
	]);
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
	] as const;
	const sortOptions = $derived([
		{ value: "auto", label: t("market.filter.sortAuto") },
		{ value: "relevance", label: t("market.filter.sortRelevance") },
		{ value: "downloads", label: t("market.filter.sortDownloads") },
		{ value: "newest", label: t("market.filter.sortNewest") },
	]);
	const localSortOptions = $derived([
		{ value: "name-asc", label: t("market.filter.sortNameAsc") },
		{ value: "name-desc", label: t("market.filter.sortNameDesc") },
	]);
	const categoryOptions = $derived([
		{ value: "", label: t("market.filter.allCategories") },
		...categories.map((category) => ({
			value: category,
			label: t(`market.categories.${category}`),
		})),
	]);
	const localSourceOptions = $derived([
		{ value: "all", label: t("market.filter.sourceAll") },
		{ value: "modrinth", label: "Modrinth" },
		{ value: "curseforge", label: "CurseForge" },
		{ value: "local", label: t("market.filter.sourceLocal") },
	]);
	const hasFilters = $derived(
		isLocal
			? filters.localSource !== "all" ||
					filters.localSort !== "name-asc" ||
					filters.localStatus !== "all"
			: filters.category !== null || filters.sort !== "auto",
	);

	function clearSearch() {
		onQueryChange("");
		searchInput.focus();
	}

	function handleShortcut(event: KeyboardEvent) {
		if (!active || event.defaultPrevented || event.altKey) return;
		// Let nested dialogs keep their own keyboard interactions.
		if (
			event.target instanceof Element &&
			event.target.closest('[role="dialog"], dialog')
		)
			return;
		if (
			(event.ctrlKey || event.metaKey) &&
			event.key.toLowerCase() === "f"
		) {
			event.preventDefault();
			searchInput.focus();
			searchInput.select();
		}
	}
</script>

<svelte:window onkeydown={handleShortcut} />

<div class="market-filter-panel">
	<div class="search-heading">
		<h2>
			<Icon name="instance:grid" size={18} />{t(
				isLocal ? "market.manage.title" : "market.browse.title",
			)}
		</h2>
		<div
			class="compatibility"
			aria-label={t("market.browse.compatibility")}
		>
			<span>Minecraft {filters.gameVersion}</span>
			{#if isModContent}<span>{filters.loader}</span>{/if}
		</div>
	</div>
	<form
		class="search-row"
		role="search"
		onsubmit={(event) => {
			event.preventDefault();
			onSearch();
		}}
	>
		<span class="search-icon"><Icon name="ui:search" size={18} /></span>
		<input
			bind:this={searchInput}
			type="search"
			class="search-input"
			{placeholder}
			aria-label={placeholder}
			autocomplete="off"
			spellcheck="false"
			value={filters.query}
			oninput={(event) => {
				if (!(event instanceof InputEvent && event.isComposing))
					onQueryChange(event.currentTarget.value);
			}}
			oncompositionend={(event) =>
				onQueryChange(event.currentTarget.value)}
			onkeydown={(event) => {
				if (event.key === "Escape" && !event.isComposing) {
					event.preventDefault();
					clearSearch();
				}
			}}
		/>
		{#if filters.query}
			<button
				type="button"
				class="clear-search"
				onclick={clearSearch}
				aria-label={t("market.filter.clearSearch")}
				title={t("market.filter.clearSearch")}
			>
				<Icon name="ui:close" size={16} />
			</button>
		{/if}
		<button type="submit" class="search-submit"
			>{t("market.browse.search")}</button
		>
	</form>
	<div class="browse-controls">
		<div
			class="source-tabs"
			role="group"
			aria-label={t("market.filter.source")}
		>
			{#each sources as source (source.value)}
				<button
					type="button"
					class:active={filters.source === source.value}
					aria-pressed={filters.source === source.value}
					onclick={() => onSourceChange(source.value)}
				>
					<Icon
						name={source.value === "local"
							? "instance:folder"
							: `brand:${source.value}`}
						size={15}
					/>
					{source.label}
				</button>
			{/each}
		</div>
		<div class="filters">
			<div class="filter-select">
				{#if isLocal}
					<Select
						compact
						disabled={!active}
						label={t("market.filter.sortBy")}
						value={filters.localSort}
						options={localSortOptions}
						onchange={(value) =>
							onLocalSortChange(value as LocalSort)}
					/>
				{:else}
					<Select
						compact
						disabled={!active}
						label={t("market.filter.sortBy")}
						value={filters.sort}
						options={sortOptions}
						onchange={(value) => onSortChange(value as MarketSort)}
					/>
				{/if}
			</div>
			{#if isLocal}
				<div class="filter-select">
					<Select
						compact
						disabled={!active}
						label={t("market.filter.source")}
						value={filters.localSource}
						options={localSourceOptions}
						onchange={(value) =>
							onLocalSourceChange(value as LocalSourceFilter)}
					/>
				</div>
			{:else if isModContent}
				<div class="filter-select">
					<Select
						compact
						disabled={!active}
						label={t("market.filter.category")}
						value={filters.category ?? ""}
						options={categoryOptions}
						onchange={(value) => onCategoryChange(value || null)}
					/>
				</div>
			{/if}
			{#if hasFilters}
				<button
					type="button"
					class="reset-filters"
					onclick={onClearFilters}
					>{t("market.browse.clearFilters")}</button
				>
			{/if}
		</div>
	</div>
</div>

<style>
	.market-filter-panel {
		padding: 20px 24px 14px;
		background: transparent;
		border-bottom: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 12px;
		flex-shrink: 0;
	}
	.search-heading,
	.compatibility,
	.browse-controls,
	.filters,
	.source-tabs,
	.search-row {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.search-heading,
	.browse-controls {
		justify-content: space-between;
		flex-wrap: wrap;
	}
	.search-heading h2 {
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 0;
		font-size: 1.15rem;
		font-weight: 700;
		color: var(--text-primary);
	}
	.compatibility {
		gap: 6px;
		flex-wrap: wrap;
		font-size: 0.75rem;
		color: var(--text-tertiary, var(--text-secondary));
	}
	.compatibility span {
		padding: 3px 8px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-selected);
	}
	.search-row {
		gap: 0;
		padding: 4px;
		background: var(--surface-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
	}
	.search-row:focus-within {
		border-color: var(--accent);
	}
	.search-icon {
		display: flex;
		margin: 0 12px;
		color: var(--text-secondary);
	}
	.search-input {
		flex: 1;
		min-width: 0;
		padding: 8px 0;
		border: none;
		background: transparent;
		color: var(--text-primary);
		font: inherit;
		font-size: 0.85rem;
		outline: none;
	}
	.search-input::-webkit-search-cancel-button {
		display: none;
	}
	.search-input::placeholder {
		color: var(--text-tertiary, var(--text-secondary));
	}
	button {
		font: inherit;
		font-size: 0.8rem;
		cursor: pointer;
		border-radius: var(--border-radius-sm);
	}
	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.clear-search {
		display: flex;
		padding: 8px;
		margin: 0 6px;
		border: none;
		color: var(--text-secondary);
		background: transparent;
	}
	.search-submit {
		padding: 8px 18px;
		border: none;
		font-weight: 700;
		background: var(--accent);
		color: var(--accent-text);
	}
	.source-tabs {
		gap: 4px;
		flex-wrap: wrap;
		align-self: flex-end;
	}
	.source-tabs button {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		padding: 8px 12px;
		border: 1px solid transparent;
		background: transparent;
		color: var(--text-secondary);
		font-weight: 600;
	}
	.source-tabs button.active {
		background: var(--surface-selected);
		border-color: var(--border);
		color: var(--text-primary);
		box-shadow: inset 0 -2px var(--accent);
	}
	.source-tabs button:hover:not(.active),
	.clear-search:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.filters {
		flex: 1 1 320px;
		justify-content: flex-end;
		flex-wrap: wrap;
		gap: 10px;
		align-items: flex-end;
	}
	.filter-select {
		min-width: 0;
		flex: 1 1 140px;
		max-width: 180px;
	}
	.reset-filters {
		padding: 8px;
		background: transparent;
		color: var(--text-secondary);
		border: 1px solid var(--border);
	}
	.reset-filters:hover {
		color: var(--text-primary);
		border-color: var(--accent);
	}
	.search-submit:hover {
		opacity: 0.9;
	}
	@container market (max-width: 700px) {
		.market-filter-panel {
			padding: 14px;
			gap: 10px;
		}
		.search-submit {
			padding: 10px 12px;
		}
		.filters {
			width: 100%;
			justify-content: flex-start;
		}
		.filter-select {
			flex: 1 1 150px;
			max-width: none;
		}
	}
</style>
