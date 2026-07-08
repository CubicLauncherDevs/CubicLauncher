import {
	deleteInstanceFile,
	getInstanceMods,
	getInstanceModsMetadata,
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
} from "$lib/api/cubicApi";
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

const PAGE_SIZE = 20;

export type MarketSource = "local" | "modrinth" | "curseforge";

export type MarketSort = "relevance" | "downloads" | "newest" | "updated";

export interface MarketFilters {
	source: MarketSource;
	query: string;
	loader: string;
	gameVersion: string;
	category: string | null;
	sort: MarketSort;
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
	});

	const items = $state<MarketProject[]>([]);
	let total = $state(0);
	let loading = $state(false);
	let loadingMore = $state(false);
	let error = $state<string | null>(null);
	let offset = $state(0);
	let hasMore = $state(true);
	let selectedId = $state<string | null>(null);
	const detail = $state<MarketDetailState>({
		versions: [],
		loading: false,
		error: null,
	});

	let overrideVersionId = $state<string | null>(null);
	let abortController: AbortController | null = null;
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
		filters.source = isModContent ? "modrinth" : "local";
		filters.query = "";
		filters.loader = fresh.loader.toLowerCase();
		filters.gameVersion = fresh.gameVersion;
		filters.category = null;
		filters.sort = "downloads";
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

	async function loadLocalItems() {
		loading = true;
		error = null;
		abortPending();

		try {
			const localItems = await localLoader(instance.uuid);

			const mapped = localItems.map((mod) => {
				return localModToMarket(mod, undefined);
			});

			// Enrich local mods that have a projectId with Modrinth data
			if (isModContent) {
				const metadata = await getInstanceModsMetadata(instance.uuid);
				for (const item of mapped) {
					const meta = metadata?.[item.installed?.filename ?? ""];
					if (meta) {
						item.modrinthProjectId = meta.project_id;
						item.modrinthVersionId = meta.version_id;
					}
				}

				const seen: Record<string, true> = {};
				const ids: string[] = [];
				for (const m of mapped) {
					const pid = m.modrinthProjectId;
					if (pid && !seen[pid]) {
						seen[pid] = true;
						ids.push(pid);
					}
				}
				if (ids.length > 0) {
					const projects = await Promise.all(
						ids.map((id) => getModrinthProject(id)),
					);
					const projectMap: Record<string, ModrinthProjectFull> = {};
					for (const p of projects) {
						if (p) projectMap[p.id] = p;
					}
					for (const item of mapped) {
						if (item.modrinthProjectId) {
							const full = projectMap[item.modrinthProjectId];
							if (full) {
								item.title = full.title;
								item.description = full.description;
								item.icon = full.icon_url ?? item.icon;
								item.downloadCount = full.downloads;
							}
						}
					}
				}
			}

			// Apply local search filter
			const query = filters.query.trim().toLowerCase();
			const filtered = query
				? mapped.filter(
						(m) =>
							m.title.toLowerCase().includes(query) ||
							m.description.toLowerCase().includes(query) ||
							m.author.toLowerCase().includes(query),
					)
				: mapped;

			items.length = 0;
			items.push(...filtered);
			total = filtered.length;
			hasMore = false;
		} catch (e) {
			error = String(e ?? "Error loading local items");
		} finally {
			loading = false;
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

			// Load local data to mark installed items
			const localItems = await localLoader(instance.uuid);
			const installedByFilename: Record<string, ModDto> = {};
			for (const item of localItems ?? []) {
				installedByFilename[item.filename.toLowerCase()] = item;
			}

			// Load metadata for mods to match by project_id
			const localByProjectId: Record<
				string,
				{ mod: ModDto; versionId?: string }
			> = {};
			if (isModContent) {
				const metadata = await getInstanceModsMetadata(instance.uuid);
				for (const [filename, meta] of Object.entries(metadata ?? {})) {
					localByProjectId[meta.project_id] = {
						mod: {
							name: filename,
							filename,
							version: meta.version_id,
							enabled: true,
						} as ModDto,
						versionId: meta.version_id,
					};
				}
			}

			function modNameFromFilename(filename: string): string {
				return filename
					.replace(/\.(jar|zip)$/i, "")
					.replace(/-mc\d[\w.-]*/gi, "")
					.replace(/-\d+[\w.-]+/g, "")
					.replace(/-(fabric|forge|neoforge|quilt|universal)/gi, "")
					.replace(/[-_]+/g, " ")
					.trim()
					.toLowerCase();
			}

			function nameMatchesSearch(
				name: string,
				hitTitle: string,
				hitSlug: string,
			): boolean {
				const norm = name.toLowerCase();
				const title = hitTitle.toLowerCase();
				const slug = hitSlug.toLowerCase();
				return (
					title.includes(norm) ||
					slug.includes(norm) ||
					norm.includes(title) ||
					norm.includes(slug)
				);
			}

			const mapped = result.hits.map((hit) => {
				const market = modrinthProjectToMarket(hit);
				const local = localByProjectId[hit.project_id];
				if (local) {
					market.installed = local.mod;
					market.installedVersion = local.versionId;
					market.modrinthProjectId = hit.project_id;
					market.modrinthVersionId = local.versionId;
					return market;
				}
				// Fallback: match by installed filename → project title/slug
				for (const [filename, installedItem] of Object.entries(
					installedByFilename,
				)) {
					const modName = modNameFromFilename(filename);
					if (
						modName &&
						nameMatchesSearch(modName, hit.title, hit.slug)
					) {
						market.installed = installedItem;
						market.modrinthProjectId = hit.project_id;
						break;
					}
				}
				return market;
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
			const category = null; // CurseForge uses numeric category IDs, skip for now
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

			// Load local metadata to mark installed mods
			const metadata = await getInstanceModsMetadata(instance.uuid);
			const localByProjectId: Record<
				string,
				{ mod: ModDto; versionId?: string }
			> = {};
			for (const [filename, meta] of Object.entries(metadata ?? {})) {
				localByProjectId[meta.project_id] = {
					mod: {
						name: filename,
						filename,
						version: meta.version_id,
						enabled: true,
					} as ModDto,
					versionId: meta.version_id,
				};
			}

			const instanceMods = await getInstanceMods(instance.uuid);
			const modsByFilename: Record<string, ModDto> = {};
			for (const m of instanceMods ?? []) {
				modsByFilename[m.filename.toLowerCase()] = m;
			}

			function modNameFromFilename(filename: string): string {
				return filename
					.replace(/\.jar$/i, "")
					.replace(/-mc\d[\w.-]*/gi, "")
					.replace(/-\d+[\w.-]+/g, "")
					.replace(/-(fabric|forge|neoforge|quilt|universal)/gi, "")
					.replace(/[-_]+/g, " ")
					.trim()
					.toLowerCase();
			}

			function nameMatchesSearch(
				name: string,
				hitTitle: string,
				hitSlug: string,
			): boolean {
				const norm = name.toLowerCase();
				const title = hitTitle.toLowerCase();
				const slug = hitSlug.toLowerCase();
				return (
					title.includes(norm) ||
					slug.includes(norm) ||
					norm.includes(title) ||
					norm.includes(slug)
				);
			}

			const mapped = result.data.map((hit) => {
				const market = curseforgeProjectToMarket(hit);
				const cfId = String(hit.id);
				const local = localByProjectId[cfId];
				if (local) {
					market.installed = local.mod;
					market.installedVersion = local.versionId;
					market.curseforgeProjectId = cfId;
					market.curseforgeVersionId = local.versionId;
					return market;
				}
				// Fallback: match by installed filename → project name/slug
				for (const [filename, installedMod] of Object.entries(
					modsByFilename,
				)) {
					const modName = modNameFromFilename(filename);
					if (
						modName &&
						nameMatchesSearch(modName, hit.name, hit.slug)
					) {
						market.installed = installedMod;
						market.curseforgeProjectId = cfId;
						break;
					}
				}
				return market;
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
			const [full, versions] = await Promise.all([
				getModrinthProject(projectId),
				getModrinthProjectVersions(
					projectId,
					filters.loader,
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

	async function install(project: MarketProject, version: MarketVersion) {
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
					},
				]);

				for (const item of items) {
					if (
						item.id === project.id ||
						item.curseforgeProjectId === cfProjectId
					) {
						item.installed = {
							name: version.primaryFileName,
							filename: version.primaryFileName,
							version: version.id,
							description: null,
							authors: null,
							icon: null,
							enabled: true,
						};
						item.installedVersion = version.id;
						item.curseforgeVersionId = version.id;
						item.curseforgeProjectId = cfProjectId;
					}
				}

				if (filters.source === "local") {
					await loadLocalItems();
				}

				const current =
					items.find((i) => i.id === project.id) ?? project;
				current.installedVersion = version.id;
				current.curseforgeVersionId = version.id;
				current.curseforgeProjectId = cfProjectId;
				await loadDetail(current);
			} catch (e) {
				console.error(e);
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

			for (const item of items) {
				if (
					item.id === project.id ||
					item.modrinthProjectId === projectId
				) {
					item.installed = {
						name: version.primaryFileName,
						filename: version.primaryFileName,
						version: version.id,
						description: null,
						authors: null,
						icon: null,
						enabled: true,
					};
					item.installedVersion = version.id;
					item.modrinthVersionId = version.id;
					item.modrinthProjectId = projectId;
				}
			}

			if (filters.source === "local") {
				await loadLocalItems();
			}

			const current = items.find((i) => i.id === project.id) ?? project;
			current.installedVersion = version.id;
			current.modrinthVersionId = version.id;
			current.modrinthProjectId = projectId;
			await loadDetail(current);
		} catch (e) {
			console.error(e);
		}
	}

	async function uninstall(project: MarketProject) {
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

	// Watch instance changes and reset
	let lastInstanceId = "";
	$effect(() => {
		if (instance.uuid !== lastInstanceId) {
			lastInstanceId = instance.uuid;
			resetState();
			performSearch(true);
		}
	});

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
		selectProject,
		loadMore,
		install,
		uninstall,
		toggleEnabled,
		refresh: () => performSearch(true),
	};
}
