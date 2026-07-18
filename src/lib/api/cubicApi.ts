import {
	type InstanceDto,
	type ModDto,
	type DeviceCode,
	type MinecraftUser,
	type Settings,
	type MinecraftVersion,
	type FabricGameVersion,
	type ForgeGameVersion,
	type NeoForgeGameVersion,
	type ModrinthSearchResult,
	type ModrinthVersion,
	type ModrinthProjectFull,
	type ModrinthVersionFull,
	type CurseForgeSearchResult,
	type CurseForgeFilesResult,
	type CurseForgeProject,
	type CurseForgeFile,
	type JreStatus,
	type McVersion,
	type MrpackInfo,
	type YggdrasilServerInfo,
} from "../types/types";
import { invoke } from "@tauri-apps/api/core";
import { showErrorParsed, showJreInstallPrompt } from "../state/state.svelte";
import { apiCache } from "../util/apiCache";

export async function killInstance(
	uuid: string,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<void> {
	try {
		await invoke("kill_instance", { uuid: uuid });
		callback?.();
	} catch (err) {
		showErrorParsed(err);
		onError?.(err);
	}
}
export async function createInstance(
	name: string,
	version: string,
	icon: string | null,
	callback?: (uuid: string) => void,
	onError?: (err: unknown) => void,
): Promise<void> {
	try {
		const uuid = await invoke<string>("create_instance", {
			name,
			version,
			icon,
		});
		callback?.(uuid);
	} catch (err) {
		showErrorParsed(err);
		onError?.(err);
	}
}

export async function deleteInstance(
	id: string,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<void> {
	try {
		await invoke("delete_instance", { id });
		callback?.();
	} catch (err) {
		showErrorParsed(err);
		onError?.(err);
	}
}

export async function renameInstance(
	id: string,
	newName: string,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<void> {
	try {
		await invoke("rename_instance", { id, newName });
		callback?.();
	} catch (err) {
		showErrorParsed(err);
		onError?.(err);
	}
}

export async function updateInstance(
	id: string,
	newName?: string,
	newVersion?: string,
	newIcon?: string | null,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<void> {
	try {
		await invoke("update_instance", { id, newName, newVersion, newIcon });
		callback?.();
	} catch (err) {
		showErrorParsed(err);
		onError?.(err);
	}
}

export async function getInstalledVersions(): Promise<string[]> {
	try {
		return await invoke<string[]>("get_installed_versions");
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export interface VersionIntegrity {
	version_id: string;
	dependencies: string[];
	missing: string[];
	complete: boolean;
}

export interface VersionStatus {
	version_id: string;
	complete: boolean;
	missing_deps: string[];
}

export async function checkVersionIntegrity(
	versionId: string,
): Promise<VersionIntegrity | null> {
	try {
		return await invoke<VersionIntegrity>("check_version_integrity", {
			versionId,
		});
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function getInstalledVersionsWithStatus(): Promise<
	VersionStatus[]
> {
	try {
		return await invoke<VersionStatus[]>(
			"get_installed_versions_with_status",
		);
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export function parseInstalledVersion(raw: string): McVersion {
	if (raw.includes("fabric")) {
		const clean = raw
			.replace(/^fabric-loader-[\d.]+-/, "")
			.replace(/-fabric$/, "");
		return { loader: "fabric", version: clean, type: "" };
	}
	if (raw.includes("-neoforge-")) {
		const idx = raw.indexOf("-neoforge-");
		const mcVersion = raw.substring(0, idx);
		const neoforgeVersion = raw.substring(idx + 10);
		return {
			loader: "neoforge",
			version: `${mcVersion}-neoforge-${neoforgeVersion}`,
			type: "",
		};
	}
	if (raw.includes("-forge-")) {
		const idx = raw.indexOf("-forge-");
		const mcVersion = raw.substring(0, idx);
		const forgeVersion = raw.substring(idx + 7);
		return {
			loader: "forge",
			version: `${mcVersion}-forge-${forgeVersion}`,
			type: "",
		};
	}
	if (raw.includes("quilt-loader-")) {
		const clean = raw.replace(/^quilt-loader-[\d.]+-/, "");
		return { loader: "quilt", version: clean, type: "" };
	}
	return { loader: "vanilla", version: raw, type: "" };
}

export function getInstalledMcVersions(raw: string[]): {
	vanilla: Set<string>;
	fabric: Set<string>;
	forge: Set<string>;
	neoforge: Set<string>;
	quilt: Set<string>;
} {
	const vanilla = new Set<string>();
	const fabric = new Set<string>();
	const forge = new Set<string>();
	const neoforge = new Set<string>();
	const quilt = new Set<string>();
	for (const v of raw) {
		const parsed = parseInstalledVersion(v);
		if (parsed.loader === "vanilla") vanilla.add(parsed.version);
		else if (parsed.loader === "fabric") fabric.add(parsed.version);
		else if (parsed.loader === "forge") forge.add(parsed.version);
		else if (parsed.loader === "neoforge") neoforge.add(parsed.version);
		else if (parsed.loader === "quilt") quilt.add(parsed.version);
	}
	return { vanilla, fabric, forge, neoforge, quilt };
}

export async function getInstanceMods(id: string): Promise<ModDto[]> {
	try {
		return await invoke<ModDto[]>("get_instance_mods", { id });
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function toggleInstanceMod(
	id: string,
	filename: string,
	enable: boolean,
): Promise<void> {
	try {
		await invoke("toggle_instance_mod", { id, filename, enable });
	} catch (err) {
		showErrorParsed(err);
	}
}

export async function launchInstance(
	instance: InstanceDto,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<void> {
	try {
		await invoke("launch", { instanceId: instance.uuid });
		callback?.();
	} catch (err) {
		const errorStr = err as string;
		try {
			const parsed = JSON.parse(errorStr);
			if (parsed.code === "INST_JRE_MISSING" && parsed.params?.version) {
				const version = parseInt(parsed.params.version, 10);
				if ([8, 17, 21, 25].includes(version)) {
					showJreInstallPrompt(version, instance);
					return;
				}
			}
		} catch {
			// JSON parse failed, fall through
		}
		showErrorParsed(err);
		onError?.(err);
	}
}

export async function fetchAll(
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<InstanceDto[]> {
	try {
		const dtos = await invoke<InstanceDto[]>("get_instances");
		callback?.();
		return dtos;
	} catch (err) {
		showErrorParsed(err);
		onError?.(err);
		return [];
	}
}

export async function getSettings(): Promise<Settings | null> {
	try {
		return await invoke<Settings>("get_settings");
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function updateSettings(settings: Settings): Promise<void> {
	try {
		await invoke("update_settings", { newSettings: settings });
	} catch (err) {
		showErrorParsed(err);
	}
}
export async function getAvailableVersions(): Promise<MinecraftVersion[]> {
	try {
		return await invoke<MinecraftVersion[]>("get_available_versions");
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function refreshAvailableVersions(): Promise<MinecraftVersion[]> {
	try {
		return await invoke<MinecraftVersion[]>("refresh_versions");
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function addToQueue(version: string): Promise<void> {
	try {
		await invoke("add_to_queue", { version });
	} catch (err) {
		showErrorParsed(err);
	}
}

export async function getFabricVersions(): Promise<FabricGameVersion[]> {
	try {
		return await invoke<FabricGameVersion[]>("get_fabric_versions");
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function getFabricLoaderVersions(
	gameVersion: string,
): Promise<string[]> {
	try {
		return await invoke<string[]>("get_fabric_loader_versions", {
			gameVersion,
		});
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function downloadFabric(
	gameVersion: string,
	loaderVersion?: string,
): Promise<string | null> {
	try {
		return await invoke<string>("download_fabric", {
			gameVersion,
			loaderVersion: loaderVersion ?? null,
		});
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function getForgeVersions(): Promise<ForgeGameVersion[]> {
	try {
		return await invoke<ForgeGameVersion[]>("get_forge_versions");
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function refreshForgeVersions(): Promise<ForgeGameVersion[]> {
	try {
		return await invoke<ForgeGameVersion[]>("refresh_forge_versions");
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function downloadForge(
	gameVersion: string,
	forgeVersion: string,
): Promise<void> {
	try {
		await invoke("download_forge", { gameVersion, forgeVersion });
	} catch (err) {
		showErrorParsed(err);
	}
}

export async function getNeoForgeVersions(): Promise<NeoForgeGameVersion[]> {
	try {
		return await invoke<NeoForgeGameVersion[]>("get_neoforge_versions");
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function refreshNeoForgeVersions(): Promise<
	NeoForgeGameVersion[]
> {
	try {
		return await invoke<NeoForgeGameVersion[]>("refresh_neoforge_versions");
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function downloadNeoForge(
	gameVersion: string,
	neoforgeVersion: string,
): Promise<string | null> {
	try {
		return await invoke<string>("download_neoforge", {
			gameVersion,
			neoforgeVersion,
		});
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function getQuiltVersions(): Promise<FabricGameVersion[]> {
	try {
		return await invoke<FabricGameVersion[]>("get_quilt_versions");
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function refreshQuiltVersions(): Promise<FabricGameVersion[]> {
	try {
		return await invoke<FabricGameVersion[]>("refresh_quilt_versions");
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function getQuiltLoaderVersions(
	gameVersion: string,
): Promise<string[]> {
	try {
		return await invoke<string[]>("get_quilt_loader_versions", {
			gameVersion,
		});
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function downloadQuilt(
	gameVersion: string,
	loaderVersion?: string,
): Promise<string | null> {
	try {
		return await invoke<string>("download_quilt", {
			gameVersion,
			loaderVersion: loaderVersion ?? null,
		});
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

// Auth Commands
export async function getDeviceCode(): Promise<DeviceCode> {
	return await invoke<DeviceCode>("get_device_code");
}

export async function authenticateWithDeviceCode(
	deviceCode: string,
	interval: number,
	expiresIn: number,
): Promise<MinecraftUser> {
	return await invoke<MinecraftUser>("authenticate_with_device_code", {
		deviceCode,
		interval,
		expiresIn,
	});
}

export async function startWebviewAuth(): Promise<MinecraftUser> {
	return await invoke<MinecraftUser>("start_webview_auth");
}

export async function getCurrentUser(): Promise<MinecraftUser | null> {
	return await invoke<MinecraftUser | null>("get_current_user");
}

export async function logout(): Promise<void> {
	await invoke("logout");
}

export async function switchUser(idx: number): Promise<void> {
	await invoke("switch_user", { idx });
}

export async function removeUser(username: string): Promise<void> {
	await invoke("remove_user", { username });
}

export async function getUserList(): Promise<MinecraftUser[]> {
	return await invoke<MinecraftUser[]>("get_user_list");
}

// Yggdrasil Auth Commands
export async function getYggdrasilServerInfo(
	url: string,
): Promise<YggdrasilServerInfo> {
	return await invoke<YggdrasilServerInfo>("get_yggdrasil_server_info", {
		url,
	});
}

export async function yggdrasilAuthenticate(
	serverUrl: string,
	username: string,
	password: string,
): Promise<MinecraftUser> {
	return await invoke<MinecraftUser>("yggdrasil_authenticate", {
		serverUrl,
		username,
		password,
	});
}

export async function initDiscordPresence(): Promise<void> {
	try {
		await invoke("init_discord_presence");
	} catch (err) {
		showErrorParsed(err);
	}
}

export async function shutdownDiscordPresence(): Promise<void> {
	try {
		await invoke("shutdown_discord_presence");
	} catch (err) {
		showErrorParsed(err);
	}
}

export async function openUrl(url: string): Promise<void> {
	await invoke("open_url", { url });
}

export async function getInstanceResourcePacks(id: string): Promise<ModDto[]> {
	try {
		return await invoke<ModDto[]>("get_instance_resourcepacks", { id });
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function deleteInstanceFile(
	id: string,
	subDir: string,
	filename: string,
): Promise<void> {
	try {
		await invoke("delete_instance_file", { id, subDir, filename });
	} catch (err) {
		showErrorParsed(err);
	}
}

export async function addInstanceFile(
	id: string,
	subDir: string,
	sourcePath: string,
): Promise<void> {
	try {
		await invoke("add_instance_file", { id, subDir, sourcePath });
	} catch (err) {
		showErrorParsed(err);
		throw err;
	}
}
export async function getDownloadQueue(): Promise<
	{
		version: string;
		status: string;
		current: number;
		total: number;
	}[]
> {
	try {
		return await invoke("get_download_queue");
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function searchModrinth(
	query: string,
	loader: string,
	gameVersion?: string,
	category: string | null = null,
	index: string = "downloads",
	limit: number = 24,
	offset: number = 0,
	signal?: AbortSignal,
	projectType: string = "mod",
): Promise<ModrinthSearchResult | null> {
	try {
		const facets = [];
		if (loader && loader.toLowerCase() !== "vanilla") {
			facets.push([`categories:${loader.toLowerCase()}`]);
		}
		if (gameVersion) {
			facets.push([`versions:${gameVersion}`]);
		}
		facets.push([`project_type:${projectType}`]);

		if (category) {
			facets.push([`categories:${category.toLowerCase()}`]);
		}

		const url = new URL("https://api.modrinth.com/v2/search");
		url.searchParams.append("query", query);
		url.searchParams.append("facets", JSON.stringify(facets));
		url.searchParams.append("index", index);
		url.searchParams.append("limit", limit.toString());
		url.searchParams.append("offset", offset.toString());

		const cacheKey = url.toString();
		if (!signal) {
			const cached = apiCache.get<ModrinthSearchResult>(cacheKey);
			if (cached) return cached;
		}

		const res = await fetch(url.toString(), { signal });
		if (!res.ok) {
			throw new Error(`Modrinth API error: ${res.status}`);
		}
		const data = (await res.json()) as ModrinthSearchResult;
		apiCache.set(cacheKey, data);
		return data;
	} catch (err) {
		if (err instanceof DOMException && err.name === "AbortError")
			return null;
		showErrorParsed(err);
		return null;
	}
}

export async function getModrinthProjectVersions(
	projectId: string,
	loader?: string,
	gameVersion?: string,
	signal?: AbortSignal,
): Promise<ModrinthVersion[]> {
	try {
		const url = new URL(
			`https://api.modrinth.com/v2/project/${projectId}/version`,
		);
		url.searchParams.append("include_changelog", "false");
		if (loader) {
			url.searchParams.append(
				"loaders",
				JSON.stringify([loader.toLowerCase()]),
			);
		}
		if (gameVersion) {
			url.searchParams.append(
				"game_versions",
				JSON.stringify([gameVersion]),
			);
		}

		const cacheKey = url.toString();
		if (!signal) {
			const cached = apiCache.get<ModrinthVersion[]>(cacheKey);
			if (cached) return cached;
		}

		const res = await fetch(url.toString(), { signal });
		if (!res.ok) {
			throw new Error(`Modrinth API error: ${res.status}`);
		}
		const data = (await res.json()) as ModrinthVersion[];
		if (!signal) {
			apiCache.set(cacheKey, data);
		}
		return data;
	} catch (err) {
		if (err instanceof DOMException && err.name === "AbortError") {
			return [];
		}
		showErrorParsed(err);
		return [];
	}
}

export async function getModrinthProject(
	projectId: string,
): Promise<ModrinthProjectFull | null> {
	try {
		const url = `https://api.modrinth.com/v2/project/${projectId}`;
		const cached = apiCache.get<ModrinthProjectFull>(url);
		if (cached) return cached;

		const res = await fetch(url);
		if (!res.ok) throw new Error(`Modrinth API error: ${res.status}`);
		const data = (await res.json()) as ModrinthProjectFull;
		apiCache.set(url, data);
		return data;
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function getModrinthVersion(
	versionId: string,
): Promise<ModrinthVersionFull | null> {
	try {
		const res = await fetch(
			`https://api.modrinth.com/v2/version/${versionId}`,
		);
		if (!res.ok) throw new Error(`Modrinth API error: ${res.status}`);
		return (await res.json()) as ModrinthVersionFull;
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function getModrinthLatestVersions(
	hashes: string[],
	algorithm: string = "sha1",
	loaders?: string[],
	gameVersions?: string[],
): Promise<Record<string, ModrinthVersionFull> | null> {
	try {
		const res = await fetch(
			"https://api.modrinth.com/v2/version_files/update",
			{
				method: "POST",
				headers: { "content-type": "application/json" },
				body: JSON.stringify({
					hashes,
					algorithm,
					loaders: loaders ?? [],
					game_versions: gameVersions ?? [],
				}),
			},
		);
		if (!res.ok) throw new Error(`Modrinth API error: ${res.status}`);
		return (await res.json()) as Record<string, ModrinthVersionFull>;
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function downloadMrpack(
	url: string,
	versionId: string,
): Promise<string | null> {
	try {
		return await invoke<string>("download_mrpack", { url, versionId });
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function installMrpackWithUpstream(
	path: string,
	instanceName: string,
	projectId?: string,
	versionId?: string,
	iconUrl?: string,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<MrpackInfo | null> {
	try {
		const result = await invoke<MrpackInfo>("install_mrpack", {
			path,
			instanceName,
			projectId: projectId ?? null,
			modrinthVersionId: versionId ?? null,
			iconUrl: iconUrl ?? null,
		});
		callback?.();
		return result;
	} catch (err) {
		showErrorParsed(err);
		onError?.(err);
		return null;
	}
}

export async function getInstanceScreenshotDir(
	instanceId: string,
): Promise<string> {
	return await invoke<string>("get_instance_screenshot_dir", { instanceId });
}

const CURSEFORGE_API_BASE = "https://api.curseforge.com/v1";
const MINECRAFT_GAME_ID = 432;
const CURSEFORGE_API_KEY =
	"$2a$10$v4G8m2LV2QhjUu5l.G24Ieqdp4JTEEQ6bRsZjvpa0YncCVaDaqBP6";

export async function searchCurseForge(
	query: string,
	loader: string,
	gameVersion?: string,
	category?: string | null,
	index: string = "popularity",
	limit: number = 24,
	offset: number = 0,
	signal?: AbortSignal,
): Promise<CurseForgeSearchResult | null> {
	try {
		const apiKey = CURSEFORGE_API_KEY;

		const url = new URL(`${CURSEFORGE_API_BASE}/mods/search`);
		url.searchParams.append("gameId", MINECRAFT_GAME_ID.toString());
		if (query) url.searchParams.append("searchFilter", query);
		url.searchParams.append("pageSize", Math.min(limit, 50).toString());
		url.searchParams.append("index", offset.toString());
		url.searchParams.append("classId", "6");

		if (loader.toLowerCase() !== "vanilla") {
			url.searchParams.append(
				"modLoaderType",
				modLoaderNameToCurseForgeId(loader).toString(),
			);
		}
		if (gameVersion) {
			url.searchParams.append("gameVersion", gameVersion);
		}
		if (category) {
			url.searchParams.append("categoryId", category);
		}
		if (index === "downloads") {
			url.searchParams.append("sortOrder", "desc");
		} else if (index === "newest") {
			url.searchParams.append("sortField", "2");
			url.searchParams.append("sortOrder", "desc");
		} else {
			url.searchParams.append("sortOrder", "desc");
		}

		const cacheKey = url.toString();
		if (!signal) {
			const cached = apiCache.get<CurseForgeSearchResult>(cacheKey);
			if (cached) return cached;
		}

		const res = await fetch(url.toString(), {
			signal,
			headers: {
				"x-api-key": apiKey,
				Accept: "application/json",
			},
		});
		if (!res.ok) {
			throw new Error(`CurseForge API error: ${res.status}`);
		}
		const data = (await res.json()) as CurseForgeSearchResult;
		apiCache.set(cacheKey, data);
		return data;
	} catch (err) {
		if (err instanceof DOMException && err.name === "AbortError")
			return null;
		showErrorParsed(err);
		return null;
	}
}

function modLoaderNameToCurseForgeId(loader: string): number {
	switch (loader.toLowerCase()) {
		case "fabric":
			return 4;
		case "forge":
			return 1;
		case "neoforge":
			return 6;
		case "quilt":
			return 5;
		default:
			return 4;
	}
}

export async function getCurseForgeProject(
	modId: number,
): Promise<CurseForgeProject | null> {
	try {
		const apiKey = CURSEFORGE_API_KEY;
		const url = `${CURSEFORGE_API_BASE}/mods/${modId}`;
		const cached = apiCache.get<CurseForgeProject>(url);
		if (cached) return cached;

		const res = await fetch(url, {
			headers: {
				"x-api-key": apiKey,
				Accept: "application/json",
			},
		});
		if (!res.ok) {
			throw new Error(`CurseForge API error: ${res.status}`);
		}
		const body = await res.json();
		const data = body.data as CurseForgeProject;
		apiCache.set(url, data);
		return data;
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function getCurseForgeProjectFiles(
	modId: number,
	loader?: string,
	gameVersion?: string,
	signal?: AbortSignal,
): Promise<CurseForgeFile[]> {
	try {
		const apiKey = CURSEFORGE_API_KEY;
		const url = new URL(`${CURSEFORGE_API_BASE}/mods/${modId}/files`);
		url.searchParams.append("pageSize", "100");

		if (gameVersion) {
			url.searchParams.append("gameVersion", gameVersion);
		}
		if (loader && loader.toLowerCase() !== "vanilla") {
			url.searchParams.append(
				"modLoaderType",
				modLoaderNameToCurseForgeId(loader).toString(),
			);
		}

		const cacheKey = url.toString();
		if (!signal) {
			const cached = apiCache.get<CurseForgeFile[]>(cacheKey);
			if (cached) return cached;
		}

		const res = await fetch(url.toString(), {
			signal,
			headers: {
				"x-api-key": apiKey,
				Accept: "application/json",
			},
		});
		if (!res.ok) {
			throw new Error(`CurseForge API error: ${res.status}`);
		}
		const body = (await res.json()) as CurseForgeFilesResult;
		const data = body.data || [];
		if (!signal) {
			apiCache.set(cacheKey, data);
		}
		return data;
	} catch (err) {
		if (err instanceof DOMException && err.name === "AbortError") {
			return [];
		}
		showErrorParsed(err);
		return [];
	}
}

export async function getCurseForgeFileDownloadUrl(
	modId: number,
	fileId: number,
): Promise<string | null> {
	try {
		const apiKey = CURSEFORGE_API_KEY;
		const res = await fetch(
			`${CURSEFORGE_API_BASE}/mods/${modId}/files/${fileId}/download-url`,
			{
				headers: {
					"x-api-key": apiKey,
					Accept: "application/json",
				},
			},
		);
		if (!res.ok) {
			throw new Error(`CurseForge API error: ${res.status}`);
		}
		const body = await res.json();
		return body.data?.downloadUrl as string | null;
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export const CURSEFORGE_HEADERS = {
	"x-api-key": CURSEFORGE_API_KEY,
};

export interface ModDownloadInfo {
	url: string;
	filename: string;
	projectTitle?: string;
	iconUrl?: string;
	project_id?: string;
	version_id?: string;
	headers?: Record<string, string>;
}

export async function downloadMods(
	instanceId: string,
	mods: ModDownloadInfo[],
): Promise<void> {
	try {
		await invoke("download_mods", {
			instanceId,
			mods,
		});
	} catch (err) {
		showErrorParsed(err);
		throw err;
	}
}

export async function downloadResourcePacks(
	instanceId: string,
	packs: ModDownloadInfo[],
): Promise<void> {
	try {
		await invoke("download_resourcepacks", {
			instanceId,
			packs,
		});
	} catch (err) {
		showErrorParsed(err);
		throw err;
	}
}

export async function getInstanceShaderPacks(
	instanceId: string,
): Promise<ModDto[]> {
	try {
		return await invoke<ModDto[]>("get_instance_shaderpacks", {
			id: instanceId,
		});
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function downloadShaderPacks(
	instanceId: string,
	packs: ModDownloadInfo[],
): Promise<void> {
	try {
		await invoke("download_shaderpacks", {
			instanceId,
			packs,
		});
	} catch (err) {
		showErrorParsed(err);
		throw err;
	}
}

export async function getJreStatus(version: number): Promise<JreStatus | null> {
	try {
		return await invoke<JreStatus>("get_jre_status", { version });
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function getJreVersions(): Promise<JreStatus[]> {
	try {
		return await invoke<JreStatus[]>("get_jre_versions");
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function installJre(version: number): Promise<void> {
	try {
		await invoke("install_jre", { version });
	} catch (err) {
		showErrorParsed(err);
		throw err;
	}
}

export async function uninstallJre(version: number): Promise<void> {
	try {
		await invoke("uninstall_jre", { version });
	} catch (err) {
		showErrorParsed(err);
		throw err;
	}
}

export async function parseMrpack(path: string): Promise<MrpackInfo | null> {
	try {
		return await invoke<MrpackInfo>("parse_mrpack", { path });
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function installMrpack(
	path: string,
	instanceName: string,
	iconUrl?: string,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<MrpackInfo | null> {
	try {
		const result = await invoke<MrpackInfo>("install_mrpack", {
			path,
			instanceName,
			iconUrl: iconUrl ?? null,
		});
		callback?.();
		return result;
	} catch (err) {
		showErrorParsed(err);
		onError?.(err);
		return null;
	}
}

export async function uploadCustomIcon(
	instanceId: string,
	sourcePath: string,
): Promise<string | null> {
	try {
		return await invoke<string>("upload_custom_icon", {
			instanceId,
			sourcePath,
		});
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function resetInstanceIcon(instanceId: string): Promise<void> {
	try {
		await invoke("reset_instance_icon", { instanceId });
	} catch (err) {
		showErrorParsed(err);
	}
}

export async function reinstallVersion(versionId: string) {
	invoke("reinstall_version", { version: versionId });
}

export async function openInstanceDir(
	id: string,
	subDir?: string,
): Promise<void> {
	await invoke("open_instance_dir", { id, subDir: subDir ?? null });
}
