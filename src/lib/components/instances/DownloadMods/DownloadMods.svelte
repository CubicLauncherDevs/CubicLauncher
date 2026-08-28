<script lang="ts">
	import {
		searchModrinth,
		getModrinthProjectVersions,
		searchCurseForge,
		getCurseForgeProjectFiles,
		downloadMods,
		getInstanceMods,
		resolveModDependencies,
		type ModDownloadInfo,
	} from "$lib/api/cubicApi";
	import {
		type DependencyRequest,
		type ResolvedDependency,
		type DependencyResolutionResult,
	} from "$lib/types/dependency";
	import type {
		ModrinthProject,
		ModrinthVersion,
		CurseForgeProject,
		CurseForgeFile,
		InstanceDto,
		ModDto,
		ModSource,
	} from "$lib/types/types";
	import { onDestroy } from "svelte";
	import { SvelteMap } from "svelte/reactivity";
	import Review from "./Review.svelte";
	import Browse from "./Browse.svelte";
	import ModDependenciesModal from "./ModDependenciesModal.svelte";

	onDestroy(() => {
		clearTimeout(debounceTimer);
	});

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
	let dependencyResult = $state<DependencyResolutionResult | null>(null);

	let selectedModVersions = $state<(ModrinthVersion | CurseForgeFile)[]>([]);
	let selectedVersionId = $state<string>("");
	let loadingVersions = $state(false);
	let versionSelection = new SvelteMap<string, string>();

	let installedModNames = $state<Set<string>>(new Set());
	let installedMods = $state<ModDto[]>([]);
	let dependenciesModalOpen = $state(false);
	let dependencyPreviewRequest = $state<DependencyRequest | null>(null);
	let dependencyPreviewTitle = $state<string>("");

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
		if (lower.startsWith("quilt-loader-")) {
			const lastDash = versionStr.lastIndexOf("-");
			if (lastDash !== -1) return versionStr.slice(lastDash + 1);
		}
		return versionStr;
	}

	const gameVersion = $derived(getGameVersion(instance.version));

	let searchGen = 0;
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
		const gen = ++searchGen;

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
				);
				if (gen !== searchGen) return;
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
				);
				if (gen !== searchGen) return;
				if (result) {
					totalHits = result.pagination.totalCount;
					allHits = resetResults
						? result.data
						: [...allHits, ...result.data];
					currentOffset = allHits.length;
				}
			}
		} finally {
			searching = false;
			loadingMore = false;
		}
	}

	function openDependenciesModal() {
		if (!selectedMod) return;
		dependencyPreviewTitle =
			"title" in selectedMod ? selectedMod.title : selectedMod.name;
		dependencyPreviewRequest = {
			source: isModrinthProject(selectedMod) ? "modrinth" : "curseforge",
			project_id: getProjectId(selectedMod),
			version_id: selectedVersionId || null,
			kind: "required",
		};
		dependenciesModalOpen = true;
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
		dependencyResult = null;
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
			installedMods = mods;
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

	function buildDependencyRequests(): DependencyRequest[] {
		const requests: DependencyRequest[] = [];
		for (const [id, project] of basket.entries()) {
			requests.push({
				source: isModrinthProject(project) ? "modrinth" : "curseforge",
				project_id: id,
				version_id: versionSelection.get(id) ?? null,
				kind: "required",
			});
		}
		return requests;
	}

	function flattenDependencies(
		deps: ResolvedDependency[],
		installedProjectIds: Set<string>,
		installedSlugs: Set<string>,
		installedFilenames: Set<string>,
		queuedFilenames: Set<string>,
		result: ModDownloadInfo[] = [],
	): ModDownloadInfo[] {
		for (const dep of deps) {
			if (dep.kind === "incompatible" || dep.kind === "optional") {
				continue;
			}
			if (
				installedProjectIds.has(dep.project_id) ||
				(installedSlugs.size > 0 && installedSlugs.has(dep.project_id))
			) {
				continue;
			}
			if (!dep.download_url || !dep.filename) {
				continue;
			}
			const filenameLower = dep.filename.toLowerCase();
			if (
				installedFilenames.has(filenameLower) ||
				queuedFilenames.has(filenameLower)
			) {
				continue;
			}
			queuedFilenames.add(filenameLower);
			result.push({
				url: dep.download_url,
				filename: dep.filename,
				projectTitle: dep.title,
				iconUrl: dep.icon_url ?? undefined,
			});
			flattenDependencies(
				dep.children,
				installedProjectIds,
				installedSlugs,
				installedFilenames,
				queuedFilenames,
				result,
			);
		}
		return result;
	}

	async function startReview() {
		reviewing = true;
		resolvingDeps = true;
		downloadQueue = [];
		dependencyResult = null;

		try {
			const mods = await getInstanceMods(instance.uuid);
			installedMods = mods;
			const installedProjectIds = new Set(
				mods
					.map((m) => m.project_id)
					.filter((id): id is string => !!id),
			);
			const installedSlugs = new Set(
				mods
					.map((m) => m.slug)
					.filter((slug): slug is string => !!slug),
			);
			const installedFilenames = new Set(
				mods.map((m) => m.filename.toLowerCase()),
			);

			const requests = buildDependencyRequests();
			const result = await resolveModDependencies(
				requests,
				instance.loader,
				gameVersion,
			);
			dependencyResult = result;

			if (result.conflicts.length > 0) {
				console.warn(
					"[DownloadMods] Conflictos de dependencias detectados:",
					result.conflicts,
				);
			}

			const queuedFilenames = new Set<string>();
			downloadQueue = flattenDependencies(
				result.tree,
				installedProjectIds,
				installedSlugs,
				installedFilenames,
				queuedFilenames,
			);
		} finally {
			resolvingDeps = false;
		}
	}

	function handleQueueChange(queue: ModDownloadInfo[]) {
		downloadQueue = queue;
	}

	async function confirmDownload() {
		downloading = true;
		try {
			await downloadMods(instance.uuid, downloadQueue);
			basket.clear();
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
					gameVersion,
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
					gameVersion,
				);
				const cfFiles = [...files].sort((a, b) => {
					const aCompat = isVersionCompatibleCurseForge(a) ? 1 : 0;
					const bCompat = isVersionCompatibleCurseForge(b) ? 1 : 0;
					if (bCompat !== aCompat) return bCompat - aCompat;
					const rank = [1, 2, 3];
					const aRank = rank.indexOf(a.releaseType);
					const bRank = rank.indexOf(b.releaseType);
					if (aRank !== -1 && bRank !== -1) return aRank - bRank;
					if (aRank !== -1) return -1;
					if (bRank !== -1) return 1;
					return a.releaseType - b.releaseType;
				});
				selectedModVersions = cfFiles;
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
			dependencyTree={dependencyResult?.tree ?? []}
			conflicts={dependencyResult?.conflicts ?? []}
			installedProjectIds={new Set(
				installedMods
					.map((m) => m.project_id)
					.filter((id): id is string => !!id),
			)}
			onBack={() => (reviewing = false)}
			onConfirmDownload={confirmDownload}
			onQueueChange={handleQueueChange}
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
			onViewDependencies={openDependenciesModal}
		/>
	{/if}
</div>

<ModDependenciesModal
	bind:open={dependenciesModalOpen}
	request={dependencyPreviewRequest}
	loader={instance.loader}
	{gameVersion}
	projectTitle={dependencyPreviewTitle}
	installedProjectIds={new Set(
		installedMods
			.map((m) => m.project_id)
			.filter((id): id is string => !!id),
	)}
/>

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
