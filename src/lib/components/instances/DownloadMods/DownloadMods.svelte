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
	import { SvelteMap } from "svelte/reactivity";
	import Review from "./Review.svelte";
	import Browse from "./Browse.svelte";

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

	let basket = $state(
		new SvelteMap<string, ModrinthProject | CurseForgeProject>(),
	);

	let selectedMod = $state<ModrinthProject | CurseForgeProject | null>(null);

	let reviewing = $state(false);
	let resolvingDeps = $state(false);
	let downloading = $state(false);
	let downloadQueue = $state<ModDownloadInfo[]>([]);

	let selectedModVersions = $state<(ModrinthVersion | CurseForgeFile)[]>([]);
	let selectedVersionId = $state<string>("");
	let loadingVersions = $state(false);
	let versionSelection = $state(new SvelteMap<string, string>());

	let installedModNames = $state<Set<string>>(new Set());

	function getGameVersion(versionStr: string): string {
		const lower = versionStr.toLowerCase();
		if (
			lower.includes("-forge-") ||
			lower.includes("-neoforge-") ||
			lower.includes("-quilt-")
		) {
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

	function isModrinthProject(
		p: ModrinthProject | CurseForgeProject,
	): p is ModrinthProject {
		return "project_id" in p;
	}

	function isCurseForgeFile(
		v: ModrinthVersion | CurseForgeFile,
	): v is CurseForgeFile {
		return "fileName" in v;
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
		basket.clear();
		versionSelection.clear();
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
		basket.clear();
		selectedMod = null;
		reviewing = false;
		resolvingDeps = false;
		downloading = false;
		downloadQueue = [];
		selectedModVersions = [];
		selectedVersionId = "";
		loadingVersions = false;
		versionSelection.clear();
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
				if (isModrinthProject(project)) {
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
					? "✔ Compatible"
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
					? "✔ Compatible"
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
		<Review
			{resolvingDeps}
			{downloading}
			{downloadQueue}
			onBack={() => (reviewing = false)}
			onConfirmDownload={confirmDownload}
		/>
	{:else}
		<Browse
			bind:source
			bind:query
			{allHits}
			{totalHits}
			{searching}
			{loadingMore}
			{basket}
			bind:selectedMod
			{selectedModVersions}
			bind:selectedVersionId
			{loadingVersions}
			{versionDropdownOptions}
			{gameVersion}
			{installedModNames}
			{onSearchInput}
			{performSearch}
			{handleNearEnd}
			{switchSource}
			{toggleBasket}
			{onVersionChange}
			{startReview}
		/>
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
</style>
