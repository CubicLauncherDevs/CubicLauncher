import { SvelteMap, SvelteSet } from "svelte/reactivity";
import {
	deleteInstanceFile,
	getInstanceMods,
	getInstanceResourcePacks,
	getInstanceShaderPacks,
	getModrinthProject,
	getModrinthProjectVersions,
	searchModrinth,
	searchCurseForge,
	getCurseForgeProject,
	getCurseForgeProjectFiles,
	getCurseForgeProjectDescription,
	toggleInstanceMod,
	downloadMods,
	downloadResourcePacks,
	downloadShaderPacks,
	resolveModDependencies,
	type ModDownloadInfo,
} from "$lib/api/cubicApi";
import { registerModsRefreshCallback } from "$lib/api/launcherService";
import {
	localModToMarket,
	modrinthProjectToMarket,
	modrinthVersionToMarket,
	curseforgeProjectToMarket,
	curseforgeVersionToMarket,
	parseInstanceVersion,
	type MarketProject,
	type MarketVersion,
	type ContentType,
} from "$lib/types/market";
import type {
	InstanceDto,
	ModDto,
	ModrinthProjectFull,
	CurseForgeProject,
} from "$lib/types/types";
import { InstState } from "$lib/types/types";
import type {
	DependencyRequest,
	DependencyResolutionResult,
} from "$lib/types/dependency";
import { showWarning } from "$lib/state/state.svelte";
import { t } from "$lib/i18n";

const PAGE_SIZE = 20;

export type MarketSource = "local" | "modrinth" | "curseforge";

export type MarketSort = "relevance" | "downloads" | "newest";
export type LocalSort = "name-asc" | "name-desc";
export type LocalSourceFilter = "all" | "modrinth" | "curseforge" | "local";

const CURSEFORGE_CATEGORY_IDS: Record<string, number> = {
	adventure: 422,
	magic: 419,
	utility: 5191,
	optimization: 6814,
	equipment: 434,
	worldgen: 406,
	food: 436,
	library: 421,
	decoration: 424,
	storage: 420,
};

export interface MarketFilters {
	source: MarketSource;
	query: string;
	loader: string;
	gameVersion: string;
	category: string | null;
	sort: MarketSort;
	localSort: LocalSort;
	localSource: LocalSourceFilter;
}

export interface MarketDetailState {
	fullProject?: ModrinthProjectFull | CurseForgeProject;
	curseforgeDescription?: string;
	versions: MarketVersion[];
	loading: boolean;
	error: string | null;
}

