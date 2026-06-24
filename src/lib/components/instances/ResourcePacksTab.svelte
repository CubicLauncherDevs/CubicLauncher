<script lang="ts">
	import {
		getInstanceResourcePacks,
		deleteInstanceFile,
		addInstanceFile,
		searchModrinth,
		getModrinthProjectVersions,
		downloadResourcePacks,
		type ModDownloadInfo,
	} from "$lib/api/cubicApi";
	import type { ModrinthProject, ModrinthVersion, ModrinthFile } from "$lib/types/types";
	import { type ModDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import { open } from "@tauri-apps/plugin-dialog";
	import Loading from "../../icons/Loading.svelte";
	import Dropdown from "../layout/Dropdown.svelte";
	import VirtualList from "../layout/VirtualList.svelte";
	import { SvelteMap } from "svelte/reactivity";

	let { instanceId, gameVersion, loader } = $props<{
		instanceId: string;
		gameVersion?: string;
		loader?: string;
	}>();

	let packs = $state<ModDto[]>([]);
	let isLoading = $state(false);
	let prevInstanceId = $state<string>("");

	let mode = $state<"list" | "browse" | "review">("list");

	const PAGE_SIZE = 12;
	let query = $state("");
	let allHits = $state<ModrinthProject[]>([]);
	let totalHits = $state(0);
	let currentOffset = $state(0);
	let searching = $state(false);
	let loadingMore = $state(false);
	let sortIndex = $state<string>("downloads");
	let basket = new SvelteMap<string, ModrinthProject>();
	let selectedMod = $state<ModrinthProject | null>(null);
	let downloading = $state(false);
	let downloadQueue = $state<ModDownloadInfo[]>([]);
	let selectedModVersions = $state<ModrinthVersion[]>([]);
	let selectedVersionId = $state<string>("");
	let loadingVersions = $state(false);
	let versionSelection = new SvelteMap<string, string>();
	let installedPackNames = $state<Set<string>>(new Set());

	const cleanGameVersion = $derived(gameVersion ? getGameVersion(gameVersion) : undefined);

	let abortController = $state<AbortController | null>(null);
	let debounceTimer: ReturnType<typeof setTimeout> | undefined;

	async function loadPacks() {
		if (instanceId) {
			isLoading = true;
			packs = await getInstanceResourcePacks(instanceId);
			installedPackNames = new Set(packs.map((p) => p.name.toLowerCase()));
			isLoading = false;
		}
	}

	$effect(() => {
		if (instanceId !== prevInstanceId) {
			prevInstanceId = instanceId;
			loadPacks();
		}
	});

	async function handleDelete(pack: ModDto) {
		if (confirm(`¿Estás seguro de que deseas eliminar ${pack.filename}?`)) {
			await deleteInstanceFile(instanceId, "resourcepacks", pack.filename);
			await loadPacks();
		}
	}

	async function handleAdd() {
		const selected = await open({
			multiple: true,
			filters: [{ name: "Resource Pack", extensions: ["zip"] }],
		});
		if (selected && Array.isArray(selected)) {
			for (const path of selected) {
				await addInstanceFile(instanceId, "resourcepacks", path);
			}
			await loadPacks();
		} else if (selected && typeof selected === "string") {
			await addInstanceFile(instanceId, "resourcepacks", selected);
			await loadPacks();
		}
	}

	function openBrowser() {
		mode = "browse";
		query = "";
		allHits = [];
		totalHits = 0;
		currentOffset = 0;
		selectedMod = null;
		basket.clear();
		versionSelection.clear();
		performSearch();
	}

	function getGameVersion(versionStr: string): string {
		const lower = versionStr.toLowerCase();
		if (lower.includes("-forge-") || lower.includes("-neoforge-") || lower.includes("-quilt-")) {
			for (const sep of ["-forge-", "-neoforge-", "-quilt-"]) {
				const idx = lower.indexOf(sep);
				if (idx !== -1) return versionStr.slice(0, idx);
			}
		}
		if (lower.startsWith("fabric-loader-")) {
			const lastDash = versionStr.lastIndexOf("-");
			if (lastDash !== -1) return versionStr.slice(lastDash + 1);
		}
		return versionStr;
	}

	async function performSearch(resetResults = true) {
		abortController?.abort();
		abortController = new AbortController();
		const signal = abortController.signal;

		if (resetResults) {
			searching = true;
			allHits = [];
			currentOffset = 0;
			totalHits = 0;
		} else {
			loadingMore = true;
		}

		try {
			const result = await searchModrinth(
				query,
				"",
				cleanGameVersion,
				null,
				sortIndex,
				PAGE_SIZE,
				resetResults ? 0 : currentOffset,
				signal,
				"resourcepack",
			);
			if (result) {
				totalHits = result.total_hits;
				allHits = resetResults ? result.hits : [...allHits, ...result.hits];
				currentOffset = allHits.length;
			}
		} finally {
			if (!signal.aborted) {
				searching = false;
				loadingMore = false;
			}
		}
	}

	function onSearchInput(value: string) {
		query = value;
		clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => performSearch(true), 300);
	}

	function handleNearEnd() {
		if (!loadingMore && !searching && allHits.length < totalHits) {
			performSearch(false);
		}
	}

	function toggleBasket(project: ModrinthProject) {
		const pid = project.project_id;
		if (basket.has(pid)) {
			basket.delete(pid);
			versionSelection.delete(pid);
		} else {
			basket.set(pid, project);
			if (selectedVersionId) {
				versionSelection.set(pid, selectedVersionId);
			}
		}
	}

	async function startReview() {
		mode = "review";
		downloadQueue = [];

		const queue: ModDownloadInfo[] = [];
		for (const [id, project] of basket) {
			const versions = await getModrinthProjectVersions(
				id,
				undefined,
				cleanGameVersion,
			);
			if (versions && versions.length > 0) {
				let targetVersion: ModrinthVersion | undefined;
				const storedVersionId = versionSelection.get(id);
				if (storedVersionId) {
					targetVersion = versions.find((v) => v.id === storedVersionId);
				}
				if (!targetVersion) {
					targetVersion = versions[0];
				}
				const primaryFile = targetVersion.files.find((f: ModrinthFile) => f.primary) || targetVersion.files[0];
				if (!queue.find((q) => q.filename === primaryFile.filename)) {
					queue.push({
						url: primaryFile.url,
						filename: primaryFile.filename,
						projectTitle: project.title,
						iconUrl: project.icon_url || undefined,
					});
				}
			}
		}
		downloadQueue = queue;
	}

	async function confirmDownload() {
		downloading = true;
		try {
			await downloadResourcePacks(instanceId, downloadQueue);
			basket = new SvelteMap();
			mode = "list";
			selectedMod = null;
			await loadPacks();
		} finally {
			downloading = false;
		}
	}

	function formatNumber(num: number): string {
		if (num > 1000000) return (num / 1000000).toFixed(1) + "M";
		if (num > 1000) return (num / 1000).toFixed(1) + "K";
		return num.toString();
	}

	function isPackInstalled(project: ModrinthProject): boolean {
		return installedPackNames.has(project.title.toLowerCase());
	}

	function isPackCompatible(project: ModrinthProject): boolean {
		if (cleanGameVersion) {
			return project.versions.some((v) => getGameVersion(v) === cleanGameVersion);
		}
		return true;
	}

	async function loadVersions(projectId: string) {
		loadingVersions = true;
		selectedModVersions = [];
		selectedVersionId = "";
		try {
			const versions = await getModrinthProjectVersions(projectId, undefined, cleanGameVersion);
			const sorted = [...versions].sort((a, b) => {
				const aCompat = cleanGameVersion ? (a.game_versions?.some((v) => getGameVersion(v) === cleanGameVersion) ? 1 : 0) : 1;
				const bCompat = cleanGameVersion ? (b.game_versions?.some((v) => getGameVersion(v) === cleanGameVersion) ? 1 : 0) : 1;
				return bCompat - aCompat;
			});
			selectedModVersions = sorted;
			if (sorted.length > 0) {
				const stored = versionSelection.get(projectId);
				if (stored && sorted.find((v) => v.id === stored)) {
					selectedVersionId = stored;
				} else if (cleanGameVersion) {
					const compatible = sorted.find((v) =>
						v.game_versions?.some((gv) => getGameVersion(gv) === cleanGameVersion),
					);
					if (compatible) {
						selectedVersionId = compatible.id;
						versionSelection.set(projectId, compatible.id);
					} else {
						selectedVersionId = sorted[0].id;
						if (!versionSelection.has(projectId)) {
							versionSelection.set(projectId, sorted[0].id);
						}
					}
				} else {
					selectedVersionId = sorted[0].id;
					if (!versionSelection.has(projectId)) {
						versionSelection.set(projectId, sorted[0].id);
					}
				}
			}
		} finally {
			loadingVersions = false;
		}
	}

	function onVersionChange() {
		if (selectedVersionId && selectedMod) {
			versionSelection.set(selectedMod.project_id, selectedVersionId);
		}
	}

	const versionDropdownOptions = $derived(
		selectedModVersions.map((v) => {
			const compatible = cleanGameVersion
				? v.game_versions?.some((gv) => getGameVersion(gv) === cleanGameVersion)
				: true;
			const subtitle = compatible ? t("instanceView.downloadMods.compatible") : v.game_versions?.slice(0, 2).join(", ");
			return {
				value: v.id,
				label: v.version_number,
				subtitle,
			};
		}),
	);

	$effect(() => {
		if (selectedMod && mode === "browse") {
			loadVersions(selectedMod.project_id);
		}
	});
