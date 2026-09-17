import { SvelteMap, SvelteSet } from "svelte/reactivity";
import {
	deleteInstanceFile,
	addInstanceFile,
	getInstanceMods,
	getInstanceModIcons,
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
import { createLocalModIcons } from "$lib/state/localModIcons";
import {
	reconcileLocalProjects,
	sameProjectList,
	matchesLocalQuery,
} from "$lib/utils/localCatalog";

const PAGE_SIZE = 20;
const MAX_CACHED_PAGES = 15;

export type MarketSource = "local" | "modrinth" | "curseforge";

export type MarketSort = "auto" | "relevance" | "downloads" | "newest";
export type LocalSort = "name-asc" | "name-desc";
export type LocalSourceFilter = "all" | "modrinth" | "curseforge" | "local";
export type LocalStatusFilter = "all" | "enabled" | "disabled";
export type LocalAction = "enable" | "disable" | "delete";
export interface LocalOperationReport {
	total: number;
	completed: number;
	succeeded: number;
	failures: { filename: string; error: string }[];
}

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
	localStatus: LocalStatusFilter;
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
	getStatus = () => instance.status,
) {
	const parsed = parseInstanceVersion(instance);

	const isModContent = contentType === "mods";
	const localLoader = isModContent
		? (id: string) => getInstanceMods(id, false)
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
		sort: "auto",
		localSort: "name-asc",
		localSource: "all",
		localStatus: "all",
	});

	// API metadata is immutable. Replacing the list avoids a deep proxy/source
	// graph for every cached project and lets evicted pages be collected directly.
	let items = $state.raw<MarketProject[]>([]);
	let total = $state(0);
	let loadingLocal = $state(false);
	let loadingRemote = $state(false);
	let loadingMore = $state(false);
	let error = $state<string | null>(null);
	let offset = $state(0);
	let hasMore = $state(true);
	const localModsById = new SvelteMap<string, ModDto>();
	let rawLocalItems = $state.raw<MarketProject[]>([]);
	const localIcons = createLocalModIcons((files) =>
		getInstanceModIcons(instance.uuid, files),
	);
	const sortedLocalItems = $derived.by(() => sortLocalItems(rawLocalItems));
	const checkedFiles = new SvelteSet<string>();
	let localOperationBusy = $state(false);
	let localOperationReport = $state<LocalOperationReport | null>(null);
	let selectedId = $state<string | null>(null);
	const detail = $state<MarketDetailState>({
		versions: [],
		loading: false,
		error: null,
	});

	let overrideVersionId = $state<string | null>(null);
	let searchGen = 0;
	let detailGen = 0;
	let searchPending = $state(false);
	let resultsRevision = $state(0);
	let localSearchGen = 0;
	let searchTimer: ReturnType<typeof setTimeout> | undefined;
	let pendingLocalRename: { from: string; to: string } | null = null;
	let disposed = false;
	let searchController: AbortController | undefined;
	let detailController: AbortController | undefined;
	let scanInFlight: Promise<void> | undefined;
	let scanAgain = false;
	const pages = new SvelteMap<number, MarketProject[]>();
	const visiblePositions = new SvelteMap<string, number>();
	let loadedCount = $state(0);
	let failedPageOffset: number | undefined;

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
		pages.clear();
		visiblePositions.clear();
		loadedCount = 0;
		failedPageOffset = undefined;
		resultsRevision++;
		offset = 0;
		hasMore = true;
		items = [];
		total = 0;
	}

	function resetState() {
		invalidateSearch();
		detailGen++;
		const fresh = parseInstanceVersion(instance);
		filters.source = "modrinth";
		filters.query = "";
		filters.loader = fresh.loader.toLowerCase();
		filters.gameVersion = fresh.gameVersion;
		filters.category = null;
		filters.sort = "auto";
		filters.localSort = "name-asc";
		filters.localSource = "all";
		filters.localStatus = "all";
		checkedFiles.clear();
		localOperationReport = null;
		resetPagination();
		selectedId = null;
		pendingLocalRename = null;
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
	const searchSort = $derived(
		filters.sort === "auto"
			? normalizedQuery
				? "relevance"
				: "downloads"
			: filters.sort,
	);

	function invalidateSearch() {
		searchGen++;
		searchController?.abort();
		searchController = undefined;
		clearTimeout(searchTimer);
		searchTimer = undefined;
		searchPending = false;
		loadingRemote = false;
		loadingMore = false;
		error = null;
	}

	function appendRemoteItems(
		mapped: MarketProject[],
		resultTotal: number,
		pageOffset: number,
	) {
		if (pageOffset >= loadedCount)
			hasMore =
				mapped.length > 0 && pageOffset + mapped.length < resultTotal;
		loadedCount = Math.min(
			resultTotal,
			Math.max(loadedCount, pageOffset + mapped.length),
		);
		offset = loadedCount;
		total = resultTotal;
		pages.delete(pageOffset);
		pages.set(pageOffset, mapped);
		while (pages.size > MAX_CACHED_PAGES) {
			const oldest = pages.keys().next().value;
			if (oldest === undefined) break;
			pages.delete(oldest);
		}
		// The virtual grid keeps provider offsets even when pages overlap.
		// Null slots are duplicates; absent pages alone need fetching again.
		visiblePositions.clear();
		const retained: MarketProject[] = [];
		for (const [start, page] of pages) {
			for (let i = 0; i < page.length; i++) {
				const item = page[i];
				const previous = visiblePositions.get(item.id);
				visiblePositions.set(
					item.id,
					Math.min(previous ?? Infinity, start + i),
				);
				if (previous === undefined) {
					retained.push(item);
				}
			}
		}
		items = retained;
	}

	function getItem(index: number): MarketProject | null | undefined {
		if (filters.source === "local") return items[index];
		for (const [start, page] of pages) {
			if (index >= start && index < start + page.length) {
				const item = page[index - start];
				return visiblePositions.get(item.id) === index ? item : null;
			}
		}
		return undefined;
	}

	function ensureRange(first: number, last: number) {
		if (!disposed && !selectedId && filters.source === "local") {
			localIcons.request(items.slice(first, last + 1));
			return;
		}
		if (
			disposed ||
			selectedId ||
			filters.source === "local" ||
			searchPending ||
			loadingRemote ||
			loadingMore ||
			error
		)
			return;
		for (let index = first; index <= last && index < loadedCount; index++) {
			const cached = getItem(index) !== undefined || pages.has(index);
			if (!cached) {
				void performSearch(false, index);
				return;
			}
		}
	}

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
		return list.filter((item) => matchesLocalQuery(item, normalizedQuery));
	}

	function syncInstalledToItems() {
		if (filters.source === "local") return;
		for (const [start, page] of pages) {
			const next = page.map((item) => {
				const id = item.modrinthProjectId ?? item.curseforgeProjectId;
				const installed = id ? localModsById.get(id) : undefined;
				return item.installed === installed
					? item
					: { ...item, installed };
			});
			if (!sameProjectList(page, next)) pages.set(start, next);
		}
		const updated = [...items];
		for (let i = 0; i < updated.length; i++) {
			const item = updated[i];
			const id = item.modrinthProjectId ?? item.curseforgeProjectId;
			const installed =
				id && localModsById.has(id) ? localModsById.get(id) : undefined;
			if (item.installed !== installed) {
				updated[i] = { ...item, installed };
			}
		}
		if (!sameProjectList(items, updated)) items = updated;
	}

	function toggleDisabledSuffix(filename: string, enabled: boolean): string {
		if (enabled) {
			return filename.replace(/\.disabled$/i, "");
		}
		return /\.disabled$/i.test(filename)
			? filename
			: `${filename}.disabled`;
	}

	function setLocalItems(sorted: MarketProject[], merge = false) {
		let next = sorted;
		if (merge && items.length > 0) {
			const updated: MarketProject[] = [];
			const newByFilename = new SvelteMap<string, MarketProject>();
			for (const item of sorted) {
				const key = item.installed?.filename ?? item.id;
				newByFilename.set(key, item);
			}
			for (const item of items) {
				const key = item.installed?.filename ?? item.id;
				const replacement = newByFilename.get(key);
				if (replacement) {
					updated.push(replacement);
					newByFilename.delete(key);
				}
			}
			for (const item of newByFilename.values()) {
				updated.push(item);
			}
			next = updated;
		}
		if (!sameProjectList(items, next)) items = next;
		total = sorted.length;
		hasMore = false;
	}

	function applyLocalFilters(merge = false) {
		if (filters.source !== "local") return;
		let filtered = filterLocalItems(sortedLocalItems);
		if (filters.localSource !== "all") {
			filtered = filtered.filter((m) => m.source === filters.localSource);
		}
		if (isModContent && filters.localStatus !== "all") {
			filtered = filtered.filter(
				(m) =>
					!!m.installed?.enabled ===
					(filters.localStatus === "enabled"),
			);
		}
		setLocalItems(filtered, merge);
	}

	function scanLocalItems(silent = false): Promise<void> {
		if (disposed) return Promise.resolve();
		if (scanInFlight) {
			scanAgain = true;
			localSearchGen++;
			if (!silent) loadingLocal = true;
			return scanInFlight;
		}
		scanInFlight = (async () => {
			do {
				scanAgain = false;
				await runLocalScan(silent);
			} while (scanAgain && !disposed);
		})().finally(() => {
			scanInFlight = undefined;
			if (!disposed) loadingLocal = false;
		});
		return scanInFlight;
	}

	async function runLocalScan(silent = false) {
		if (disposed) return;
		if (!silent) {
			loadingLocal = true;
		}
		const gen = ++localSearchGen;
		error = null;

		try {
			const localItems = await localLoader(instance.uuid);
			if (gen !== localSearchGen) return;

			const mapped = reconcileLocalProjects(rawLocalItems, localItems);
			if (gen !== localSearchGen) return;

			rawLocalItems = mapped;
			localIcons.sync(mapped);
			// eslint-disable-next-line svelte/prefer-svelte-reactivity -- Temporary scan lookup, never rendered.
			const filenames = new Set(localItems.map((item) => item.filename));
			for (const filename of checkedFiles) {
				if (!filenames.has(filename)) checkedFiles.delete(filename);
			}
			// Reconcile against the winning scan, even if enrichment superseded a toggle's scan.
			if (pendingLocalRename) {
				const { from, to } = pendingLocalRename;
				if (
					selectedId === from &&
					!mapped.some((item) => item.id === from) &&
					mapped.some((item) => item.id === to)
				) {
					selectedId = to;
				}
				pendingLocalRename = null;
			}
			localModsById.clear();
			for (const item of mapped) {
				const id = item.installed?.project_id;
				if (id) localModsById.set(id, item.installed!);
			}

			if (filters.source === "local") {
				applyLocalFilters(silent);
				if (selectedProject) localIcons.request([selectedProject]);
			} else {
				syncInstalledToItems();
			}
		} catch (e) {
			if (gen === localSearchGen) {
				error = String(e ?? "Error loading local items");
			}
		} finally {
			if (gen === localSearchGen) {
				loadingLocal = false;
			}
		}
	}

	async function searchRemoteModrinth(reset = false, pageOffset?: number) {
		if (disposed) return;
		if (!reset && (loadingRemote || loadingMore)) return;

		if (reset) {
			resetPagination();
		} else if ((pageOffset === undefined && !hasMore) || loadingMore) {
			return;
		}

		const gen = ++searchGen;
		const controller = new AbortController();
		searchController = controller;
		const currentOffset = pageOffset ?? offset;

		if (reset) {
			loadingRemote = true;
		} else {
			loadingMore = true;
		}
		error = null;

		try {
			const category = filters.category;
			const index = searchSort;

			const searchLoader = isModContent ? filters.loader : "";

			const result = await searchModrinth(
				filters.query.trim(),
				searchLoader,
				filters.gameVersion,
				category,
				index,
				PAGE_SIZE,
				currentOffset,
				projectType,
				controller.signal,
			);

			if (gen !== searchGen) return;
			if (!result) throw new Error(t("market.browse.searchError"));

			const mapped = result.hits.map((hit) => {
				const project = modrinthProjectToMarket(hit);
				const id = project.modrinthProjectId ?? project.id;
				const local = localModsById.get(id);
				if (local) project.installed = local;
				return project;
			});

			appendRemoteItems(mapped, result.total_hits, currentOffset);
			failedPageOffset = undefined;
		} catch (e) {
			if (gen === searchGen) {
				failedPageOffset = currentOffset;
				error = String(e ?? "Error searching Modrinth");
			}
		} finally {
			if (gen === searchGen) {
				searchController = undefined;
				loadingRemote = false;
				loadingMore = false;
			}
		}
	}

	async function searchRemoteCurseForge(reset = false, pageOffset?: number) {
		if (disposed) return;
		if (!isModContent) return;
		if (!reset && (loadingRemote || loadingMore)) return;

		if (reset) {
			resetPagination();
		} else if ((pageOffset === undefined && !hasMore) || loadingMore) {
			return;
		}

		const gen = ++searchGen;
		const controller = new AbortController();
		searchController = controller;
		const currentOffset = pageOffset ?? offset;

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
			const index = searchSort;

			const result = await searchCurseForge(
				filters.query.trim(),
				filters.loader,
				filters.gameVersion,
				category,
				index,
				PAGE_SIZE,
				currentOffset,
				controller.signal,
			);

			if (gen !== searchGen) return;
			if (!result) throw new Error(t("market.browse.searchError"));

			const mapped = result.data.map((hit) => {
				const project = curseforgeProjectToMarket(hit);
				const id = project.curseforgeProjectId ?? project.id;
				const local = localModsById.get(id);
				if (local) project.installed = local;
				return project;
			});

			appendRemoteItems(
				mapped,
				result.pagination.totalCount,
				currentOffset,
			);
			failedPageOffset = undefined;
		} catch (e) {
			if (gen === searchGen) {
				failedPageOffset = currentOffset;
				error = String(e ?? "Error searching CurseForge");
			}
		} finally {
			if (gen === searchGen) {
				searchController = undefined;
				loadingRemote = false;
				loadingMore = false;
			}
		}
	}

	function performSearch(reset = false, pageOffset?: number) {
		if (disposed) return Promise.resolve();
		if (reset) {
			invalidateSearch();
			selectProject(null);
		}
		if (filters.source === "local") {
			applyLocalFilters();
			return Promise.resolve();
		}
		if (filters.source === "curseforge") {
			return searchRemoteCurseForge(reset, pageOffset);
		}
		return searchRemoteModrinth(reset, pageOffset);
	}

	function debouncedSearch(reset = true) {
		if (disposed) return;
		// Invalidate immediately: an old response can arrive during the debounce.
		invalidateSearch();
		selectProject(null);
		searchPending = true;
		searchTimer = setTimeout(() => {
			searchTimer = undefined;
			searchPending = false;
			performSearch(reset);
		}, 250);
	}

	async function loadDetail(project: MarketProject) {
		if (disposed) return;
		detailController?.abort();
		const controller = new AbortController();
		detailController = controller;
		const gen = ++detailGen;
		detail.loading = true;
		detail.error = null;
		overrideVersionId = null;
		detail.fullProject = undefined;
		detail.curseforgeDescription = "";
		detail.versions = [];

		if (project.source === "curseforge") {
			const projectId = project.curseforgeProjectId ?? project.id;
			if (!projectId || isNaN(Number(projectId))) {
				detail.loading = false;
				detailController = undefined;
				return;
			}

			try {
				const [full, files, description] = await Promise.all([
					getCurseForgeProject(Number(projectId), controller.signal),
					getCurseForgeProjectFiles(
						Number(projectId),
						filters.loader,
						filters.gameVersion,
						controller.signal,
					),
					getCurseForgeProjectDescription(
						Number(projectId),
						controller.signal,
					),
				]);

				if (gen !== detailGen) return;
				if (full) {
					detail.fullProject = full;
				}
				detail.curseforgeDescription = description ?? "";

				const installedFileId = project.curseforgeVersionId;
				detail.versions = files.map((f) =>
					curseforgeVersionToMarket(f, installedFileId),
				);
			} catch (e) {
				if (gen !== detailGen) return;
				detail.error = String(
					e ?? "Error loading CurseForge project details",
				);
			} finally {
				controller.abort();
				if (gen === detailGen) {
					detail.loading = false;
					detailController = undefined;
				}
			}
			return;
		}

		const projectId =
			project.modrinthProjectId ??
			(project.source === "modrinth" ? project.id : undefined);
		if (!projectId) {
			detail.loading = false;
			detailController = undefined;
			return;
		}

		try {
			const versionLoader = isModContent ? filters.loader : "";
			const [full, versions] = await Promise.all([
				getModrinthProject(projectId, controller.signal),
				getModrinthProjectVersions(
					projectId,
					versionLoader,
					filters.gameVersion,
					controller.signal,
				),
			]);

			if (gen !== detailGen) return;
			if (full) {
				detail.fullProject = full;
			}

			const installedVersionId = project.modrinthVersionId;
			detail.versions = versions.map((v) =>
				modrinthVersionToMarket(v, installedVersionId),
			);
		} catch (e) {
			if (gen !== detailGen) return;
			detail.error = String(e ?? "Error loading project details");
		} finally {
			controller.abort();
			if (gen === detailGen) {
				detail.loading = false;
				detailController = undefined;
			}
		}
	}

	function selectProject(id: string | null) {
		if (disposed) return;
		detailController?.abort();
		detailController = undefined;
		detailGen++;
		pendingLocalRename = null;
		selectedId = id;
		if (selectedProject) {
			localIcons.request([selectedProject]);
			loadDetail(selectedProject);
		} else {
			detail.loading = false;
			detail.fullProject = undefined;
			detail.curseforgeDescription = "";
			detail.versions = [];
			detail.error = null;
			overrideVersionId = null;
		}
	}

	function isGameVersionCompatible(version: MarketVersion): boolean {
		return version.gameVersions.includes(filters.gameVersion);
	}

	function setSelectedVersion(version: MarketVersion) {
		if (disposed) return;
		overrideVersionId = version.id;
	}

	function isVersionCompatible(version: MarketVersion): boolean {
		if (!isGameVersionCompatible(version)) return false;
		return isModContent ? version.loaders.includes(filters.loader) : true;
	}

	function isInstanceBusy() {
		const status = getStatus();
		return status === InstState.Started || status === InstState.Starting;
	}

	async function prepareInstall(
		project: MarketProject,
		version: MarketVersion,
	): Promise<
		DependencyResolutionResult & { installedProjectIds: Set<string> }
	> {
		if (disposed) throw new DOMException("Market closed", "AbortError");
		if (isInstanceBusy()) {
			showWarning(t("errors.title"), t("errors.INST_BUSY"));
			throw new Error(t("errors.INST_BUSY"));
		}

		const mods = await getInstanceMods(instance.uuid, false);
		if (disposed) throw new DOMException("Market closed", "AbortError");
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
		if (disposed) throw new DOMException("Market closed", "AbortError");

		return { ...result, installedProjectIds };
	}

	async function confirmInstall(
		project: MarketProject,
		queue: ModDownloadInfo[],
	) {
		if (disposed) return;
		if (isInstanceBusy()) {
			showWarning(t("errors.title"), t("errors.INST_BUSY"));
			return;
		}
		if (queue.length === 0) return;

		try {
			await downloadFn(instance.uuid, queue);
			if (disposed) return;
			await scanLocalItems(true);
			if (disposed) return;

			if (selectedId === project.id) {
				const current =
					items.find((i) => i.id === project.id) ?? project;
				await loadDetail(current);
			}
		} catch (e) {
			console.error(e);
			throw e;
		}
	}

	async function uninstall(project: MarketProject) {
		if (disposed || localOperationBusy) return;
		if (isInstanceBusy()) {
			showWarning(t("errors.title"), t("errors.INST_BUSY"));
			return;
		}
		if (!project.installed) return;
		const filename = project.installed.filename;
		try {
			await deleteInstanceFile(instance.uuid, subDir, filename);
			if (disposed) return;
			// Refresh by file; another version of this project may still be installed.
			await scanLocalItems(true);
			if (
				selectedId === project.id &&
				!rawLocalItems.some(
					(item) => item.installed?.filename === filename,
				)
			) {
				selectProject(null);
			}
		} catch (e) {
			console.error(e);
		}
	}

	async function toggleEnabled(project: MarketProject) {
		if (disposed || localOperationBusy) return;
		if (isInstanceBusy()) {
			showWarning(t("errors.title"), t("errors.INST_BUSY"));
			return;
		}
		if (!project.installed || !isModContent) return;
		const newEnabled = !project.installed.enabled;
		const filename = project.installed.filename;
		try {
			await toggleInstanceMod(instance.uuid, filename, newEnabled);
			if (disposed) return;
			if (filters.source === "local" && selectedId === project.id) {
				pendingLocalRename = {
					from: project.id,
					to: `local-${toggleDisabledSuffix(filename, newEnabled)}`,
				};
			}
			await scanLocalItems(true);
			if (
				selectedId === project.id &&
				filters.source !== "local" &&
				selectedProject
			) {
				await loadDetail(selectedProject);
			}
		} catch (e) {
			console.error(e);
		}
	}

	function toggleChecked(filename: string) {
		if (disposed || localOperationBusy) return;
		if (checkedFiles.has(filename)) checkedFiles.delete(filename);
		else if (items.some((item) => item.installed?.filename === filename))
			checkedFiles.add(filename);
	}

	function selectAllLocal() {
		if (disposed || localOperationBusy || filters.source !== "local")
			return;
		for (const item of items) {
			if (item.installed) checkedFiles.add(item.installed.filename);
		}
	}

	function clearChecked() {
		if (!localOperationBusy) checkedFiles.clear();
	}

	async function runLocalBatch(
		filenames: string[],
		operation: (filename: string) => Promise<void>,
	) {
		if (disposed || localOperationBusy || filenames.length === 0) return;
		if (isInstanceBusy()) {
			showWarning(t("errors.title"), t("errors.INST_BUSY"));
			return;
		}
		localOperationBusy = true;
		localOperationReport = {
			total: filenames.length,
			completed: 0,
			succeeded: 0,
			failures: [],
		};
		let next = 0;
		// A bounded worker pool avoids flooding IPC and rescanning after each file.
		async function worker() {
			while (!disposed && next < filenames.length) {
				const filename = filenames[next++];
				try {
					if (isInstanceBusy())
						throw new Error(t("errors.INST_BUSY"));
					await operation(filename);
					if (disposed) return;
					checkedFiles.delete(filename);
					localOperationReport!.succeeded++;
				} catch (error) {
					if (disposed) return;
					localOperationReport!.failures.push({
						filename,
						error: String(error),
					});
				}
				localOperationReport!.completed++;
			}
		}
		try {
			await Promise.all(
				Array.from({ length: Math.min(4, filenames.length) }, worker),
			);
			if (!disposed) {
				await scanLocalItems(true);
				if (selectedId && !items.some((item) => item.id === selectedId))
					selectProject(null);
			}
		} finally {
			localOperationBusy = false;
		}
	}

	async function manageLocal(
		action: LocalAction,
		filenames = [...checkedFiles],
	) {
		if (
			disposed ||
			localOperationBusy ||
			(action !== "delete" && !isModContent)
		)
			return;
		// eslint-disable-next-line svelte/prefer-svelte-reactivity -- Immutable batch snapshot, never rendered.
		const byFilename = new Map(
			rawLocalItems.map((item) => [item.installed!.filename, item]),
		);
		// eslint-disable-next-line svelte/prefer-svelte-reactivity -- Deduplicate the snapshot without reactive allocations.
		const targets = [...new Set(filenames)].filter((filename) =>
			byFilename.has(filename),
		);
		await runLocalBatch(targets, async (filename) => {
			if (action === "delete") {
				await deleteInstanceFile(instance.uuid, subDir, filename, true);
				return;
			}
			const enabled = action === "enable";
			if (byFilename.get(filename)!.installed!.enabled === enabled)
				return;
			const renamed = toggleDisabledSuffix(filename, enabled);
			if (byFilename.has(renamed))
				throw new Error(t("market.manage.fileExists"));
			await toggleInstanceMod(instance.uuid, filename, enabled, true);
			if (selectedId === `local-${filename}`) {
				pendingLocalRename = {
					from: selectedId,
					to: `local-${renamed}`,
				};
			}
		});
	}

	async function importLocal(paths: string[]) {
		if (disposed || localOperationBusy) return;
		// eslint-disable-next-line svelte/prefer-svelte-reactivity -- Private collision lookup, never rendered.
		const existing = new Set(
			rawLocalItems.map((item) => item.installed!.filename),
		);
		// eslint-disable-next-line svelte/prefer-svelte-reactivity -- Deduplicate the snapshot without reactive allocations.
		await runLocalBatch([...new Set(paths)], async (path) => {
			const filename = path.split(/[\\/]/).pop() ?? "";
			const extension = isModContent
				? /\.jar(?:\.disabled)?$/i
				: /\.zip$/i;
			if (!extension.test(filename))
				throw new Error(t("market.manage.invalidFile"));
			if (existing.has(filename))
				throw new Error(t("market.manage.fileExists"));
			existing.add(filename);
			await addInstanceFile(instance.uuid, subDir, path, false);
		});
	}

	function loadMore() {
		if (disposed || selectedId) return;
		if (
			filters.source !== "local" &&
			searchTimer === undefined &&
			!error &&
			hasMore &&
			!loadingRemote &&
			!loadingMore
		) {
			performSearch(false);
		}
	}

	function setSource(source: MarketSource) {
		if (disposed) return;
		if (source === "curseforge" && !isModContent) return;
		if (source === filters.source) return;
		// Invalidate in-flight pages before changing the list's source.
		invalidateSearch();
		resetPagination();
		filters.source = source;
		localIcons.request([]);
		checkedFiles.clear();
		selectProject(null);
		searchPending = true;
		searchTimer = setTimeout(async () => {
			searchTimer = undefined;
			searchPending = false;
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
		if (disposed) return;
		if (query === filters.query) return;
		filters.query = query;
		if (filters.source === "local") {
			resultsRevision++;
			selectProject(null);
			checkedFiles.clear();
			applyLocalFilters();
			return;
		}
		debouncedSearch(true);
	}

	function setCategory(category: string | null) {
		if (disposed) return;
		if (category === filters.category) return;
		filters.category = category;
		debouncedSearch(true);
	}

	function setSort(sort: MarketSort) {
		if (disposed) return;
		if (sort === filters.sort) return;
		filters.sort = sort;
		debouncedSearch(true);
	}

	function setLocalSort(sort: LocalSort) {
		if (disposed) return;
		filters.localSort = sort;
		if (filters.source === "local") {
			resultsRevision++;
			applyLocalFilters();
		}
	}

	function setLocalSource(source: LocalSourceFilter) {
		if (disposed) return;
		filters.localSource = source;
		checkedFiles.clear();
		if (filters.source === "local") {
			resultsRevision++;
			selectProject(null);
			applyLocalFilters();
		}
	}

	function setLocalStatus(status: LocalStatusFilter) {
		if (disposed || !isModContent || status === filters.localStatus) return;
		filters.localStatus = status;
		checkedFiles.clear();
		resultsRevision++;
		selectProject(null);
		applyLocalFilters();
	}

	function clearFilters() {
		if (disposed) return;
		filters.category = null;
		filters.sort = "auto";
		filters.localSort = "name-asc";
		filters.localSource = "all";
		filters.localStatus = "all";
		checkedFiles.clear();
		if (filters.source === "local") {
			resultsRevision++;
			applyLocalFilters();
		} else {
			debouncedSearch(true);
		}
	}

	function refresh() {
		if (localOperationBusy) return;
		if (filters.source === "local") return scanLocalItems();
		return performSearch(true);
	}

	function retry() {
		if (filters.source === "local") return refresh();
		return performSearch(items.length === 0, failedPageOffset);
	}

	// Watch instance changes and reset
	let lastInstanceId = "";
	$effect(() => {
		if (!disposed && instance.uuid !== lastInstanceId) {
			lastInstanceId = instance.uuid;
			resetState();
			Promise.all([scanLocalItems(true), performSearch(true)]);
		}
	});

	// Auto-refresh local items when background enrichment completes
	const _unregisterRefresh = registerModsRefreshCallback(
		instance.uuid,
		() => {
			if (localOperationBusy) return;
			scanLocalItems(true);
		},
	);

	function destroy() {
		if (disposed) return;
		disposed = true;
		detailController?.abort();
		detailController = undefined;
		scanAgain = false;
		loadingLocal = false;
		pages.clear();
		visiblePositions.clear();
		loadedCount = 0;
		hasMore = false;
		detailGen++;
		pendingLocalRename = null;
		invalidateSearch();
		localSearchGen++;
		_unregisterRefresh();
		localOperationReport = null;

		items = [];
		localIcons.destroy();
		total = 0;
		selectedId = null;
		overrideVersionId = null;
		detail.fullProject = undefined;
		detail.curseforgeDescription = "";
		detail.versions = [];
		detail.loading = false;
		detail.error = null;
		rawLocalItems = [];
		checkedFiles.clear();
		localModsById.clear();
	}

	return {
		getLocalIcon: localIcons.get,
		checkedFiles,
		toggleChecked,
		selectAllLocal,
		clearChecked,
		manageLocal,
		importLocal,
		setLocalStatus,
		get localOperationBusy() {
			return localOperationBusy;
		},
		get localOperationReport() {
			return localOperationReport;
		},
		get localCount() {
			return rawLocalItems.length;
		},
		get instanceBusy() {
			return isInstanceBusy();
		},
		get itemCount() {
			return filters.source === "local" ? items.length : loadedCount;
		},
		get cachedPageCount() {
			return pages.size;
		},
		getItem,
		ensureRange,
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
			return loadingLocal || loadingRemote || searchPending;
		},
		get resultsRevision() {
			return resultsRevision;
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
		clearFilters,
		selectProject,
		loadMore,
		prepareInstall,
		confirmInstall,
		uninstall,
		toggleEnabled,
		refresh,
		retry,
		destroy,
	};
}
