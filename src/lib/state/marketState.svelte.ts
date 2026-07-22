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
	getCurseForgeFileDownloadUrl,
	toggleInstanceMod,
	downloadMods,
	downloadResourcePacks,
	downloadShaderPacks,
	CURSEFORGE_HEADERS,
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
import { showWarning } from "$lib/state/state.svelte";
import { t } from "$lib/i18n";

const PAGE_SIZE = 20;

export type MarketSource = "local" | "modrinth" | "curseforge";

export type MarketSort = "relevance" | "downloads" | "newest";
export type LocalSort = "name-asc" | "name-desc";

export interface MarketFilters {
	source: MarketSource;
	query: string;
	loader: string;
	gameVersion: string;
	category: string | null;
	sort: MarketSort;
	localSort: LocalSort;
}

export interface MarketDetailState {
	fullProject?: ModrinthProjectFull | CurseForgeProject;
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
	});

	const items = $state<MarketProject[]>([]);
	let total = $state(0);
	let loading = $state(false);
	let loadingMore = $state(false);
	let error = $state<string | null>(null);
	let offset = $state(0);
	let hasMore = $state(true);
	let localModsById = $state<Map<string, ModDto>>(new Map());
	let selectedId = $state<string | null>(null);
	const detail = $state<MarketDetailState>({
		versions: [],
		loading: false,
		error: null,
	});

	let overrideVersionId = $state<string | null>(null);
	let abortController: AbortController | null = null;
	let localAbortController: AbortController | null = null;
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	const selectedProject = $derived<MarketProject | null>(
		items.find((i) => i.id === selectedId) ?? null,
	);

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
		resetPagination();
		selectedId = null;
		overrideVersionId = null;
		detail.fullProject = undefined;
		detail.versions = [];
		detail.loading = false;
		detail.error = null;
	}

	function abortPending() {
		if (abortController) {
			abortController.abort();
			abortController = null;
		}
	}

	function sortLocalItems(list: MarketProject[]): MarketProject[] {
		const sort = filters.localSort;
		if (sort === "name-asc")
			return [...list].sort((a, b) => a.title.localeCompare(b.title));
		if (sort === "name-desc")
			return [...list].sort((a, b) => b.title.localeCompare(a.title));
		return [...list];
	}

	async function loadLocalItems(silent = false) {
		if (!silent) loading = true;
		error = null;
		if (!silent) {
			abortPending();
			localAbortController?.abort();
			localAbortController = new AbortController();
		}
		const signal = localAbortController?.signal;

		try {
			const localItems = await localLoader(instance.uuid);
			if (signal?.aborted) return;

			const mapped = localItems.map((mod) => localModToMarket(mod));
			if (signal?.aborted) return;

			const query = filters.query.trim().toLowerCase();
			const filtered = query
				? mapped.filter(
						(m) =>
							m.title.toLowerCase().includes(query) ||
							m.description.toLowerCase().includes(query) ||
							m.author.toLowerCase().includes(query),
					)
				: mapped;

			const sorted = sortLocalItems(filtered);

			if (silent && items.length > 0) {
				const newByFilename = new Map<string, MarketProject>();
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

			const newMap = new Map<string, ModDto>();
			for (const item of sorted) {
				const id = item.installed?.project_id;
				if (id) newMap.set(id, item.installed!);
			}
			localModsById = newMap;

			if (newMap.size > 0 && filters.source !== "local") {
				for (const item of items) {
					const id = item.modrinthProjectId ?? item.curseforgeProjectId;
					if (id && newMap.has(id)) {
						item.installed = newMap.get(id)!;
					}
				}
			}
		} catch (e) {
			if (e instanceof DOMException && e.name === "AbortError") return;
			error = String(e ?? "Error loading local items");
		} finally {
			if (!silent) {
				localAbortController = null;
				loading = false;
			}
		}
	}

	async function searchRemoteModrinth(reset = false) {
		if (loading) return;

		if (reset) {
			resetPagination();
		} else if (!hasMore || loadingMore) {
			return;
		}

		if (reset) {
			loading = true;
		} else {
			loadingMore = true;
		}
		error = null;
		abortPending();
		abortController = new AbortController();
		const signal = abortController.signal;

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
				signal,
				projectType,
			);

			if (!result) return;

			const mapped = result.hits.map((hit) => {
				const project = modrinthProjectToMarket(hit);
				if (project.modrinthProjectId) {
					const local = localModsById.get(project.modrinthProjectId);
					if (local) project.installed = local;
				}
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
			if (e instanceof DOMException && e.name === "AbortError") return;
			error = String(e ?? "Error searching Modrinth");
		} finally {
			loading = false;
			loadingMore = false;
			abortController = null;
		}
	}

	async function searchRemoteCurseForge(reset = false) {
		if (!isModContent) return;
		if (loading) return;

		if (reset) {
			resetPagination();
		} else if (!hasMore || loadingMore) {
			return;
		}

		if (reset) {
			loading = true;
		} else {
			loadingMore = true;
		}
		error = null;
		abortPending();
		abortController = new AbortController();
		const signal = abortController.signal;

		try {
			const category = null;
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
				signal,
			);

			if (!result) return;

			const mapped = result.data.map((hit) => {
				const project = curseforgeProjectToMarket(hit);
				if (project.curseforgeProjectId) {
					const local = localModsById.get(project.curseforgeProjectId);
					if (local) project.installed = local;
				}
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
			if (e instanceof DOMException && e.name === "AbortError") return;
			error = String(e ?? "Error searching CurseForge");
		} finally {
			loading = false;
			loadingMore = false;
			abortController = null;
		}
	}

	function performSearch(reset = false) {
		if (filters.source === "local") {
			return loadLocalItems();
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
				const [full, files] = await Promise.all([
					getCurseForgeProject(Number(projectId)),
					getCurseForgeProjectFiles(
						Number(projectId),
						filters.loader,
						filters.gameVersion,
					),
				]);

				if (full) {
					detail.fullProject = full;
				}

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

	function selectedVersion(): MarketVersion | null {
		if (detail.versions.length === 0) return null;

		if (overrideVersionId) {
			const overridden = detail.versions.find(
				(v) => v.id === overrideVersionId,
			);
			if (overridden) return overridden;
		}

		const installed = detail.versions.find((v) => v.isInstalled);
		if (installed) return installed;

		const compatible = detail.versions.find(
			(v) =>
				v.gameVersions.includes(filters.gameVersion) &&
				v.loaders.includes(filters.loader),
		);
		if (compatible) return compatible;

		return detail.versions[0];
	}

	function setSelectedVersion(version: MarketVersion) {
		overrideVersionId = version.id;
	}

	function isVersionCompatible(version: MarketVersion): boolean {
		return (
			version.gameVersions.includes(filters.gameVersion) &&
			version.loaders.includes(filters.loader)
		);
	}

	function isInstanceBusy() {
		return instance.status === InstState.Started || instance.status === InstState.Starting;
	}

	async function install(project: MarketProject, version: MarketVersion) {
		if (isInstanceBusy()) {
			showWarning(t("errors.title"), t("errors.INST_BUSY"));
			return;
		}
		let fileUrl = version.primaryFileUrl;

		if (project.source === "curseforge") {
			const cfProjectId = project.curseforgeProjectId ?? project.id;
			if (!fileUrl) {
				fileUrl =
					(await getCurseForgeFileDownloadUrl(
						Number(cfProjectId),
						Number(version.id),
					)) ?? "";
			}
			if (!fileUrl) return;

			try {
				await downloadFn(instance.uuid, [
					{
						url: fileUrl,
						filename: version.primaryFileName,
						project_id: cfProjectId,
						version_id: version.id,
						headers: CURSEFORGE_HEADERS,
					},
				]);

				if (filters.source === "local") {
					await loadLocalItems();
				}

				const current =
					items.find((i) => i.id === project.id) ?? project;
				await loadDetail(current);
			} catch (e) {
				console.error(e);
				throw e;
			}
			return;
		}

		const projectId = project.modrinthProjectId ?? project.id;
		if (!fileUrl) return;

		try {
			await downloadFn(instance.uuid, [
				{
					url: fileUrl,
					filename: version.primaryFileName,
					project_id: projectId,
					version_id: version.id,
				},
			]);

			if (filters.source === "local") {
				await loadLocalItems();
			}

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
				if (
					item.id === project.id ||
					item.modrinthProjectId === project.modrinthProjectId ||
					item.curseforgeProjectId === project.curseforgeProjectId
				) {
					item.installed = undefined;
					item.installedVersion = undefined;
					item.modrinthVersionId = undefined;
					item.modrinthProjectId = undefined;
					item.curseforgeVersionId = undefined;
					item.curseforgeProjectId = undefined;
				}
			}
			if (filters.source === "local") {
				await loadLocalItems();
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
			// Update enabled state in all matching items
			for (const item of items) {
				if (
					item.installed &&
					(item.id === project.id ||
						item.modrinthProjectId === project.modrinthProjectId ||
						item.curseforgeProjectId ===
							project.curseforgeProjectId)
				) {
					item.installed.enabled = newEnabled;
					item.disabled = !newEnabled;
				}
			}
			if (filters.source === "local") {
				await loadLocalItems();
			}
			if (selectedId) {
				await loadDetail(project);
			}
		} catch (e) {
			console.error(e);
		}
	}

	function loadMore() {
		if (filters.source !== "local" && hasMore && !loading && !loadingMore) {
			performSearch(false);
		}
	}

	function setSource(source: MarketSource) {
		if (source === "curseforge" && !isModContent) return;
		filters.source = source;
		selectedId = null;
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => performSearch(true), 200);
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
			loadLocalItems();
		}
	}

	// Watch instance changes and reset
	let lastInstanceId = "";
	$effect(() => {
		if (instance.uuid !== lastInstanceId) {
			lastInstanceId = instance.uuid;
		resetState();
		localModsById = new Map();
		loadLocalItems(true);
		performSearch(true);
		}
	});

	// Auto-refresh local items when background enrichment completes
	const _unregisterRefresh = registerModsRefreshCallback(
		instance.uuid,
		() => {
			if (filters.source === "local") {
				loadLocalItems(true);
			}
		},
	);

	function destroy() {
		abortPending();
		localAbortController?.abort();
		localAbortController = null;
		clearTimeout(searchTimer);
		searchTimer = undefined;
		_unregisterRefresh();

		items.length = 0;
		total = 0;
		selectedId = null;
		overrideVersionId = null;
		detail.fullProject = undefined;
		detail.versions = [];
		detail.loading = false;
		detail.error = null;
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
			return loading;
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
		selectedVersion,
		setSelectedVersion,
		isVersionCompatible,
		setSource,
		setQuery,
		setCategory,
		setSort,
		setLocalSort,
		selectProject,
		loadMore,
		install,
		uninstall,
		toggleEnabled,
		refresh: () => performSearch(true),
		destroy,
	};
}
