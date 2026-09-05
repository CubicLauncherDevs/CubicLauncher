<script lang="ts">
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import Loading from "$lib/icons/Loading.svelte";
	import DownloadIcon from "$lib/icons/DownloadIcon.svelte";
	import { animateHeight } from "$lib/utils/animateHeight";
	import { animDuration } from "$lib/utils/animations";
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
	const resizeDuration = $derived(animDuration(220));

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
	{#if !selectedItem}
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
						{
							value: "",
							label: t("createInstance.filterAllVersions"),
						},
						...gameVersionOptions,
					]}
					placeholder={t("createInstance.filterVersionLabel")}
					label={t("createInstance.filterVersionLabel")}
					onchange={(value) => setGameVersion(value || null)}
				/>
			</div>
		</div>
	{/if}

	{#if selectedItem}
		<div class="detail-view">
			<button
				type="button"
				class="back-btn"
				onclick={onBack}
				disabled={installing}
			>
				<Icon name="ui:chevron-left" size={16} />
				{t("createInstance.backBtn")}
			</button>

			<div class="detail-header">
				<div class="detail-icon-wrap" aria-hidden="true">
					<Icon name="instance:box" size={40} />
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
				</div>
				<div class="detail-title-group">
					<h3>{selectedItem.title}</h3>
					{#if selectedItem.author}
						<span class="detail-author">{selectedItem.author}</span>
					{/if}
					<p class="detail-desc">{selectedItem.description}</p>
					<span class="detail-downloads">
						{formatDownloads(selectedItem.downloads)}
						{t("createInstance.downloads")}
					</span>

					<div
						class="install-card"
						use:animateHeight={resizeDuration}
					>
						<div class="install-card-content">
							<div class="detail-actions">
								<div class="version-select">
									<span class="version-label">
										{t("createInstance.versionLabel")}
									</span>
									<Select
										bind:value={selectedVersion}
										options={versionOptions}
										placeholder={t(
											"createInstance.versionLabel",
										)}
										loading={loadingVersions}
										disabled={versionOptions.length === 0 ||
											installing}
									/>
								</div>

								<div class="install-action">
									{#if needsCustomName}
										<button
											type="button"
											class="install-btn"
											onclick={onCancelCustomName}
											disabled={installing}
										>
											{t("createInstance.cancel")}
										</button>
									{:else}
										<button
											type="button"
											class="install-btn"
											onclick={onInstall}
											aria-label={installing
												? t(
														"createInstance.installingModpack",
													)
												: t(
														"createInstance.installBtn",
													)}
											aria-busy={installing}
											disabled={installing ||
												loadingVersions ||
												!selectedVersion ||
												versionOptions.length === 0}
										>
											{#if installing}
												<Loading />
											{:else}
												<DownloadIcon size={16} />
												<span
													>{t(
														"createInstance.installBtn",
													)}</span
												>
											{/if}
										</button>
									{/if}
								</div>
							</div>
							{#if installing}
								<div class="install-status" role="status">
									{installStep ||
										t("createInstance.installingModpack")}
								</div>
							{/if}
							{#if installError}
								<div class="error-msg" role="alert">
									{installError}
								</div>
							{/if}
						</div>
					</div>
				</div>
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

			<h4 class="detail-separator">{t("createInstance.detailsTitle")}</h4>

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
		container-type: inline-size;
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
		border-top: 1px solid var(--border-color);
	}

	.install-status {
		border-top: 1px solid var(--border-color);
		padding: 10px 14px;
		color: var(--text-secondary);
		font-size: 0.75rem;
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
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 18px;
		overflow-wrap: anywhere;
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

	.back-btn:hover:not(:disabled) {
		color: var(--text-primary);
		border-color: var(--text-secondary);
	}

	.back-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.detail-separator {
		display: flex;
		align-items: center;
		gap: 12px;
		margin: 0;
		color: var(--text-muted);
		font-size: 0.75rem;
		font-style: italic;
		font-weight: 500;
	}

	.detail-separator::before,
	.detail-separator::after {
		content: "";
		height: 1px;
		background: var(--border-color);
	}

	.detail-separator::before {
		width: 20px;
	}

	.detail-separator::after {
		flex: 1;
	}

	.detail-header {
		display: flex;
		align-items: flex-start;
		gap: 20px;
	}

	.detail-icon-wrap {
		position: relative;
		width: 144px;
		aspect-ratio: 1;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		color: var(--text-muted);
		overflow: hidden;
	}

	.detail-icon {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		background: var(--bg-card);
		object-fit: cover;
	}

	.detail-title-group {
		display: flex;
		flex-direction: column;
		gap: 3px;
		min-width: 0;
		flex: 1;
	}

	.detail-title-group h3 {
		margin: 0;
		font-size: 1.25rem;
		line-height: 1.2;
		color: var(--text-primary);
		font-weight: 700;
	}

	.detail-author {
		font-size: 0.75rem;
		line-height: 1.2;
		color: var(--text-secondary);
	}

	.detail-downloads {
		font-size: 0.7rem;
		line-height: 1.2;
		color: var(--text-muted);
	}

	.detail-desc {
		font-size: 0.78rem;
		color: var(--text-secondary);
		line-height: 1.35;
		margin: 0;
	}

	.install-card {
		box-sizing: border-box;
		width: 100%;
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 var(--surface-selected);
		overflow: hidden;
	}

	.detail-actions {
		display: flex;
	}

	.version-select {
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: 2px;
		flex: 1;
		min-width: 0;
		padding: 8px;
	}

	.version-select :global(.select-trigger) {
		padding: 5px 8px;
		font-size: 0.75rem;
	}

	.version-label {
		font-size: 0.6rem;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.8px;
	}

	.install-action {
		border-left: 1px solid var(--border-color);
		display: flex;
		flex-shrink: 0;
		width: 96px;
	}

	.install-btn {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		width: 100%;
		padding: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		font-family: inherit;
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			color 0.15s,
			background 0.15s;
	}

	.install-btn:hover:not(:disabled) {
		color: var(--text-primary);
		background: var(--surface-selected);
	}

	.install-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.install-btn:focus-visible,
	.back-btn:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
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
		min-width: 0;
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

	@container (max-width: 480px) {
		.detail-header {
			gap: 14px;
		}

		.detail-icon-wrap {
			width: 112px;
		}

		.detail-title-group h3 {
			font-size: 1.05rem;
		}
	}

	@container (max-width: 340px) {
		.detail-header {
			flex-direction: column;
			align-items: flex-start;
		}

		.detail-title-group {
			align-self: stretch;
		}

		.custom-name-input-row {
			flex-direction: column;
			align-items: stretch;
		}
	}
</style>