</script>

{#if mode === "review"}
	<div class="rp-review">
		<div class="rp-review-header">
			<div>
				<span class="rp-section-label">{t("instanceView.resources.sectionLabel")}</span>
				<h2 class="rp-review-title">{t("instanceView.downloadMods.reviewTitle")}</h2>
			</div>
			<button type="button" class="rp-back-btn" onclick={() => (mode = "browse")} disabled={downloading}>
				← {t("instanceView.downloadMods.back")}
			</button>
		</div>

		<div class="rp-review-body">
			{#if downloadQueue.length === 0}
				<div class="rp-center-state">
					<p>{t("instanceView.resources.noSelection")}</p>
				</div>
			{:else}
				<div class="rp-queue-box">
					<p class="rp-queue-subtitle">
						{downloadQueue.length}
						{downloadQueue.length === 1 ? t("instanceView.downloadMods.file_one") : t("instanceView.downloadMods.file_other")}
						{t("instanceView.resources.toDownload")}
					</p>
					<div class="rp-queue-list">
						{#each downloadQueue as item (item.filename)}
							<div class="rp-queue-item">
								{#if item.iconUrl}
									<img src={item.iconUrl} alt="" class="rp-queue-icon-img" />
								{:else}
									<span class="rp-queue-icon">🎨</span>
								{/if}
								<div class="rp-queue-item-info">
									{#if item.projectTitle}
										<span class="rp-queue-title">{item.projectTitle}</span>
									{/if}
									<span class="rp-queue-filename">{item.filename}</span>
								</div>
							</div>
						{/each}
					</div>
				</div>

				<div class="rp-review-footer">
					<span class="rp-review-count">
						<strong>{downloadQueue.length}</strong>
						{downloadQueue.length !== 1 ? t("instanceView.downloadMods.file_other") : t("instanceView.downloadMods.file_one")}
					</span>
					<button type="button" class="rp-primary-btn" onclick={confirmDownload} disabled={downloading}>
						{#if downloading}
							<Loading />
							{t("instanceView.downloadMods.downloading")}
						{:else}
							{t("instanceView.downloadMods.confirmDownload")}
						{/if}
					</button>
				</div>
			{/if}
		</div>
	</div>
{:else if mode === "browse"}
	<div class="rp-browse">
		<div class="rp-browse-header">
			<span class="rp-section-label">{t("instanceView.resources.sectionLabel")}</span>
			<button type="button" class="rp-back-btn" onclick={() => (mode = "list")}>
				{t("instanceView.resources.backToList")}
			</button>
		</div>

		<div class="rp-search-bar-wrap">
			<span class="rp-search-icon">
				<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<circle cx="11" cy="11" r="8" /><path d="m21 21-4.35-4.35" />
				</svg>
			</span>
			<input
				class="rp-search-input"
				type="text"
				placeholder={t("instanceView.resources.searchPlaceholder")}
				oninput={(e) => onSearchInput(e.currentTarget.value)}
				onkeydown={(e) => e.key === "Enter" && performSearch(true)}
			/>
			{#if query}
				<button type="button" class="rp-search-clear" onclick={() => { query = ""; performSearch(true); }}>×</button>
			{/if}
		</div>

		{#if totalHits > 0 && !searching}
			<div class="rp-results-meta">
				<span>{totalHits.toLocaleString()} {t("instanceView.resources.resultsFound")}</span>
			</div>
		{/if}

		<div class="rp-results-area">
			{#if searching}
				<div class="rp-center-state">
					<Loading />
					<p>{t("instanceView.resources.searching")}</p>
				</div>
			{:else if allHits.length > 0}
				<div class="rp-vlist-wrap">
					<VirtualList items={allHits} itemHeight={130} onNearEnd={handleNearEnd}>
						{#snippet children(project)}
							<div
								class="rp-pack-card-v {selectedMod && selectedMod.project_id === project.project_id ? 'selected' : ''}"
								onclick={() => (selectedMod = project)}
								onkeydown={() => {}}
								role="button"
								tabindex="0"
							>
								<div class="rp-pack-icon-v">
									{#if project.icon_url}
										<img src={project.icon_url} alt={project.title} loading="lazy" />
									{:else}
										<span class="rp-pack-icon-placeholder">🎨</span>
									{/if}
								</div>
								<div class="rp-pack-body-v">
									<div class="rp-pack-top-v">
										<h4 class="rp-pack-title-v" title={project.title}>{project.title}</h4>
										<div class="rp-pack-badges-v">
											{#if isPackInstalled(project)}
												<span class="rp-installed-badge">{t("instanceView.downloadMods.installed")}</span>
											{/if}
											{#if cleanGameVersion && !isPackCompatible(project)}
												<span class="rp-incompat-badge">
													{t("instanceView.downloadMods.noVersionCompat").replace("{version}", cleanGameVersion)}
												</span>
											{/if}
										</div>
									</div>
									<span class="rp-pack-author-v">{t("instanceView.downloadMods.by")} {project.author}</span>
									<p class="rp-pack-desc-v">{project.description}</p>
								</div>
								<div class="rp-pack-actions-v">
									<span class="rp-pack-stat">↓ {formatNumber(project.downloads)}</span>
									<button
										type="button"
										class="rp-select-btn {basket.has(project.project_id) ? 'selected' : ''}"
										onclick={(e) => { e.stopPropagation(); toggleBasket(project); }}
									>
										{basket.has(project.project_id)
											? t("instanceView.downloadMods.selected")
											: t("instanceView.downloadMods.select")}
									</button>
								</div>
							</div>
						{/snippet}
					</VirtualList>
					{#if loadingMore}
						<div class="rp-vlist-loading">
							<Loading class="rp-spinning" />
							<span>{t("instanceView.resources.loadingMore")}</span>
						</div>
					{:else if allHits.length >= totalHits && totalHits > 0}
						<div class="rp-vlist-end">
							<span class="rp-end-label">
								— {t("instanceView.resources.endOfResults").replace("{count}", allHits.length.toString())} —
							</span>
						</div>
					{/if}
				</div>
			{:else}
				<div class="rp-center-state">
				<p>{t("instanceView.resources.noResults")}</p>
				<button type="button" class="rp-ghost-btn" onclick={() => { query = ""; performSearch(true); }}>
					{t("instanceView.resources.clearFilters")}
				</button>
				</div>
			{/if}
		</div>

		{#if selectedMod}
			<aside class="rp-details">
				<button
					type="button"
					class="rp-close-btn"
					aria-label={t("instanceView.downloadMods.closeDetails")}
					onclick={() => (selectedMod = null)}
				>
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
						<path d="M18 6 6 18M6 6l12 12" />
					</svg>
				</button>
				<div class="rp-details-scroll">
					<div class="rp-details-icon">
						{#if selectedMod.icon_url}
							<img src={selectedMod.icon_url} alt={selectedMod.title} />
						{:else}
							<span>🎨</span>
						{/if}
					</div>
					<h3 class="rp-details-title">{selectedMod.title}</h3>
					<p class="rp-details-author">{t("instanceView.downloadMods.by")} {selectedMod.author}</p>

					<div class="rp-details-stat-row">
						<div class="rp-details-stat">
							<span class="rp-details-stat-label">{t("instanceView.downloadMods.downloads")}</span>
							<span class="rp-details-stat-value">{formatNumber(selectedMod.downloads)}</span>
						</div>
					</div>

					<div class="rp-tags">
						{#each selectedMod.categories.slice(0, 4) as cat}
							<span class="rp-tag">{cat}</span>
						{/each}
					</div>

					<div class="rp-details-version-row">
						<span class="rp-details-version-label">{t("instanceView.downloadMods.versionLabel")}</span>
						{#if loadingVersions}
							<span class="rp-loading-versions">{t("instanceView.downloadMods.loadingVersions")}</span>
						{:else if selectedModVersions.length === 0}
							<span class="rp-no-versions-msg">
								{t("instanceView.downloadMods.noCompatibleVersions").replace("{version}", cleanGameVersion || "")}
							</span>
						{:else}
							<Dropdown
								bind:value={selectedVersionId}
								options={versionDropdownOptions}
								placeholder={t("instanceView.downloadMods.anyVersion")}
								onchange={onVersionChange}
							/>
						{/if}
					</div>

					<p class="rp-details-desc">{selectedMod.description}</p>

					<button
						type="button"
						class="rp-primary-btn rp-full-width {basket.has(selectedMod.project_id) ? 'rp-btn-remove' : ''}"
						onclick={() => toggleBasket(selectedMod!)}
					>
						{basket.has(selectedMod.project_id)
							? t("instanceView.downloadMods.removeSelection")
							: t("instanceView.downloadMods.selectToDownload")}
					</button>
				</div>
			</aside>
		{/if}

		{#if basket.size > 0}
			<div class="rp-bottom-bar">
				<span class="rp-bottom-count">{t("instanceView.downloadMods.selectionLabel")}: {basket.size}</span>
				<button type="button" class="rp-primary-btn" onclick={startReview}>
					{t("instanceView.downloadMods.reviewBtn")}
				</button>
			</div>
		{/if}
	</div>
{:else}
	<div class="resources-section">
		<div class="section-header">
			<span class="section-title">{t("instanceView.resources.title")} ({packs.length})</span>
			<div class="section-actions">
				<button type="button" class="browse-btn" onclick={openBrowser}>
					{t("instanceView.resources.getPacks")}
				</button>
				<button type="button" class="add-btn" onclick={handleAdd}>
					{t("instanceView.resources.addBtn")}
				</button>
			</div>
		</div>

		<div class="packs-grid">
			{#each packs as pack (pack.filename)}
				<div class="pack-card">
					<div class="pack-icon">
						{#if pack.icon}
							<img src={pack.icon} alt={pack.name} />
						{:else}
							<div class="mod-icon-placeholder">🎨</div>
						{/if}
					</div>
					<div class="pack-info">
						<span class="pack-name" title={pack.name}>{pack.name}</span>
						<p class="pack-description" title={pack.description}>
							{pack.description || t("instanceView.mods.noDescription")}
						</p>
					</div>
					<button
						type="button"
						class="delete-btn"
						onclick={() => handleDelete(pack)}
						title="Eliminar"
					>
						<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18" /><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" /><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" /><line x1="10" y1="11" x2="10" y2="17" /><line x1="14" y1="11" x2="14" y2="17" /></svg>
					</button>
				</div>
			{/each}

			{#if packs.length === 0 && !isLoading}
				<div class="empty-state">
					{t("instanceView.resources.empty")}
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.resources-section {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.section-title {
		font-size: 1.2rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.section-actions {
		display: flex;
		gap: 8px;
	}

	.browse-btn,
	.add-btn {
		padding: 0.5rem 1rem;
		background: var(--accent);
		color: var(--bg-main);
		border: none;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-weight: 600;
		transition: opacity 0.2s;
	}

	.browse-btn:hover,
	.add-btn:hover {
		opacity: 0.9;
	}

	.packs-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
		gap: 0.75rem;
	}

	.pack-card {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 14px;
		display: flex;
		gap: 14px;
		align-items: flex-start;
		transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
		position: relative;
	}

	.pack-card:hover {
		background: rgba(255, 255, 255, 0.06);
		border-color: rgba(255, 255, 255, 0.1);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
	}

	.pack-icon {
		width: 48px;
		height: 48px;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		overflow: hidden;
	}

	.pack-icon img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		image-rendering: pixelated;
	}

	.pack-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.pack-name {
		font-size: 0.88rem;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.pack-description {
		font-size: 0.72rem;
		color: var(--text-secondary);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		line-height: 1.4;
		margin-top: 4px;
	}

	.delete-btn {
		background: transparent;
		border: none;
		color: #ff4444;
		cursor: pointer;
		padding: 4px;
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background 0.2s;
	}

	.delete-btn:hover {
		background: rgba(255, 68, 68, 0.1);
	}

	.empty-state {
		grid-column: 1 / -1;
		text-align: center;
		padding: 2rem;
		color: var(--text-secondary);
		background: rgba(255, 255, 255, 0.02);
		border-radius: var(--border-radius-sm);
		border: 1px dashed rgba(255, 255, 255, 0.1);
	}

	/* Browse mode styles */
	.rp-browse {
		display: flex;
		flex-direction: column;
		height: calc(100% + 64px);
		margin: -32px -40px;
		background: var(--bg-main);
		color: var(--text-primary);
		overflow: hidden;
		position: relative;
	}

	.rp-browse-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 20px;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.rp-section-label {
		font-size: 0.62rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 1.5px;
		color: var(--text-secondary);
	}

	.rp-search-bar-wrap {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 12px 20px;
		background: var(--bg-sidebar);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.rp-search-icon {
		color: var(--text-secondary);
		display: flex;
		align-items: center;
		flex-shrink: 0;
	}
	.rp-search-input {
		flex: 1;
		background: transparent;
		border: none;
		color: var(--text-primary);
		font-size: 0.9rem;
		outline: none;
	}
	.rp-search-input::placeholder {
		color: var(--text-secondary);
	}
	.rp-search-clear {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 1.2rem;
		line-height: 1;
		transition: color 0.2s;
	}
	.rp-search-clear:hover {
		color: var(--text-primary);
	}

	.rp-results-meta {
		padding: 8px 20px;
		font-size: 0.72rem;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 1px;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.rp-results-area {
		flex: 1;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.rp-vlist-wrap {
		flex: 1;
		overflow: hidden;
		display: flex;
		flex-direction: column;
		padding: 4px 16px 0px 16px;
	}
	.rp-vlist-loading,
	.rp-vlist-end {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 16px 0 8px;
		color: var(--text-secondary);
		font-size: 0.78rem;
		flex-shrink: 0;
	}
	.rp-end-label {
		font-size: 0.7rem;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 1px;
	}

	.rp-pack-card-v {
		display: flex;
		align-items: stretch;
		gap: 14px;
		padding: 14px 16px;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition: all 0.15s cubic-bezier(0.4, 0, 0.2, 1);
		height: calc(100% - 6px);
		margin: 3px 0;
		box-sizing: border-box;
	}

	.rp-pack-card-v:hover {
		background: rgba(255, 255, 255, 0.04);
		border-color: rgba(255, 255, 255, 0.1);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
	}
	.rp-pack-card-v.selected {
		border-color: var(--accent);
		background: rgba(255, 255, 255, 0.04);
	}

	.rp-pack-icon-v {
		width: 56px;
		height: 56px;
		border-radius: var(--border-radius-sm);
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		overflow: hidden;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		align-self: center;
	}

	.rp-pack-icon-v img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		image-rendering: pixelated;
	}

	.rp-pack-body-v {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: 4px;
		overflow: hidden;
	}
	.rp-pack-top-v {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}
	.rp-pack-title-v {
		font-size: 0.85rem;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}
	.rp-pack-badges-v {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-shrink: 0;
	}
	.rp-pack-author-v {
		font-size: 0.7rem;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.rp-pack-desc-v {
		font-size: 0.75rem;
		color: var(--text-secondary);
		line-height: 1.35;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}

	.rp-pack-actions-v {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		justify-content: center;
		gap: 6px;
		flex-shrink: 0;
	}
	.rp-pack-stat {
		font-size: 0.72rem;
		color: var(--text-secondary);
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid var(--border);
		padding: 2px 7px;
		border-radius: var(--border-radius-sm);
		white-space: nowrap;
	}

	.rp-select-btn {
		padding: 4px 12px;
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.75rem;
		font-weight: 600;
		transition: all 0.15s;
	}

	.rp-select-btn:hover {
		background: rgba(255, 255, 255, 0.06);
		color: var(--text-primary);
		border-color: rgba(255, 255, 255, 0.2);
	}
	.rp-select-btn.selected {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--bg-main);
	}

	.rp-installed-badge {
		font-size: 0.6rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: #4ade80;
		background: rgba(74, 222, 128, 0.1);
		border: 1px solid rgba(74, 222, 128, 0.25);
		padding: 1px 5px;
		border-radius: var(--border-radius-sm);
		white-space: nowrap;
	}

	:global(.rp-spinning) {
		animation: spin 0.8s linear infinite;
		will-change: transform;
	}

	.rp-details {
		position: fixed;
		right: 0;
		top: 0;
		bottom: 0;
		width: 280px;
		flex-shrink: 0;
		background-color: var(--bg-sidebar);
		border-left: 1px solid var(--border);
		z-index: 100;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		animation: rp-slide-in 0.25s cubic-bezier(0.2, 0.8, 0.2, 1);
		box-shadow: -10px 0 30px rgba(0, 0, 0, 0.5);
	}

	@keyframes rp-slide-in {
		from {
			transform: translateX(30px);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}

	.rp-close-btn {
		position: absolute;
		top: 14px;
		right: 14px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		width: 26px;
		height: 26px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s;
		z-index: 2;
	}

	.rp-close-btn:hover {
		background: rgba(255, 255, 255, 0.1);
		color: var(--text-primary);
	}

	.rp-details-scroll {
		padding: 20px 16px;
		overflow-y: auto;
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.rp-details-icon {
		width: 80px;
		height: 80px;
		margin: 8px auto 0 auto;
		border-radius: var(--border-radius-sm);
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 2rem;
	}

	.rp-details-icon img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		image-rendering: pixelated;
	}
	.rp-details-title {
		font-size: 1rem;
		font-weight: 700;
		color: var(--text-primary);
		text-align: center;
		margin: 0;
	}
	.rp-details-author {
		font-size: 0.75rem;
		color: var(--text-secondary);
		text-align: center;
		margin: 0;
	}
	.rp-details-stat-row {
		display: flex;
		gap: 8px;
	}
	.rp-details-stat {
		flex: 1;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 8px 10px;
		text-align: center;
	}
	.rp-details-stat-label {
		display: block;
		font-size: 0.62rem;
		text-transform: uppercase;
		letter-spacing: 1px;
		color: var(--text-secondary);
		margin-bottom: 4px;
	}
	.rp-details-stat-value {
		font-size: 0.95rem;
		font-weight: 700;
		color: var(--text-primary);
	}
	.rp-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.rp-tag {
		font-size: 0.68rem;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid var(--border);
		padding: 2px 8px;
		border-radius: 20px;
		color: var(--text-secondary);
		text-transform: capitalize;
	}
	.rp-details-desc {
		font-size: 0.8rem;
		line-height: 1.55;
		color: var(--text-secondary);
		margin: 0;
	}
	.rp-details-version-row {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.rp-details-version-label {
		font-size: 0.68rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 1px;
		color: var(--text-secondary);
	}
	.rp-no-versions-msg {
		font-size: 0.75rem;
		color: #f87171;
		text-align: center;
		padding: 8px;
		border: 1px solid rgba(248, 113, 113, 0.2);
		border-radius: var(--border-radius-sm);
		background: rgba(248, 113, 113, 0.06);
	}
	.rp-loading-versions {
		font-size: 0.72rem;
		color: var(--text-secondary);
		text-align: center;
	}
	.rp-incompat-badge {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 0.62rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: #f87171;
		background: rgba(248, 113, 113, 0.08);
		border: 1px solid rgba(248, 113, 113, 0.2);
		padding: 2px 6px;
		border-radius: var(--border-radius-sm);
		white-space: nowrap;
	}
	.rp-queue-icon-img {
		width: 24px;
		height: 24px;
		border-radius: var(--border-radius-sm);
		object-fit: cover;
		flex-shrink: 0;
	}

	.rp-bottom-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 24px;
		background: var(--bg-sidebar);
		border-top: 1px solid var(--border);
		flex-shrink: 0;
	}

	.rp-bottom-count {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.rp-center-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		min-height: 240px;
		gap: 14px;
		color: var(--text-secondary);
		font-size: 0.85rem;
		text-transform: uppercase;
		letter-spacing: 1px;
	}

	.rp-pack-icon-placeholder {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 1.2rem;
		color: var(--text-secondary);
		background: rgba(255, 255, 255, 0.05);
	}

	.rp-primary-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 8px 18px;
		background: var(--accent);
		color: var(--bg-main);
		border: none;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.82rem;
		font-weight: 700;
		letter-spacing: 0.3px;
		transition: all 0.15s;
	}
	.rp-primary-btn:hover:not(:disabled) {
		filter: brightness(0.9);
	}
	.rp-primary-btn:disabled {
		opacity: 0.35;
		cursor: not-allowed;
	}
	.rp-primary-btn.rp-btn-remove {
		background: rgba(255, 255, 255, 0.06);
		color: var(--text-primary);
		border: 1px solid var(--border);
	}
	.rp-primary-btn.rp-btn-remove:hover:not(:disabled) {
		background: rgba(255, 68, 68, 0.12);
		color: #ff6b6b;
		border-color: rgba(255, 68, 68, 0.3);
	}
	.rp-full-width {
		width: 100%;
	}

	.rp-back-btn {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 7px 14px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.82rem;
		transition: all 0.15s;
	}
	.rp-back-btn:hover:not(:disabled) {
		background: var(--bg-item-active);
		color: var(--text-primary);
	}
	.rp-back-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.rp-ghost-btn {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 7px 16px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.8rem;
		transition: all 0.15s;
	}
	.rp-ghost-btn:hover {
		background: var(--bg-item-active);
		color: var(--text-primary);
	}

	.rp-review {
		display: flex;
		flex-direction: column;
		height: 100%;
		padding: 28px 32px;
	}
	.rp-review-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 24px;
		padding-bottom: 16px;
		border-bottom: 1px solid var(--border);
	}
	.rp-review-title {
		font-size: 1.3rem;
		font-weight: 700;
		color: var(--text-primary);
		margin: 0;
	}
	.rp-review-body {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.rp-queue-box {
		flex: 1;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 16px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.rp-queue-subtitle {
		font-size: 0.78rem;
		color: var(--text-secondary);
		margin: 0 0 14px 0;
	}
	.rp-queue-list {
		flex: 1;
		overflow-y: auto;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 8px;
		align-content: flex-start;
	}
	.rp-queue-item {
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		padding: 10px 12px;
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.rp-queue-item-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
		gap: 2px;
	}
	.rp-queue-title {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.rp-queue-icon {
		font-size: 1rem;
		opacity: 0.6;
	}
	.rp-queue-filename {
		font-size: 0.8rem;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.rp-review-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 16px;
		padding: 14px 16px;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
	}
	.rp-review-count {
		font-size: 0.85rem;
		color: var(--text-secondary);
	}
	.rp-review-count strong {
		color: var(--text-primary);
		font-size: 1.1rem;
	}

	@media (max-width: 1200px) {
		.rp-details {
			width: 260px;
		}
	}

	@media (max-width: 950px) {
		.rp-details {
			width: 320px;
		}
	}

	@media (max-width: 700px) {
		.rp-search-bar-wrap {
			padding: 12px 16px;
		}
		.rp-results-meta {
			padding: 6px 16px;
		}
		.rp-vlist-wrap {
			padding: 4px 16px 12px;
		}
		.rp-bottom-bar {
			padding: 10px 16px;
		}
	}

	@media (max-width: 550px) {
		.rp-details {
			width: 100%;
		}
		.rp-vlist-wrap {
			padding: 4px 12px 8px;
		}
		.rp-pack-card-v {
			padding: 10px 12px;
			gap: 10px;
		}
		.rp-pack-icon-v {
			width: 36px;
			height: 36px;
		}
		.rp-pack-title-v {
			font-size: 0.85rem;
		}
	}

	@media (max-width: 700px) {
		.packs-grid {
			grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		}
	}

	@media (max-width: 550px) {
		.packs-grid {
			grid-template-columns: 1fr;
			gap: 8px;
		}
	}

	@media (max-width: 400px) {
		.packs-grid {
			grid-template-columns: 1fr;
			gap: 6px;
		}
	}
</style>
