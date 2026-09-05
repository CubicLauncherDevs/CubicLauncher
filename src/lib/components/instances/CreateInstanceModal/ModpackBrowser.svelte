<script lang="ts">
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import Loading from "$lib/icons/Loading.svelte";
	import Select from "$lib/components/layout/Select.svelte";
	import { MAX_INSTANCE_NAME_LEN } from "$lib/utils/instanceName";
	import type { Snippet } from "svelte";

	export type ModpackItem = {
		id: string | number;
		title: string;
		description: string;
		iconUrl?: string | null;
		downloads: number;
		author?: string;
	};

	export type ModpackSort = "relevance" | "downloads" | "newest";

	export interface ModpackFilters {
		sort: ModpackSort;
		category: string | null;
		gameVersion: string | null;
	}

	let {
		query = $bindable(""),
		items = [],
		totalHits = 0,
		searching = false,
		loadingMore = false,
		selectedItem = $bindable<ModpackItem | null>(null),
		versionOptions = [],
		selectedVersion = $bindable<string>(""),
		loadingVersions = false,
		installing = false,
		installError = null,
		installStep = "",
		needsCustomName = false,
		customName = $bindable(""),
		customNameError = null,
		searchPlaceholder = "",
		emptySearchingText = "",
		emptyNoResultsText = "",
		filters = $bindable<ModpackFilters>({
			sort: "downloads",
			category: null,
			gameVersion: null,
		}),
		categoryOptions = [] as { value: string; label: string }[],
		gameVersionOptions = [] as { value: string; label: string }[],
		onSearch,
		onLoadMore,
		onFilterChange,
		onSelect,
		onBack,
		onInstall,
		onConfirmCustomName,
		onCancelCustomName,
		detailExtra,
	}: {
		query?: string;
		items?: ModpackItem[];
		totalHits?: number;
		searching?: boolean;
		loadingMore?: boolean;
		selectedItem?: ModpackItem | null;
		versionOptions?: { value: string; label: string }[];
		selectedVersion?: string;
		loadingVersions?: boolean;
		installing?: boolean;
		installError?: string | null;
		installStep?: string;
		needsCustomName?: boolean;
		customName?: string;
		customNameError?: string | null;
		searchPlaceholder?: string;
		emptySearchingText?: string;
		emptyNoResultsText?: string;
		filters?: ModpackFilters;
		categoryOptions?: { value: string; label: string }[];
		gameVersionOptions?: { value: string; label: string }[];
		onSearch?: () => void;
		onLoadMore?: () => void;
		onFilterChange?: () => void;
		onSelect?: (item: ModpackItem) => void;
		onBack?: () => void;
		onInstall?: () => void;
		onConfirmCustomName?: () => void;
		onCancelCustomName?: () => void;
		detailExtra?: Snippet<[ModpackItem]>;
	} = $props();

	let sentinelEl: HTMLDivElement | undefined = $state();

	function handleSearch() {
		onSearch?.();
	}

	function handleFilterChange() {
		onFilterChange?.();
	}

	function setSort(sort: ModpackSort) {
		filters.sort = sort;
		handleFilterChange();
	}

	function setCategory(category: string | null) {
		filters.category = category;
		handleFilterChange();
	}

	function setGameVersion(version: string | null) {
		filters.gameVersion = version;
		handleFilterChange();
	}

	function formatDownloads(n: number): string {
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
		return String(n);
	}

	$effect(() => {
		const el = sentinelEl;
		if (!el) return;

		const observer = new IntersectionObserver(
			(entries) => {
				if (entries[0].isIntersecting) {
					onLoadMore?.();
				}
			},
			{ rootMargin: "300px" },
		);

		observer.observe(el);
		return () => observer.disconnect();
	});
</script>

