import {
	deleteInstanceFile,
	getInstanceMods,
	getInstanceModsMetadata,
	getModrinthProject,
	getModrinthProjectVersions,
	searchModrinth,
	toggleInstanceMod,
	downloadMods,
} from "$lib/api/cubicApi";
import {
	localModToMarket,
	modrinthProjectToMarket,
	modrinthVersionToMarket,
	parseInstanceVersion,
	type MarketProject,
	type MarketVersion,
} from "$lib/types/market";
import type { InstanceDto, ModDto, ModrinthProjectFull } from "$lib/types/types";

const PAGE_SIZE = 20;

export type MarketSource = "local" | "remote";

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
	fullProject?: ModrinthProjectFull;
	versions: MarketVersion[];
	loading: boolean;
	error: string | null;
}

export function createMarketState(instance: InstanceDto) {
	const parsed = parseInstanceVersion(instance);

	const filters = $state<MarketFilters>({
		source: "remote",
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
		filters.source = "remote";
		filters.query = "";
		filters.loader = fresh.loader.toLowerCase();
		filters.gameVersion = fresh.gameVersion;
		filters.category = null;
		filters.sort = "downloads";
		resetPagination();
		selectedId = null;
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
			const [mods, metadata] = await Promise.all([
				getInstanceMods(instance.uuid),
				getInstanceModsMetadata(instance.uuid),
			]);

			const mapped = mods.map((mod) => {
				const meta = metadata?.[mod.filename];
				return localModToMarket(mod, meta);
			});

			// Enrich local mods that have a projectId with Modrinth data
			const ids = [
				...new Set(
					mapped
						.filter((m) => m.modrinthProjectId)
						.map((m) => m.modrinthProjectId!),
				),
			];
			if (ids.length > 0) {
				const projects = await Promise.all(
					ids.map((id) => getModrinthProject(id)),
				);
				const projectMap = new Map<string, ModrinthProjectFull>();
				for (const p of projects) {
					if (p) projectMap.set(p.id, p);
				}
				for (const item of mapped) {
					if (item.modrinthProjectId) {
						const full = projectMap.get(item.modrinthProjectId);
						if (full) {
							item.title = full.title;
							item.description = full.description;
							item.icon = full.icon_url ?? item.icon;
							item.downloadCount = full.downloads;
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
			error = String(e ?? "Error loading local mods");
		} finally {
			loading = false;
		}
	}

	async function searchRemote(reset = false) {
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

			const result = await searchModrinth(
				filters.query,
				filters.loader,
				filters.gameVersion,
				category,
				index,
				PAGE_SIZE,
				currentOffset,
				signal,
				"mod",
			);

			if (!result) return;

			// Load local metadata to mark installed mods by project id
			const metadata = await getInstanceModsMetadata(instance.uuid);
			const localByProjectId = new Map<string, { mod: ModDto; versionId?: string }>();
			for (const [filename, meta] of Object.entries(metadata ?? {})) {
				localByProjectId.set(meta.project_id, {
					mod: { name: filename, filename, version: meta.version_id, enabled: true } as ModDto,
					versionId: meta.version_id,
				});
			}

			// Load instance mods for filename-based fallback matching
			const instanceMods = await getInstanceMods(instance.uuid);
			const modsByFilename = new Map<string, ModDto>();
			for (const m of instanceMods ?? []) {
				modsByFilename.set(m.filename.toLowerCase(), m);
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

			function nameMatchesSearch(name: string, hitTitle: string, hitSlug: string): boolean {
				const norm = name.toLowerCase();
				const title = hitTitle.toLowerCase();
				const slug = hitSlug.toLowerCase();
				return title.includes(norm) || slug.includes(norm) || norm.includes(title) || norm.includes(slug);
			}

			const mapped = result.hits.map((hit) => {
				const market = modrinthProjectToMarket(hit);
				const local = localByProjectId.get(hit.project_id);
				if (local) {
					market.installed = local.mod;
					market.installedVersion = local.versionId;
					market.modrinthProjectId = hit.project_id;
					market.modrinthVersionId = local.versionId;
					return market;
				}
				// Fallback: match by installed filename → project title/slug
				for (const [filename, installedMod] of modsByFilename) {
					const modName = modNameFromFilename(filename);
					if (modName && nameMatchesSearch(modName, hit.title, hit.slug)) {
						market.installed = installedMod;
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

	function performSearch(reset = false) {
		if (filters.source === "local") {
			return loadLocalItems();
		}
		return searchRemote(reset);
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
		detail.fullProject = undefined;
		detail.versions = [];

		const projectId = project.modrinthProjectId ?? (project.source === "modrinth" ? project.id : undefined);
		if (!projectId) {
			detail.loading = false;
			return;
		}

		try {
			const [full, versions] = await Promise.all([
				getModrinthProject(projectId),
				getModrinthProjectVersions(projectId, filters.loader, filters.gameVersion),
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

	function isVersionCompatible(version: MarketVersion): boolean {
		return (
			version.gameVersions.includes(filters.gameVersion) &&
			version.loaders.includes(filters.loader)
		);
	}

	async function install(project: MarketProject, version: MarketVersion) {
		if (!version?.primaryFileUrl) return;

		const projectId = project.modrinthProjectId ?? project.id;
		try {
			await downloadMods(instance.uuid, [
				{
					url: version.primaryFileUrl,
					filename: version.primaryFileName,
					project_id: projectId,
					version_id: version.id,
				},
			]);

			// Mark installed on all matching items in-place
			for (const item of items) {
				if (item.id === project.id || item.modrinthProjectId === projectId) {
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

			// Reload detail for the current project
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
			await deleteInstanceFile(instance.uuid, "mods", project.installed.filename);
			// Remove installed state from all matching items
			for (const item of items) {
				if (item.id === project.id || item.modrinthProjectId === project.modrinthProjectId) {
					item.installed = undefined;
					item.installedVersion = undefined;
					item.modrinthVersionId = undefined;
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
		if (!project.installed) return;
		const newEnabled = !project.installed.enabled;
		try {
			await toggleInstanceMod(instance.uuid, project.installed.filename, newEnabled);
			// Update enabled state in all matching items
			for (const item of items) {
				if (item.installed && (item.id === project.id || item.modrinthProjectId === project.modrinthProjectId)) {
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
		if (filters.source === "remote" && hasMore && !loading && !loadingMore) {
			performSearch(false);
		}
	}

	function setSource(source: MarketSource) {
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
		get filters() { return filters; },
		get items() { return items; },
		get total() { return total; },
		get loading() { return loading; },
		get loadingMore() { return loadingMore; },
		get error() { return error; },
		get hasMore() { return hasMore; },
		get selectedId() { return selectedId; },
		get selectedProject() { return selectedProject; },
		get detail() { return detail; },
		selectedVersion,
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
