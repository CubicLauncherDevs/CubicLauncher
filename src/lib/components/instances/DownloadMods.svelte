<script lang="ts">
	import {
		searchModrinth,
		getModrinthProjectVersions,
		searchCurseForge,
		getCurseForgeProjectFiles,
		getCurseForgeFileDownloadUrl,
		downloadMods,
		getInstanceMods,
		type ModDownloadInfo,
	} from "$lib/api/cubicApi";
	import type {
		ModrinthProject,
		ModrinthVersion,
		ModrinthFile,
		CurseForgeProject,
		CurseForgeFile,
		InstanceDto,
		ModSource,
	} from "$lib/types/types";
	import { t } from "$lib/i18n";
	import Loading from "../../icons/Loading.svelte";
	import Dropdown from "../layout/Dropdown.svelte";
	import VirtualList from "../layout/VirtualList.svelte";
	import { SvelteMap } from "svelte/reactivity";

	let { instance } = $props<{ instance: InstanceDto }>();

	const PAGE_SIZE = 12;

	let source = $state<ModSource>("modrinth");

	let query = $state("");
	let allHits = $state<(ModrinthProject | CurseForgeProject)[]>([]);
	let totalHits = $state(0);
	let currentOffset = $state(0);
	let searching = $state(true);
	let loadingMore = $state(false);
	let activeCategory = $state<string | null>(null);
	let sortIndex = $state<string>("downloads");

	let basket = new SvelteMap<string, ModrinthProject | CurseForgeProject>();

	let selectedMod = $state<ModrinthProject | CurseForgeProject | null>(null);

	let reviewing = $state(false);
	let resolvingDeps = $state(false);
	let downloading = $state(false);
	let downloadQueue = $state<ModDownloadInfo[]>([]);

	let selectedModVersions = $state<(ModrinthVersion | CurseForgeFile)[]>([]);
	let selectedVersionId = $state<string>("");
	let loadingVersions = $state(false);
	let versionSelection = new SvelteMap<string, string>();

	let installedModNames = $state<Set<string>>(new Set());

	function getGameVersion(versionStr: string): string {
		const segments = versionStr.split("-");
		if (segments.length > 1) {
			return segments[segments.length - 1];
		}
		return versionStr;
	}

	const gameVersion = $derived(getGameVersion(instance.version));

	let abortController = $state<AbortController | null>(null);
	let debounceTimer: ReturnType<typeof setTimeout> | undefined;

	function getProjectId(
		project: ModrinthProject | CurseForgeProject,
	): string {
		return "project_id" in project
			? project.project_id
			: project.id.toString();
	}

	function getProjectTitle(
		project: ModrinthProject | CurseForgeProject,
	): string {
		return "title" in project ? project.title : project.name;
	}

	function getProjectAuthor(
		project: ModrinthProject | CurseForgeProject,
	): string {
		return "author" in project
			? project.author
			: project.authors.map((a) => a.name).join(", ");
	}

	function getProjectDescription(
		project: ModrinthProject | CurseForgeProject,
	): string {
		return "description" in project ? project.description : project.summary;
	}

	function getProjectIcon(
		project: ModrinthProject | CurseForgeProject,
	): string | null {
		return "icon_url" in project
			? project.icon_url
			: (project.logo?.url ?? null);
	}

	function getProjectDownloads(
		project: ModrinthProject | CurseForgeProject,
	): number {
		return "downloads" in project
			? project.downloads
			: project.downloadCount;
	}

	function isModrinthProject(
		p: ModrinthProject | CurseForgeProject,
	): p is ModrinthProject {
		return "project_id" in p;
	}

	function isCurseForgeProject(
		p: ModrinthProject | CurseForgeProject,
	): p is CurseForgeProject {
		return "id" in p && !("project_id" in p);
	}

	function isCurseForgeFile(
		v: ModrinthVersion | CurseForgeFile,
	): v is CurseForgeFile {
		return "fileName" in v;
	}

	function getProjectCategories(
		project: ModrinthProject | CurseForgeProject,
	): string[] {
		if (isModrinthProject(project)) {
			return project.categories;
		}
		return project.categories.map((c) => c.slug);
	}

	function isFromSource(
		project: ModrinthProject | CurseForgeProject,
		s: ModSource,
	): boolean {
		if (s === "modrinth") return isModrinthProject(project);
		return isCurseForgeProject(project);
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
			if (source === "modrinth") {
				const result = await searchModrinth(
					query,
					instance.loader,
					gameVersion,
					activeCategory,
					sortIndex,
					PAGE_SIZE,
					resetResults ? 0 : currentOffset,
					signal,
				);
				if (result) {
					totalHits = result.total_hits;
					allHits = resetResults
						? result.hits
						: [...allHits, ...result.hits];
					currentOffset = allHits.length;
				}
			} else {
				const cfCategory =
					activeCategory && !isNaN(Number(activeCategory))
						? activeCategory
						: null;
				const result = await searchCurseForge(
					query,
					instance.loader,
					gameVersion,
					cfCategory,
					sortIndex,
					PAGE_SIZE,
					resetResults ? 0 : currentOffset,
					signal,
				);
				if (result) {
					totalHits = result.pagination.totalCount;
					allHits = resetResults
						? result.data
						: [...allHits, ...result.data];
					currentOffset = allHits.length;
				}
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

	function switchSource(newSource: ModSource) {
		source = newSource;
		query = "";
		allHits = [];
		totalHits = 0;
		currentOffset = 0;
		searching = true;
		selectedMod = null;
		basket = new SvelteMap();
		versionSelection = new SvelteMap();
		reviewing = false;
		performSearch(true);
	}

	function resetState() {
		query = "";
		allHits = [];
		totalHits = 0;
		currentOffset = 0;
		searching = true;
		loadingMore = false;
		activeCategory = null;
		sortIndex = "downloads";
		basket = new SvelteMap();
		selectedMod = null;
		reviewing = false;
		resolvingDeps = false;
		downloading = false;
		downloadQueue = [];
		selectedModVersions = [];
		selectedVersionId = "";
		loadingVersions = false;
		versionSelection = new SvelteMap();
		installedModNames = new Set();
	}

	let pendingInstanceId: string | null = null;
	let prevInstanceId = $state<string>("");

	$effect(() => {
		const id = instance.uuid;
		if (id === prevInstanceId) return;
		prevInstanceId = id;
		pendingInstanceId = id;
		resetState();
		getInstanceMods(id).then((mods) => {
			if (pendingInstanceId !== id) return;
			installedModNames = new Set(mods.map((m) => m.name.toLowerCase()));
		});
		performSearch();
	});

	function toggleBasket(project: ModrinthProject | CurseForgeProject) {
		const pid = getProjectId(project);
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
		reviewing = true;
		resolvingDeps = true;
		downloadQueue = [];

		try {
			const installedMods = await getInstanceMods(instance.uuid);
			const installedFilenames = new Set(
				installedMods.map((m) => m.filename.toLowerCase()),
			);

			const queue: ModDownloadInfo[] = [];
			for (const [id, project] of basket) {
				if (isFromSource(project, "modrinth")) {
					const mrProject = project as ModrinthProject;
					const versions = await getModrinthProjectVersions(
						id,
						instance.loader,
						gameVersion,
					);
					if (versions && versions.length > 0) {
						let targetVersion: ModrinthVersion | undefined;
						const storedVersionId = versionSelection.get(id);
						if (storedVersionId) {
							targetVersion = versions.find(
								(v) => v.id === storedVersionId,
							);
						}
						if (!targetVersion) {
							targetVersion = versions[0];
						}
						const primaryFile =
							targetVersion.files.find(
								(f: ModrinthFile) => f.primary,
							) || targetVersion.files[0];
						if (
							!queue.find(
								(q) => q.filename === primaryFile.filename,
							)
						) {
							queue.push({
								url: primaryFile.url,
								filename: primaryFile.filename,
								projectTitle: mrProject.title,
								iconUrl: mrProject.icon_url || undefined,
							});
						}

						if (targetVersion.dependencies) {
							for (const dep of targetVersion.dependencies) {
								if (
									dep.dependency_type === "required" &&
									dep.project_id
								) {
									const depVersions =
										await getModrinthProjectVersions(
											dep.project_id,
											instance.loader,
											gameVersion,
										);
									if (depVersions && depVersions.length > 0) {
										const depLatest = depVersions[0];
										const depFile =
											depLatest.files.find(
												(f: ModrinthFile) => f.primary,
											) || depLatest.files[0];

										const alreadyInstalled =
											installedFilenames.has(
												depFile.filename.toLowerCase(),
											);
										const alreadyQueued = queue.find(
											(q) =>
												q.filename === depFile.filename,
										);
										if (
											!alreadyInstalled &&
											!alreadyQueued
										) {
											queue.push({
												url: depFile.url,
												filename: depFile.filename,
											});
										}
									}
								}
							}
						}
					}
				} else {
					const cfProject = project as CurseForgeProject;
					const files = await getCurseForgeProjectFiles(
						cfProject.id,
						instance.loader,
						gameVersion,
					);
					if (files && files.length > 0) {
						let targetFile: CurseForgeFile | undefined;
						const storedFileId = versionSelection.get(id);
						if (storedFileId) {
							targetFile = files.find(
								(f) => f.id.toString() === storedFileId,
							);
						}
						if (!targetFile) {
							targetFile = files[0];
						}
						let downloadUrl = targetFile.downloadUrl;
						if (!downloadUrl) {
							downloadUrl = await getCurseForgeFileDownloadUrl(
								cfProject.id,
								targetFile.id,
							);
						}
						if (downloadUrl) {
							if (
								!queue.find(
									(q) => q.filename === targetFile!.fileName,
								)
							) {
								queue.push({
									url: downloadUrl,
									filename: targetFile.fileName,
									projectTitle: cfProject.name,
									iconUrl: cfProject.logo?.url || undefined,
								});
							}
						}
					}
				}
			}
			downloadQueue = queue;
		} finally {
			resolvingDeps = false;
		}
	}

	async function confirmDownload() {
		downloading = true;
		try {
			await downloadMods(instance.uuid, downloadQueue);
			basket = new SvelteMap();
			reviewing = false;
			selectedMod = null;
		} finally {
			downloading = false;
		}
	}

	function formatNumber(num: number): string {
		if (num > 1000000) return (num / 1000000).toFixed(1) + "M";
		if (num > 1000) return (num / 1000).toFixed(1) + "K";
		return num.toString();
	}

	function isModInstalled(
		project: ModrinthProject | CurseForgeProject,
	): boolean {
		return installedModNames.has(getProjectTitle(project).toLowerCase());
	}

	function isVersionCompatibleModrinth(version: ModrinthVersion): boolean {
		return version.game_versions?.some(
			(gv) => getGameVersion(gv) === gameVersion,
		);
	}

	function isVersionCompatibleCurseForge(file: CurseForgeFile): boolean {
		return file.gameVersions.some((gv) => {
			const clean = getGameVersion(gv);
			return clean === gameVersion;
		});
	}

	function isModCompatible(
		project: ModrinthProject | CurseForgeProject,
	): boolean {
		if ("versions" in project) {
			return project.versions.some((v) => v === gameVersion);
		}
		if ("latestFilesIndexes" in project) {
			return (
				project.latestFilesIndexes?.some(
					(idx) => getGameVersion(idx.gameVersion) === gameVersion,
				) ?? false
			);
		}
		return true;
	}

	async function loadVersions(projectId: string) {
		loadingVersions = true;
		selectedModVersions = [];
		selectedVersionId = "";
		try {
			if (source === "modrinth") {
				const versions = await getModrinthProjectVersions(
					projectId,
					instance.loader,
				);
				const sorted = [...versions].sort((a, b) => {
					const aCompat = isVersionCompatibleModrinth(a) ? 1 : 0;
					const bCompat = isVersionCompatibleModrinth(b) ? 1 : 0;
					return bCompat - aCompat;
				});
				selectedModVersions = sorted;
				if (sorted.length > 0) {
					const stored = versionSelection.get(projectId);
					if (stored && sorted.find((v) => v.id === stored)) {
						selectedVersionId = stored;
					} else {
						const compatible = sorted.find((v) =>
							isVersionCompatibleModrinth(v),
						);
						if (compatible) {
							selectedVersionId = compatible.id;
							versionSelection.set(projectId, compatible.id);
						}
					}
				}
			} else {
				const modId = Number(projectId);
				const files = await getCurseForgeProjectFiles(
					modId,
					instance.loader,
				);
				const cfFiles = [...files].sort((a, b) => {
					const aCompat = isVersionCompatibleCurseForge(a) ? 1 : 0;
					const bCompat = isVersionCompatibleCurseForge(b) ? 1 : 0;
					return bCompat - aCompat;
				});
				const prioritized = [
					...cfFiles.filter((f) => f.releaseType === 1),
					...cfFiles.filter((f) => f.releaseType === 2),
					...cfFiles.filter((f) => f.releaseType === 3),
				];
				selectedModVersions =
					prioritized.length > 0 ? prioritized : cfFiles;
				if (selectedModVersions.length > 0) {
					const stored = versionSelection.get(projectId);
					const first = selectedModVersions[0] as CurseForgeFile;
					const storedMatch = stored
						? selectedModVersions.find((f) =>
								"id" in f ? f.id.toString() === stored : false,
							)
						: undefined;
					if (storedMatch && stored) {
						selectedVersionId = stored;
					} else {
						const compatible =
							selectedModVersions.find(
								(f) =>
									isCurseForgeFile(f) &&
									isVersionCompatibleCurseForge(f),
							) || first;
						selectedVersionId = compatible.id.toString();
						versionSelection.set(
							projectId,
							compatible.id.toString(),
						);
					}
				}
			}
		} finally {
			loadingVersions = false;
		}
	}

	function onVersionChange() {
		if (selectedVersionId && selectedMod) {
			versionSelection.set(getProjectId(selectedMod), selectedVersionId);
		}
	}

	const versionDropdownOptions = $derived(
		selectedModVersions.map((v) => {
			if (isCurseForgeFile(v)) {
				const compatible = isVersionCompatibleCurseForge(v);
				const subtitle = compatible
					? t("instanceView.downloadMods.compatible")
					: v.gameVersions.slice(0, 2).join(", ");

				let extra = "";
				if (v.releaseType === 1) extra = "release";
				else if (v.releaseType === 2) extra = "beta";
				else if (v.releaseType === 3) extra = "alpha";

				return {
					value: v.id.toString(),
					label: extra ? `${v.fileName} [${extra}]` : v.fileName,
					subtitle,
				};
			} else {
				const compatible = isVersionCompatibleModrinth(v);
				const subtitle = compatible
					? t("instanceView.downloadMods.compatible")
					: v.game_versions?.slice(0, 2).join(", ");

				return {
					value: v.id,
					label: v.version_number,
					subtitle,
				};
			}
		}),
	);

	$effect(() => {
		if (selectedMod && !reviewing) {
			loadVersions(getProjectId(selectedMod));
		}
	});

</script>

<div class="dm-root">
	{#if reviewing}
		<div class="dm-review">
			<div class="dm-review-header">
				<div>
					<span class="dm-section-label"
						>{t("instanceView.downloadMods.sectionLabel")}</span
					>
					<h2 class="dm-review-title">
						{t("instanceView.downloadMods.reviewTitle")}
					</h2>
				</div>
				<button
					type="button"
					class="dm-back-btn"
					onclick={() => (reviewing = false)}
					disabled={downloading}
				>
					{t("instanceView.downloadMods.back")}
				</button>
			</div>

			<div class="dm-review-body">
				{#if resolvingDeps}
					<div class="dm-center-state">
						<Loading />
						<p>{t("instanceView.downloadMods.resolvingDeps")}</p>
					</div>
				{:else if downloadQueue.length === 0}
					<div class="dm-center-state">
						<p>{t("instanceView.downloadMods.allInstalled")}</p>
						<span style="font-size:0.75rem; opacity:0.5;"
							>{t(
								"instanceView.downloadMods.allInstalledSub",
							)}</span
						>
					</div>
				{:else}
					<div class="dm-queue-box">
						<p class="dm-queue-subtitle">
							{downloadQueue.length}
							{downloadQueue.length === 1
								? t("instanceView.downloadMods.file_one")
								: t("instanceView.downloadMods.file_other")} para
							descargar:
						</p>
						<div class="dm-queue-list">
							{#each downloadQueue as item (item.filename)}
								<div class="dm-queue-item">
									{#if item.iconUrl}
										<img
											src={item.iconUrl}
											alt=""
											class="dm-queue-icon-img"
										/>
									{:else}
										<span class="dm-queue-icon">📦</span>
									{/if}
									<div class="dm-queue-item-info">
										{#if item.projectTitle}
											<span class="dm-queue-title"
												>{item.projectTitle}</span
											>
										{/if}
										<span class="dm-queue-filename"
											>{item.filename}</span
										>
									</div>
								</div>
							{/each}
						</div>
					</div>

					<div class="dm-review-footer">
						<span class="dm-review-count">
							<strong>{downloadQueue.length}</strong>
							{downloadQueue.length !== 1
								? t("instanceView.downloadMods.file_other")
								: t("instanceView.downloadMods.file_one")}
						</span>
						<button
							type="button"
							class="dm-primary-btn"
							onclick={confirmDownload}
							disabled={downloading}
						>
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
	{:else}
		<div class="dm-layout">
			<main class="dm-main">
				<div class="dm-search-bar-wrap">
					<div class="dm-source-tabs">
						<button
							type="button"
							class="dm-source-tab {source === 'modrinth'
								? 'active'
								: ''}"
							onclick={() => switchSource("modrinth")}
						>
							{t("instanceView.downloadMods.sourceModrinth")}
						</button>
						<button
							type="button"
							class="dm-source-tab {source === 'curseforge'
								? 'active'
								: ''}"
							onclick={() => switchSource("curseforge")}
						>
							{t("instanceView.downloadMods.sourceCurseForge")}
						</button>
					</div>
					<span class="dm-search-icon">
						<svg
							width="15"
							height="15"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<circle cx="11" cy="11" r="8" /><path
								d="m21 21-4.35-4.35"
							/>
						</svg>
					</span>
					<input
						class="dm-search-input"
						type="text"
						placeholder={t(
							"instanceView.downloadMods.searchPlaceholder",
						)}
						oninput={(e) => onSearchInput(e.currentTarget.value)}
						onkeydown={(e) =>
							e.key === "Enter" && performSearch(true)}
					/>
					{#if query}
						<button
							type="button"
							class="dm-search-clear"
							onclick={() => {
								query = "";
								performSearch(true);
							}}>×</button
						>
					{/if}
				</div>

				{#if totalHits > 0 && !searching}
					<div class="dm-results-meta">
						<span
							>{t(
								"instanceView.downloadMods.resultsFound",
							).replace(
								"{count}",
								totalHits.toLocaleString(),
							)}</span
						>
					</div>
				{/if}

				<div class="dm-results-area">
					{#if searching}
						<div class="dm-center-state">
							<Loading />
							<p>{t("instanceView.downloadMods.searching")}</p>
						</div>
					{:else if allHits.length > 0}
						<div class="dm-vlist-wrap">
							<VirtualList
								items={allHits}
								itemHeight={130}
								onNearEnd={handleNearEnd}
							>
								{#snippet children(project)}
									<!-- svelte-ignore a11y_click_events_have_key_events -->
									<!-- svelte-ignore a11y_no_static_element_interactions -->
									<div
										class="dm-mod-card-v {selectedMod &&
										getProjectId(selectedMod) ===
											getProjectId(project)
											? 'selected'
											: ''}"
										onclick={() => (selectedMod = project)}
									>
										<div class="dm-mod-icon-v">
											{#if getProjectIcon(project)}
												<img
													src={getProjectIcon(
														project,
													)!}
													alt={getProjectTitle(
														project,
													)}
													loading="lazy"
												/>
											{:else}
												<span
													class="dm-mod-icon-placeholder"
													>📦</span
												>
											{/if}
										</div>
										<div class="dm-mod-body-v">
											<div class="dm-mod-top-v">
												<h4
													class="dm-mod-title-v"
													title={getProjectTitle(
														project,
													)}
												>
													{getProjectTitle(project)}
												</h4>
												<div class="dm-mod-badges-v">
													{#if isModInstalled(project)}
														<span
															class="dm-installed-badge"
															>{t(
																"instanceView.downloadMods.installed",
															)}</span
														>
													{/if}
													{#if !isModCompatible(project)}
														<span
															class="dm-incompat-badge"
															title={t(
																"instanceView.downloadMods.noCompatibleVersions",
															).replace(
																"{version}",
																gameVersion,
															)}
															>{t(
																"instanceView.downloadMods.noVersionCompat",
															).replace(
																"{version}",
																gameVersion,
															)}</span
														>
													{/if}
												</div>
											</div>
											<span class="dm-mod-author-v">
												{t(
													"instanceView.downloadMods.by",
												)}
												{getProjectAuthor(project)}
											</span>
											<p class="dm-mod-desc-v">
												{getProjectDescription(project)}
											</p>
										</div>
										<div class="dm-mod-actions-v">
											<span class="dm-mod-stat"
												>↓ {formatNumber(
													getProjectDownloads(
														project,
													),
												)}</span
											>
											<button
												type="button"
												class="dm-select-btn {basket.has(
													getProjectId(project),
												)
													? 'selected'
													: ''}"
												onclick={(e) => {
													e.stopPropagation();
													toggleBasket(project);
												}}
											>
												{basket.has(
													getProjectId(project),
												)
													? t(
															"instanceView.downloadMods.selected",
														)
													: t(
															"instanceView.downloadMods.select",
														)}
											</button>
										</div>
									</div>
								{/snippet}
							</VirtualList>
							{#if loadingMore}
								<div class="dm-vlist-loading">
									<Loading class="dm-spinning" />
									<span
										>{t(
											"instanceView.downloadMods.loadingMore",
										)}</span
									>
								</div>
							{:else if allHits.length >= totalHits && totalHits > 0}
								<div class="dm-vlist-end">
									<span class="dm-end-label"
										>— {t(
											"instanceView.downloadMods.endOfResults",
										).replace(
											"{count}",
											allHits.length.toString(),
										)} —</span
									>
								</div>
							{/if}
						</div>
					{:else}
						<div class="dm-center-state">
							<p>{t("instanceView.downloadMods.noResults")}</p>
							<button
								type="button"
								class="dm-ghost-btn"
								onclick={() => {
									query = "";
									activeCategory = null;
									performSearch(true);
								}}
							>
								{t("instanceView.downloadMods.clearFilters")}
							</button>
						</div>
					{/if}
				</div>
			</main>

			{#if selectedMod}
				<aside class="dm-details">
					<button
						type="button"
						class="dm-close-btn"
						aria-label={t("instanceView.downloadMods.closeDetails")}
						onclick={() => (selectedMod = null)}
					>
						<svg
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2.5"
						>
							<path d="M18 6 6 18M6 6l12 12" />
						</svg>
					</button>
					<div class="dm-details-scroll">
						<div class="dm-details-icon">
							{#if getProjectIcon(selectedMod)}
								<img
									src={getProjectIcon(selectedMod)!}
									alt={getProjectTitle(selectedMod)}
								/>
							{:else}
								<span>📦</span>
							{/if}
						</div>
						<h3 class="dm-details-title">
							{getProjectTitle(selectedMod)}
						</h3>
						<p class="dm-details-author">
							{t("instanceView.downloadMods.by")}
							{getProjectAuthor(selectedMod)}
						</p>

						<div class="dm-details-stat-row">
							<div class="dm-details-stat">
								<span class="dm-details-stat-label"
									>{t(
										"instanceView.downloadMods.downloads",
									)}</span
								>
								<span class="dm-details-stat-value"
									>{formatNumber(
										getProjectDownloads(selectedMod),
									)}</span
								>
							</div>
						</div>

						<div class="dm-tags">
							{#each getProjectCategories(selectedMod).slice(0, 4) as cat (cat)}
								<span class="dm-tag">{cat}</span>
							{/each}
						</div>

						<div class="dm-details-version-row">
							<span class="dm-details-version-label"
								>{t(
									"instanceView.downloadMods.versionLabel",
								)}</span
							>
							{#if loadingVersions}
								<span class="dm-loading-versions"
									>{t(
										"instanceView.downloadMods.loadingVersions",
									)}</span
								>
							{:else if selectedModVersions.length === 0}
								<span class="dm-no-versions-msg"
									>{t(
										"instanceView.downloadMods.noCompatibleVersions",
									).replace("{version}", gameVersion)}</span
								>
								<span class="dm-loading-versions"
									>{t(
										"instanceView.downloadMods.noCompatibleDesc",
									)}</span
								>
							{:else}
								<Dropdown
									bind:value={selectedVersionId}
									options={versionDropdownOptions}
									placeholder={t(
										"instanceView.downloadMods.anyVersion",
									)}
									onchange={onVersionChange}
								/>
							{/if}
						</div>

						<p class="dm-details-desc">
							{getProjectDescription(selectedMod)}
						</p>

						<button
							type="button"
							class="dm-primary-btn dm-full-width {basket.has(
								getProjectId(selectedMod),
							)
								? 'dm-btn-remove'
								: ''}"
							onclick={() => toggleBasket(selectedMod!)}
						>
							{basket.has(getProjectId(selectedMod))
								? t("instanceView.downloadMods.removeSelection")
								: t(
										"instanceView.downloadMods.selectToDownload",
									)}
						</button>
					</div>
				</aside>
			{/if}
		</div>

		{#if basket.size > 0}
			<div class="dm-bottom-bar">
				<span class="dm-bottom-count">
					{t("instanceView.downloadMods.selectionLabel")}: {basket.size}
				</span>
				<button
					type="button"
					class="dm-primary-btn"
					onclick={startReview}
				>
					{t("instanceView.downloadMods.reviewBtn")}
				</button>
			</div>
		{/if}
	{/if}
</div>

<style>
	.dm-root {
		display: flex;
		flex-direction: column;
		height: calc(100% + 64px);
		margin: -32px -40px;
		background: var(--bg-main);
		color: var(--text-primary);

		overflow: hidden;
	}

	.dm-layout {
		display: flex;
		flex: 1;
		overflow: hidden;
	}

	.dm-section-label {
		font-size: 0.62rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 1.5px;
		color: var(--text-secondary);
		margin-bottom: 8px;
		display: block;
	}

	.dm-full-width {
		width: 100%;
	}

	.dm-main {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.dm-search-bar-wrap {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 16px 20px;
		background: var(--bg-sidebar);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.dm-source-tabs {
		display: flex;
		gap: 2px;
		background: rgba(255, 255, 255, 0.04);
		border-radius: var(--border-radius-sm);
		padding: 2px;
		flex-shrink: 0;
	}

	.dm-source-tab {
		padding: 4px 12px;
		background: transparent;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 0.72rem;
		font-weight: 600;

		border-radius: calc(var(--border-radius-sm) - 1px);
		transition: all 0.15s;
		white-space: nowrap;
	}

	.dm-source-tab:hover {
		color: var(--text-primary);
		background: rgba(255, 255, 255, 0.04);
	}

	.dm-source-tab.active {
		background: var(--accent);
		color: var(--bg-main);
	}

	.dm-search-icon {
		color: var(--text-secondary);
		display: flex;
		align-items: center;
		flex-shrink: 0;
	}
	.dm-search-input {
		flex: 1;
		background: transparent;
		border: none;
		color: var(--text-primary);
		font-size: 0.9rem;
		outline: none;
	}
	.dm-search-input::placeholder {
		color: var(--text-secondary);
	}
	.dm-search-clear {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 1.2rem;
		line-height: 1;
		transition: color 0.2s;
	}
	.dm-search-clear:hover {
		color: var(--text-primary);
	}

	.dm-results-meta {
		padding: 8px 20px;
		font-size: 0.72rem;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 1px;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.dm-bottom-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 24px;
		background: var(--bg-sidebar);
		border-top: 1px solid var(--border);
		flex-shrink: 0;
	}

	.dm-bottom-count {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.dm-results-area {
		flex: 1;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.dm-vlist-wrap {
		flex: 1;
		overflow: hidden;
		display: flex;
		flex-direction: column;
		padding: 4px 20px 16px;
	}
	.dm-vlist-loading,
	.dm-vlist-end {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 16px 0 8px;
		color: var(--text-secondary);
		font-size: 0.78rem;
		flex-shrink: 0;
	}
	.dm-end-label {
		font-size: 0.7rem;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 1px;
	}

	.dm-mod-card-v {
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

	.dm-mod-card-v:hover {
		background: rgba(255, 255, 255, 0.04);
		border-color: rgba(255, 255, 255, 0.1);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
	}
	.dm-mod-card-v.selected {
		border-color: var(--accent);
		background: rgba(255, 255, 255, 0.04);
	}

	.dm-mod-icon-v {
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

	.dm-mod-icon-v img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		image-rendering: pixelated;
	}

	.dm-mod-body-v {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: 4px;
		overflow: hidden;
	}
	.dm-mod-top-v {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}
	.dm-mod-title-v {
		font-size: 0.85rem;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}
	.dm-mod-badges-v {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-shrink: 0;
	}
	.dm-mod-author-v {
		font-size: 0.7rem;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.dm-mod-desc-v {
		font-size: 0.75rem;
		color: var(--text-secondary);
		line-height: 1.35;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}

	.dm-mod-actions-v {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		justify-content: center;
		gap: 6px;
		flex-shrink: 0;
	}
	.dm-mod-stat {
		font-size: 0.72rem;
		color: var(--text-secondary);
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid var(--border);
		padding: 2px 7px;
		border-radius: var(--border-radius-sm);
		white-space: nowrap;
	}

	.dm-select-btn {
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

	.dm-select-btn:hover {
		background: rgba(255, 255, 255, 0.06);
		color: var(--text-primary);
		border-color: rgba(255, 255, 255, 0.2);
	}
	.dm-select-btn.selected {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--bg-main);
	}

	.dm-installed-badge {
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

	:global(.dm-spinning) {
		animation: spin 0.8s linear infinite;
		will-change: transform;
	}
	:global {
		@keyframes spin {
			to {
				transform: rotate(360deg);
			}
		}
	}

	.dm-details {
		width: 280px;
		flex-shrink: 0;
		background-color: var(--bg-sidebar);
		border-left: 1px solid var(--border);
		position: relative;
		animation: dm-slide-in 0.25s cubic-bezier(0.2, 0.8, 0.2, 1);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	@keyframes dm-slide-in {
		from {
			transform: translateX(30px);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}

	.dm-close-btn {
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

	.dm-close-btn:hover {
		background: rgba(255, 255, 255, 0.1);
		color: var(--text-primary);
	}

	.dm-details-scroll {
		padding: 20px 16px;
		overflow-y: auto;
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.dm-details-icon {
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

	.dm-details-icon img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		image-rendering: pixelated;
	}
	.dm-details-title {
		font-size: 1rem;
		font-weight: 700;
		color: var(--text-primary);
		text-align: center;
		margin: 0;
	}
	.dm-details-author {
		font-size: 0.75rem;
		color: var(--text-secondary);
		text-align: center;
		margin: 0;
	}
	.dm-details-stat-row {
		display: flex;
		gap: 8px;
	}
	.dm-details-stat {
		flex: 1;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 8px 10px;
		text-align: center;
	}
	.dm-details-stat-label {
		display: block;
		font-size: 0.62rem;
		text-transform: uppercase;
		letter-spacing: 1px;
		color: var(--text-secondary);
		margin-bottom: 4px;
	}
	.dm-details-stat-value {
		font-size: 0.95rem;
		font-weight: 700;
		color: var(--text-primary);
	}
	.dm-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.dm-tag {
		font-size: 0.68rem;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid var(--border);
		padding: 2px 8px;
		border-radius: 20px;
		color: var(--text-secondary);
		text-transform: capitalize;
	}
	.dm-details-desc {
		font-size: 0.8rem;
		line-height: 1.55;
		color: var(--text-secondary);
		margin: 0;
	}
	.dm-details-version-row {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.dm-details-version-label {
		font-size: 0.68rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 1px;
		color: var(--text-secondary);
	}
	.dm-no-versions-msg {
		font-size: 0.75rem;
		color: #f87171;
		text-align: center;
		padding: 8px;
		border: 1px solid rgba(248, 113, 113, 0.2);
		border-radius: var(--border-radius-sm);
		background: rgba(248, 113, 113, 0.06);
	}
	.dm-loading-versions {
		font-size: 0.72rem;
		color: var(--text-secondary);
		text-align: center;
	}
	.dm-incompat-badge {
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
	.dm-queue-icon-img {
		width: 24px;
		height: 24px;
		border-radius: var(--border-radius-sm);
		object-fit: cover;
		flex-shrink: 0;
	}

	.dm-review {
		display: flex;
		flex-direction: column;
		height: 100%;
		padding: 28px 32px;
	}
	.dm-review-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 24px;
		padding-bottom: 16px;
		border-bottom: 1px solid var(--border);
	}
	.dm-review-title {
		font-size: 1.3rem;
		font-weight: 700;
		color: var(--text-primary);
		margin: 0;
	}
	.dm-review-body {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.dm-queue-box {
		flex: 1;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 16px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.dm-queue-subtitle {
		font-size: 0.78rem;
		color: var(--text-secondary);
		margin: 0 0 14px 0;
	}
	.dm-queue-list {
		flex: 1;
		overflow-y: auto;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 8px;
		align-content: flex-start;
	}
	.dm-queue-item {
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		padding: 10px 12px;
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.dm-queue-item-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
		gap: 2px;
	}
	.dm-queue-title {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.dm-queue-icon {
		font-size: 1rem;
		opacity: 0.6;
	}
	.dm-queue-filename {
		font-size: 0.8rem;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.dm-review-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 16px;
		padding: 14px 16px;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
	}
	.dm-review-count {
		font-size: 0.85rem;
		color: var(--text-secondary);
	}
	.dm-review-count strong {
		color: var(--text-primary);
		font-size: 1.1rem;
	}

	.dm-primary-btn {
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
	.dm-primary-btn:hover:not(:disabled) {
		filter: brightness(0.9);
	}
	.dm-primary-btn:disabled {
		opacity: 0.35;
		cursor: not-allowed;
	}
	.dm-primary-btn.dm-btn-remove {
		background: rgba(255, 255, 255, 0.06);
		color: var(--text-primary);
		border: 1px solid var(--border);
	}
	.dm-primary-btn.dm-btn-remove:hover:not(:disabled) {
		background: rgba(255, 68, 68, 0.12);
		color: #ff6b6b;
		border-color: rgba(255, 68, 68, 0.3);
	}

	.dm-back-btn {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 7px 14px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.82rem;

		transition: all 0.15s;
	}
	.dm-back-btn:hover:not(:disabled) {
		background: var(--bg-item-active);
		color: var(--text-primary);
	}
	.dm-back-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.dm-ghost-btn {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 7px 16px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.8rem;

		transition: all 0.15s;
	}
	.dm-ghost-btn:hover {
		background: var(--bg-item-active);
		color: var(--text-primary);
	}

	.dm-center-state {
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

	.dm-mod-icon-placeholder {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 1.2rem;
		color: var(--text-secondary);
		background: rgba(255, 255, 255, 0.05);
	}

	@media (max-width: 1200px) {
		.dm-details {
			width: 260px;
		}
	}

	@media (max-width: 950px) {
		.dm-details {
			position: fixed;
			right: 0;
			top: 0;
			bottom: 0;
			z-index: 100;
			width: 320px;
			box-shadow: -10px 0 30px rgba(0, 0, 0, 0.5);
			animation: dm-slide-in 0.25s cubic-bezier(0.2, 0.8, 0.2, 1);
		}
	}

	@media (max-width: 700px) {
		.dm-main .dm-search-bar-wrap {
			padding: 12px 16px;
		}
		.dm-results-meta {
			padding: 6px 16px;
		}
		.dm-vlist-wrap {
			padding: 4px 16px 12px;
		}
		.dm-bottom-bar {
			padding: 10px 16px;
		}
	}

	@media (max-width: 550px) {
		.dm-details {
			width: 100%;
		}
		.dm-vlist-wrap {
			padding: 4px 12px 8px;
		}
		.dm-mod-card-v {
			padding: 10px 12px;
			gap: 10px;
		}
		.dm-mod-icon-v {
			width: 36px;
			height: 36px;
		}
		.dm-mod-title-v {
			font-size: 0.85rem;
		}
	}
</style>