<div class="modpack-browser">
	<div class="search-bar">
		<input
			type="text"
			class="search-input"
			bind:value={query}
			placeholder={searchPlaceholder}
			onkeydown={(e) => e.key === "Enter" && handleSearch()}
		/>
		<button
			type="button"
			class="btn-primary search-btn"
			onclick={handleSearch}
			disabled={searching || !query.trim()}
		>
			{#if searching}
				<Loading />
			{/if}
			{t("createInstance.searchBtn")}
		</button>
	</div>

	<div class="filter-bar">
		<div class="filter-group">
			<span class="filter-label"
				>{t("createInstance.filterSortLabel")}</span
			>
			<div class="filter-chips">
				<button
					type="button"
					class="filter-chip"
					class:active={filters.sort === "relevance"}
					onclick={() => setSort("relevance")}
				>
					{t("createInstance.sortRelevance")}
				</button>
				<button
					type="button"
					class="filter-chip"
					class:active={filters.sort === "downloads"}
					onclick={() => setSort("downloads")}
				>
					{t("createInstance.sortDownloads")}
				</button>
				<button
					type="button"
					class="filter-chip"
					class:active={filters.sort === "newest"}
					onclick={() => setSort("newest")}
				>
					{t("createInstance.sortNewest")}
				</button>
			</div>
		</div>

		<div class="filter-selects">
			<Select
				value={filters.category ?? ""}
				options={[
					{
						value: "",
						label: t("createInstance.filterAllCategories"),
					},
					...categoryOptions,
				]}
				placeholder={t("createInstance.filterCategoryLabel")}
				label={t("createInstance.filterCategoryLabel")}
				onchange={(value) => setCategory(value || null)}
			/>
			<Select
				value={filters.gameVersion ?? ""}
				options={[
					{ value: "", label: t("createInstance.filterAllVersions") },
					...gameVersionOptions,
				]}
				placeholder={t("createInstance.filterVersionLabel")}
				label={t("createInstance.filterVersionLabel")}
				onchange={(value) => setGameVersion(value || null)}
			/>
		</div>
	</div>

	{#if installError}
		<div class="error-msg">{installError}</div>
	{/if}

	{#if installing}
		<div class="installing-overlay">
			<Loading />
			<span>{installStep}</span>
		</div>
	{/if}

	{#if selectedItem}
		<div class="detail-view">
			<button type="button" class="back-btn" onclick={onBack}>
				<Icon name="ui:chevron-left" size={16} />
				{t("createInstance.backBtn")}
			</button>

			<div class="detail-header">
				{#if selectedItem.iconUrl}
					<img
						src={selectedItem.iconUrl}
						alt=""
						class="detail-icon"
						loading="lazy"
						decoding="async"
						onerror={(e) => {
							(
								e.currentTarget as HTMLImageElement
							).style.display = "none";
						}}
					/>
				{/if}
				<div class="detail-title-group">
					<h3>{selectedItem.title}</h3>
					<span class="detail-author"
						>{selectedItem.author ?? ""}</span
					>
				</div>
			</div>

			<p class="detail-desc">{selectedItem.description}</p>

			<div class="detail-actions">
				<div class="version-select">
					<span class="version-label">
						{t("createInstance.versionLabel")}
					</span>
					<Select
						bind:value={selectedVersion}
						options={versionOptions}
						placeholder={t("createInstance.selectLoaderVersion")}
						loading={loadingVersions}
						disabled={versionOptions.length === 0 || installing}
					/>
				</div>

				{#if needsCustomName}
					<button
						type="button"
						class="btn-secondary install-btn"
						onclick={onCancelCustomName}
						disabled={installing}
					>
						{t("createInstance.cancel")}
					</button>
				{:else}
					<button
						type="button"
						class="btn-primary install-btn"
						onclick={onInstall}
						disabled={installing || !selectedVersion}
					>
						{installing
							? t("createInstance.installingModpack")
							: t("createInstance.installBtn")}
					</button>
				{/if}
			</div>

			{#if needsCustomName}
				<div class="custom-name-section">
					<p class="custom-name-hint">
						{t("createInstance.customNameNeeded")}
					</p>
					<div class="custom-name-input-row">
						<input
							type="text"
							class="text-input"
							class:error={customNameError}
							bind:value={customName}
							maxlength={MAX_INSTANCE_NAME_LEN}
							disabled={installing}
							oninput={() => (customNameError = null)}
							onkeydown={(e) =>
								e.key === "Enter" && onConfirmCustomName?.()}
							placeholder={t(
								"createInstance.customNamePlaceholder",
							)}
						/>
						<button
							type="button"
							class="btn-primary"
							onclick={onConfirmCustomName}
							disabled={installing || !customName.trim()}
						>
							{installing
								? t("createInstance.installingModpack")
								: t("createInstance.installBtn")}
						</button>
					</div>
					{#if customNameError}
						<span class="input-error">{customNameError}</span>
					{/if}
				</div>
			{/if}

			{#if detailExtra}
				{@render detailExtra(selectedItem)}
			{/if}
		</div>
	{:else}
		<div class="results-panel">
			{#if searching && items.length === 0}
				<div class="empty-state">
					<Loading />
					<span>{emptySearchingText}</span>
				</div>
			{:else if items.length === 0}
				<div class="empty-state">{emptyNoResultsText}</div>
			{:else}
				<div class="results-grid">
					{#each items as item (item.id)}
						<button
							type="button"
							class="pack-card"
							onclick={() => onSelect?.(item)}
						>
							<div class="pack-icon-wrap">
								{#if item.iconUrl}
									<img
										src={item.iconUrl}
										alt=""
										class="pack-icon"
										loading="lazy"
										decoding="async"
										onerror={(e) => {
											(
												e.currentTarget as HTMLImageElement
											).style.display = "none";
										}}
									/>
								{/if}
							</div>
							<div class="pack-info">
								<span class="pack-title">{item.title}</span>
								<span class="pack-desc">{item.description}</span
								>
								<span class="pack-meta">
									{formatDownloads(item.downloads)}
									{t("createInstance.downloads")}
								</span>
							</div>
						</button>
					{/each}
				</div>
				{#if items.length < totalHits}
					<div bind:this={sentinelEl} class="load-sentinel">
						{#if loadingMore}
							<Loading />
						{:else}
							<span class="sentinel-hint">Scroll for more</span>
						{/if}
					</div>
				{/if}
			{/if}
		</div>
	{/if}
</div>

<style>
	.modpack-browser {
		display: flex;
		flex-direction: column;
		gap: 12px;
		height: 100%;
		min-height: 300px;
	}

	.search-bar {
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.search-input {
		flex: 1;
		padding: 8px 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		color: var(--text-primary);
		font-size: 0.82rem;
		outline: none;
	}

	.search-input:focus {
		border-color: var(--accent);
	}

	.search-btn {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.filter-bar {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 8px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
	}

	.filter-group {
		display: flex;
		flex-direction: column;
		gap: 4px;
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

	.filter-selects {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}

	.error-msg {
		color: var(--color-error);
		font-size: 0.8rem;
		padding: 8px 12px;
		background: rgba(var(--color-error-rgb), 0.1);
		border-radius: var(--border-radius-sm);
	}

	.installing-overlay {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 16px;
		color: var(--text-secondary);
		font-size: 0.85rem;
	}

	.results-panel {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
		max-height: 440px;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 40px 16px;
		color: var(--text-muted);
		font-size: 0.82rem;
	}

	.results-grid {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.pack-card {
		display: flex;
		gap: 10px;
		padding: 10px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: inherit;
		cursor: pointer;
		text-align: left;
		width: 100%;
		transition:
			background 0.15s ease,
			border-color 0.15s ease;
	}

	.pack-card:hover {
		background: var(--bg-item-active);
	}

	.pack-icon-wrap {
		width: 48px;
		height: 48px;
		border-radius: var(--border-radius-sm);
		overflow: hidden;
		flex-shrink: 0;
		background: var(--bg-card);
	}

	.pack-icon {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.pack-info {
		display: flex;
		flex-direction: column;
		gap: 3px;
		min-width: 0;
		flex: 1;
	}

	.pack-title {
		font-size: 0.82rem;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.pack-desc {
		font-size: 0.72rem;
		color: var(--text-secondary);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		line-clamp: 2;
		overflow: hidden;
	}

	.pack-meta {
		font-size: 0.65rem;
		color: var(--text-tertiary);
	}

	.load-sentinel {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 12px;
	}

	.sentinel-hint {
		font-size: 0.7rem;
		color: var(--text-tertiary);
	}

	.detail-view {
		flex: 1;
		overflow-y: auto;
		max-height: 440px;
		display: flex;
		flex-direction: column;
		gap: 14px;
		animation: slideIn 0.2s ease-out;
	}

	@keyframes slideIn {
		from {
			opacity: 0.5;
			transform: translateX(24px);
		}
		to {
			opacity: 1;
			transform: translateX(0);
		}
	}

	.back-btn {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 10px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: var(--text-secondary);
		font-size: 0.75rem;
		font-family: inherit;
		cursor: pointer;
		align-self: flex-start;
		transition:
			color 0.15s,
			border-color 0.15s;
	}

	.back-btn:hover {
		color: var(--text-primary);
		border-color: var(--text-secondary);
	}

	.detail-header {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.detail-icon {
		width: 48px;
		height: 48px;
		border-radius: var(--border-radius-sm);
		object-fit: cover;
	}

	.detail-title-group h3 {
		margin: 0;
		font-size: 1rem;
		font-weight: 700;
	}

	.detail-author {
		font-size: 0.75rem;
		color: var(--text-secondary);
	}

	.detail-desc {
		font-size: 0.78rem;
		color: var(--text-secondary);
		line-height: 1.4;
		margin: 0;
	}

	.detail-actions {
		display: flex;
		gap: 12px;
		align-items: flex-end;
	}

	.version-select {
		display: flex;
		flex-direction: column;
		gap: 4px;
		flex: 1;
	}

	.version-label {
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.install-btn {
		flex-shrink: 0;
		height: fit-content;
		padding: 8px 20px;
		justify-content: center;
	}

	.custom-name-section {
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 12px;
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.03);
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.custom-name-hint {
		font-size: 0.78rem;
		color: var(--text-secondary);
		margin: 0;
		line-height: 1.4;
	}

	.custom-name-input-row {
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.custom-name-input-row :global(.text-input) {
		flex: 1;
		padding: 8px 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		color: var(--text-primary);
		font-size: 0.82rem;
		font-family: inherit;
		outline: none;
	}

	.custom-name-input-row :global(.text-input:focus) {
		border-color: var(--accent);
	}

	.custom-name-input-row :global(.text-input.error) {
		border-color: var(--color-error) !important;
		box-shadow: 0 0 0 1px var(--color-error) !important;
	}

	.input-error {
		font-size: 0.7rem;
		color: var(--color-error);
		display: block;
	}
</style>