export function createMarketState(
	instance: InstanceDto,
	contentType: ContentType = "mods",
) {
	const parsed = parseInstanceVersion(instance);

	const isModContent = contentType === "mods";
	const localLoader = isModContent
		? getInstanceMods
		: contentType === "resourcepacks"
			? getInstanceResourcePacks
			: getInstanceShaderPacks;
	const downloadFn = isModContent
		? downloadMods
		: contentType === "resourcepacks"
			? downloadResourcePacks
			: downloadShaderPacks;
	const subDir = isModContent
		? "mods"
		: contentType === "resourcepacks"
			? "resourcepacks"
			: "shaderpacks";
	const projectType = isModContent
		? "mod"
		: contentType === "resourcepacks"
			? "resourcepack"
			: "shader";

	const filters = $state<MarketFilters>({
		source: "modrinth",
		query: "",
		loader: parsed.loader.toLowerCase(),
		gameVersion: parsed.gameVersion,
		category: null,
		sort: "downloads",
		localSort: "name-asc",
		localSource: "all",
	});

	const items = $state<MarketProject[]>([]);
	let total = $state(0);
	let loadingLocal = $state(false);
	let loadingRemote = $state(false);
	let loadingMore = $state(false);
	let error = $state<string | null>(null);
	let offset = $state(0);
	let hasMore = $state(true);
	const localModsById = new SvelteMap<string, ModDto>();
	let rawLocalItems: MarketProject[] = [];
	let selectedId = $state<string | null>(null);
	const detail = $state<MarketDetailState>({
		versions: [],
		loading: false,
		error: null,
	});

	let overrideVersionId = $state<string | null>(null);
	let searchGen = 0;
	let localSearchGen = 0;
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	const selectedProject = $derived<MarketProject | null>(
		items.find((i) => i.id === selectedId) ?? null,
	);

	const selectedVersion = $derived.by<MarketVersion | null>(() => {
		if (detail.versions.length === 0) return null;

		if (overrideVersionId) {
			const overridden = detail.versions.find(
				(v) => v.id === overrideVersionId,
			);
			if (overridden) return overridden;
		}

		const installed = detail.versions.find((v) => v.isInstalled);
		if (installed) return installed;

		const compatible = detail.versions.find((v) => {
			if (!isGameVersionCompatible(v)) return false;
			return isModContent ? v.loaders.includes(filters.loader) : true;
		});
		if (compatible) return compatible;

		return detail.versions[0];
	});

	function resetPagination() {
		offset = 0;
		hasMore = true;
		items.length = 0;
		total = 0;
	}

	function resetState() {
		const fresh = parseInstanceVersion(instance);
		filters.source = "modrinth";
		filters.query = "";
		filters.loader = fresh.loader.toLowerCase();
		filters.gameVersion = fresh.gameVersion;
		filters.category = null;
		filters.sort = "downloads";
		filters.localSort = "name-asc";
		filters.localSource = "all";
		resetPagination();
		selectedId = null;
		overrideVersionId = null;
		detail.fullProject = undefined;
		detail.curseforgeDescription = "";
		detail.versions = [];
		detail.loading = false;
		detail.error = null;
		rawLocalItems = [];
		localModsById.clear();
	}

	const normalizedQuery = $derived(filters.query.trim().toLowerCase());

	function sortLocalItems(list: MarketProject[]): MarketProject[] {
		const sort = filters.localSort;
		if (sort === "name-asc")
			return [...list].sort((a, b) => a.title.localeCompare(b.title));
		if (sort === "name-desc")
			return [...list].sort((a, b) => b.title.localeCompare(a.title));
		return list;
	}

	function filterLocalItems(list: MarketProject[]): MarketProject[] {
		if (!normalizedQuery) return list;
		return list.filter(
			(m) =>
				m.title.toLowerCase().includes(normalizedQuery) ||
				m.description.toLowerCase().includes(normalizedQuery) ||
				m.author.toLowerCase().includes(normalizedQuery),
		);
	}

	function syncInstalledToItems() {
		if (filters.source === "local") return;
		for (let i = 0; i < items.length; i++) {
			const item = items[i];
			const id = item.modrinthProjectId ?? item.curseforgeProjectId;
			const installed =
				id && localModsById.has(id) ? localModsById.get(id) : undefined;
			if (item.installed !== installed) {
				items[i] = { ...item, installed };
			}
		}
	}

	function isSameProject(
		item: MarketProject,
		project: MarketProject,
	): boolean {
		if (item.id === project.id) return true;

		const modrinthId = project.modrinthProjectId;
		if (modrinthId != null && item.modrinthProjectId === modrinthId)
			return true;

		const curseId = project.curseforgeProjectId;
		if (curseId != null && item.curseforgeProjectId === curseId)
			return true;

		const filename = project.installed?.filename;
		if (
			filename != null &&
			filename !== "" &&
			item.installed?.filename === filename
		)
			return true;

		return false;
	}

	function toggleDisabledSuffix(filename: string, enabled: boolean): string {
		if (enabled) {
			return filename.replace(/\.disabled$/i, "");
		}
		return /\.disabled$/i.test(filename)
			? filename
			: `${filename}.disabled`;
	}

	function patchRawLocalItem(project: MarketProject) {
		if (!project.installed) return;
		const idx = rawLocalItems.findIndex((i) => i.id === project.id);
		if (idx !== -1) rawLocalItems[idx] = project;
	}

	function removeRawLocalItem(project: MarketProject) {
		const idx = rawLocalItems.findIndex((i) => i.id === project.id);
		if (idx !== -1) rawLocalItems.splice(idx, 1);
	}

	function setLocalItems(sorted: MarketProject[], merge = false) {
		if (merge && items.length > 0) {
			const newByFilename = new SvelteMap<string, MarketProject>();
			for (const item of sorted) {
				const key = item.installed?.filename ?? item.id;
				newByFilename.set(key, item);
			}
			for (let i = items.length - 1; i >= 0; i--) {
				const key = items[i].installed?.filename ?? items[i].id;
				const replacement = newByFilename.get(key);
				if (replacement) {
					items[i] = replacement;
					newByFilename.delete(key);
				} else {
					items.splice(i, 1);
				}
			}
			for (const item of newByFilename.values()) {
				items.push(item);
			}
		} else {
			items.length = 0;
			items.push(...sorted);
		}
		total = sorted.length;
		hasMore = false;
	}

	function applyLocalFilters(merge = false) {
		if (filters.source !== "local") return;
		let filtered = filterLocalItems(rawLocalItems);
		if (filters.localSource !== "all") {
			filtered = filtered.filter((m) => m.source === filters.localSource);
		}
		const sorted = sortLocalItems(filtered);
		setLocalItems(sorted, merge);
	}

	async function scanLocalItems(silent = false) {
		if (!silent) {
			loadingLocal = true;
			localSearchGen++;
		}
		const gen = localSearchGen;
		error = null;

		try {
			const localItems = await localLoader(instance.uuid);
			if (gen !== localSearchGen) return;

			const mapped = localItems.map((mod) => localModToMarket(mod));
			if (gen !== localSearchGen) return;

			rawLocalItems = mapped;
			localModsById.clear();
			for (const item of mapped) {
				const id = item.installed?.project_id;
				if (id) localModsById.set(id, item.installed!);
			}

			if (filters.source === "local") {
				applyLocalFilters(silent);
			} else {
				syncInstalledToItems();
			}
		} catch (e) {
			if (gen === localSearchGen) {
				error = String(e ?? "Error loading local items");
			}
		} finally {
			if (!silent && gen === localSearchGen) {
				loadingLocal = false;
			}
		}
	}

	async function searchRemoteModrinth(reset = false) {
		if (loadingRemote || loadingMore) return;

		if (reset) {
			resetPagination();
		} else if (!hasMore || loadingMore) {
			return;
		}

		const gen = ++searchGen;

		if (reset) {
			loadingRemote = true;
		} else {
			loadingMore = true;
		}
		error = null;

		try {
			const category = filters.category;
			const index = filters.sort;
			const currentOffset = offset;

			const searchLoader = isModContent ? filters.loader : "";

			const result = await searchModrinth(
				filters.query,
				searchLoader,
				filters.gameVersion,
				category,
				index,
				PAGE_SIZE,
				currentOffset,
				projectType,
			);

			if (gen !== searchGen) return;
			if (!result) return;

			const mapped = result.hits.map((hit) => {
				const project = modrinthProjectToMarket(hit);
				const id = project.modrinthProjectId ?? project.id;
				const local = localModsById.get(id);
				if (local) project.installed = local;
				return project;
			});

			if (reset) {
				items.length = 0;
			}
			items.push(...mapped);
			total = result.total_hits;
			offset = items.length;
			hasMore = items.length < result.total_hits;
		} catch (e) {
			error = String(e ?? "Error searching Modrinth");
		} finally {
			loadingRemote = false;
			loadingMore = false;
		}
	}

	async function searchRemoteCurseForge(reset = false) {
		if (!isModContent) return;
		if (loadingRemote || loadingMore) return;

		if (reset) {
			resetPagination();
		} else if (!hasMore || loadingMore) {
			return;
		}

		const gen = ++searchGen;

		if (reset) {
			loadingRemote = true;
		} else {
			loadingMore = true;
		}
		error = null;

		try {
			const categoryId = filters.category
				? CURSEFORGE_CATEGORY_IDS[filters.category]
				: null;
			const category = categoryId ? String(categoryId) : null;
			const index = filters.sort;
			const currentOffset = offset;

			const result = await searchCurseForge(
				filters.query,
				filters.loader,
				filters.gameVersion,
				category,
				index,
				PAGE_SIZE,
				currentOffset,
			);

			if (gen !== searchGen) return;
			if (!result) return;

			const mapped = result.data.map((hit) => {
				const project = curseforgeProjectToMarket(hit);
				const id = project.curseforgeProjectId ?? project.id;
				const local = localModsById.get(id);
				if (local) project.installed = local;
				return project;
			});

			if (reset) {
				items.length = 0;
			}
			items.push(...mapped);
			total = result.pagination.totalCount;
			offset = items.length;
			hasMore = items.length < result.pagination.totalCount;
		} catch (e) {
			error = String(e ?? "Error searching CurseForge");
		} finally {
			loadingRemote = false;
			loadingMore = false;
		}
	}

	function performSearch(reset = false) {
		if (filters.source === "local") {
			applyLocalFilters();
			return Promise.resolve();
		}
		if (filters.source === "curseforge") {
			return searchRemoteCurseForge(reset);
		}
		return searchRemoteModrinth(reset);
	}

	function debouncedSearch(reset = true) {
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => {
			performSearch(reset);
		}, 250);
	}

	async function loadDetail(project: MarketProject) {
		detail.loading = true;
		detail.error = null;
		overrideVersionId = null;
		detail.fullProject = undefined;
		detail.versions = [];

		if (project.source === "curseforge") {
			const projectId = project.curseforgeProjectId ?? project.id;
			if (!projectId || isNaN(Number(projectId))) {
				detail.loading = false;
				return;
			}

			try {
				const [full, files, description] = await Promise.all([
					getCurseForgeProject(Number(projectId)),
					getCurseForgeProjectFiles(
						Number(projectId),
						filters.loader,
						filters.gameVersion,
					),
					getCurseForgeProjectDescription(Number(projectId)),
				]);

				if (full) {
					detail.fullProject = full;
				}
				detail.curseforgeDescription = description ?? "";

				const installedFileId = project.curseforgeVersionId;
				detail.versions = files.map((f) =>
					curseforgeVersionToMarket(f, installedFileId),
				);
			} catch (e) {
				detail.error = String(
					e ?? "Error loading CurseForge project details",
				);
			} finally {
				detail.loading = false;
			}
			return;
		}

		const projectId =
			project.modrinthProjectId ??
			(project.source === "modrinth" ? project.id : undefined);
		if (!projectId) {
			detail.loading = false;
			return;
		}

		try {
			const versionLoader = isModContent ? filters.loader : "";
			const [full, versions] = await Promise.all([
				getModrinthProject(projectId),
				getModrinthProjectVersions(
					projectId,
					versionLoader,
					filters.gameVersion,
				),
			]);

			if (full) {
				detail.fullProject = full;
			}

			const installedVersionId = project.modrinthVersionId;
			detail.versions = versions.map((v) =>
				modrinthVersionToMarket(v, installedVersionId),
			);
		} catch (e) {
			detail.error = String(e ?? "Error loading project details");
		} finally {
			detail.loading = false;
		}
	}

	function selectProject(id: string | null) {
		selectedId = id;
		if (selectedProject) {
			loadDetail(selectedProject);
		} else {
			detail.fullProject = undefined;
			detail.versions = [];
			detail.error = null;
		}
	}

	function isGameVersionCompatible(version: MarketVersion): boolean {
		return version.gameVersions.includes(filters.gameVersion);
	}

	function setSelectedVersion(version: MarketVersion) {
		overrideVersionId = version.id;
	}

	function isVersionCompatible(version: MarketVersion): boolean {
		if (!isGameVersionCompatible(version)) return false;
		return isModContent ? version.loaders.includes(filters.loader) : true;
	}

	function isInstanceBusy() {
		return (
			instance.status === InstState.Started ||
			instance.status === InstState.Starting
		);
	}

	async function prepareInstall(
		project: MarketProject,
		version: MarketVersion,
	): Promise<
		DependencyResolutionResult & { installedProjectIds: Set<string> }
	> {
		if (isInstanceBusy()) {
			showWarning(t("errors.title"), t("errors.INST_BUSY"));
			throw new Error(t("errors.INST_BUSY"));
		}

		const mods = await getInstanceMods(instance.uuid);
		const installedProjectIds = new SvelteSet(
			mods.map((m) => m.project_id).filter((id): id is string => !!id),
		);

		if (!isModContent) {
			return { tree: [], conflicts: [], installedProjectIds };
		}

		const source =
			project.source === "curseforge" ? "curseforge" : "modrinth";
		const projectId =
			source === "curseforge"
				? (project.curseforgeProjectId ?? project.id)
				: (project.modrinthProjectId ?? project.id);

		const request: DependencyRequest = {
			source,
			project_id: projectId,
			version_id: version.id,
			kind: "required",
		};

		const result = await resolveModDependencies(
			[request],
			filters.loader,
			filters.gameVersion,
		);

		return { ...result, installedProjectIds };
	}

	async function confirmInstall(
		project: MarketProject,
		queue: ModDownloadInfo[],
	) {
		if (isInstanceBusy()) {
			showWarning(t("errors.title"), t("errors.INST_BUSY"));
			return;
		}
		if (queue.length === 0) return;

		try {
			await downloadFn(instance.uuid, queue);
			await scanLocalItems(true);

			const current = items.find((i) => i.id === project.id) ?? project;
			await loadDetail(current);
		} catch (e) {
			console.error(e);
			throw e;
		}
	}

	async function uninstall(project: MarketProject) {
		if (isInstanceBusy()) {
			showWarning(t("errors.title"), t("errors.INST_BUSY"));
			return;
		}
		if (!project.installed) return;
		try {
			await deleteInstanceFile(
				instance.uuid,
				subDir,
				project.installed.filename,
			);
			// Remove installed state from all matching items
			for (const item of items) {
				if (isSameProject(item, project)) {
					item.installed = undefined;
					item.installedVersion = undefined;
					item.modrinthVersionId = undefined;
					item.modrinthProjectId = undefined;
					item.curseforgeVersionId = undefined;
					item.curseforgeProjectId = undefined;
				}
			}
			if (filters.source === "local") {
				const installedId =
					project.modrinthProjectId ?? project.curseforgeProjectId;
				if (installedId) localModsById.delete(installedId);
				removeRawLocalItem(project);
				const idx = items.findIndex((i) => i.id === project.id);
				if (idx !== -1) items.splice(idx, 1);
				total = items.length;
			}
			selectProject(null);
		} catch (e) {
			console.error(e);
		}
	}

	async function toggleEnabled(project: MarketProject) {
		if (isInstanceBusy()) {
			showWarning(t("errors.title"), t("errors.INST_BUSY"));
			return;
		}
		if (!project.installed || !isModContent) return;
		const newEnabled = !project.installed.enabled;
		try {
			await toggleInstanceMod(
				instance.uuid,
				project.installed.filename,
				newEnabled,
			);
			// Update enabled state and filename in all matching items
			for (const item of items) {
				if (item.installed && isSameProject(item, project)) {
					item.installed.enabled = newEnabled;
					item.installed.filename = toggleDisabledSuffix(
						item.installed.filename,
						newEnabled,
					);
					item.disabled = !newEnabled;
				}
			}
			patchRawLocalItem(project);
			if (selectedId && filters.source !== "local") {
				await loadDetail(project);
			}
		} catch (e) {
			console.error(e);
		}
	}

	function loadMore() {
		if (
			filters.source !== "local" &&
			hasMore &&
			!loadingRemote &&
			!loadingMore
		) {
			performSearch(false);
		}
	}

	function setSource(source: MarketSource) {
		if (source === "curseforge" && !isModContent) return;
		filters.source = source;
		selectedId = null;
		clearTimeout(searchTimer);
		searchTimer = setTimeout(async () => {
			if (source === "local") {
				if (rawLocalItems.length === 0) {
					await scanLocalItems();
				} else {
					applyLocalFilters();
				}
			} else {
				await Promise.all([scanLocalItems(true), performSearch(true)]);
			}
		}, 200);
	}

	function setQuery(query: string) {
		filters.query = query;
		debouncedSearch(true);
	}

	function setCategory(category: string | null) {
		filters.category = category;
		debouncedSearch(true);
	}

	function setSort(sort: MarketSort) {
		filters.sort = sort;
		debouncedSearch(true);
	}

	function setLocalSort(sort: LocalSort) {
		filters.localSort = sort;
		if (filters.source === "local") {
			applyLocalFilters();
		}
	}

	function setLocalSource(source: LocalSourceFilter) {
		filters.localSource = source;
		if (filters.source === "local") {
			applyLocalFilters();
		}
	}

	// Watch instance changes and reset
	let lastInstanceId = "";
	$effect(() => {
		if (instance.uuid !== lastInstanceId) {
			lastInstanceId = instance.uuid;
			resetState();
			Promise.all([scanLocalItems(true), performSearch(true)]);
		}
	});

	// Auto-refresh local items when background enrichment completes
	const _unregisterRefresh = registerModsRefreshCallback(
		instance.uuid,
		() => {
			scanLocalItems(true);
		},
	);

	function destroy() {
		searchGen++;
		localSearchGen++;
		clearTimeout(searchTimer);
		searchTimer = undefined;
		_unregisterRefresh();

		items.length = 0;
		total = 0;
		selectedId = null;
		overrideVersionId = null;
		detail.fullProject = undefined;
		detail.curseforgeDescription = "";
		detail.versions = [];
		detail.loading = false;
		detail.error = null;
		rawLocalItems = [];
		localModsById.clear();
	}

	return {
		get filters() {
			return filters;
		},
		get items() {
			return items;
		},
		get total() {
			return total;
		},
		get loading() {
			return loadingLocal || loadingRemote;
		},
		get loadingLocal() {
			return loadingLocal;
		},
		get loadingRemote() {
			return loadingRemote;
		},
		get loadingMore() {
			return loadingMore;
		},
		get error() {
			return error;
		},
		get hasMore() {
			return hasMore;
		},
		get selectedId() {
			return selectedId;
		},
		get selectedProject() {
			return selectedProject;
		},
		get detail() {
			return detail;
		},
		get selectedVersion() {
			return selectedVersion;
		},
		setSelectedVersion,
		isVersionCompatible,
		setSource,
		setQuery,
		setCategory,
		setSort,
		setLocalSort,
		setLocalSource,
		selectProject,
		loadMore,
		prepareInstall,
		confirmInstall,
		uninstall,
		toggleEnabled,
		refresh: () => performSearch(true),
		destroy,
	};
}
