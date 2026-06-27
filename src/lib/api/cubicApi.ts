import {
	type InstanceDto,
	type ModDto,
	type DeviceCode,
	type MinecraftUser,
	type Settings,
	type MinecraftVersion,
	type FabricGameVersion,
	type ForgeGameVersion,
	type ModrinthSearchResult,
	type ModrinthVersion,
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
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<void> {
	try {
		await invoke("create_instance", { name, version, icon });
		callback?.();
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
	return { loader: "vanilla", version: raw, type: "" };
}

export function getInstalledMcVersions(raw: string[]): {
	vanilla: Set<string>;
	fabric: Set<string>;
	forge: Set<string>;
} {
	const vanilla = new Set<string>();
	const fabric = new Set<string>();
	const forge = new Set<string>();
	for (const v of raw) {
		const parsed = parseInstalledVersion(v);
		if (parsed.loader === "vanilla") vanilla.add(parsed.version);
		else if (parsed.loader === "fabric") fabric.add(parsed.version);
		else if (parsed.loader === "forge") forge.add(parsed.version);
	}
	return { vanilla, fabric, forge };
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

export async function downloadFabric(
	gameVersion: string,
	loaderVersion?: string,
): Promise<void> {
	try {
		await invoke("download_fabric", {
			gameVersion,
			loaderVersion: loaderVersion ?? null,
		});
	} catch (err) {
		showErrorParsed(err);
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

		const res = await fetch(url.toString(), { signal });
		if (!res.ok) {
			throw new Error(`Modrinth API error: ${res.status}`);
		}
		return (await res.json()) as ModrinthSearchResult;
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
): Promise<ModrinthVersion[]> {
	try {
		const url = new URL(
			`https://api.modrinth.com/v2/project/${projectId}/version`,
		);
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

		const res = await fetch(url.toString());
		if (!res.ok) {
			throw new Error(`Modrinth API error: ${res.status}`);
		}
		return (await res.json()) as ModrinthVersion[];
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
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
		} else if (index === "updated") {
			url.searchParams.append("sortField", "3");
			url.searchParams.append("sortOrder", "desc");
		} else {
			url.searchParams.append("sortOrder", "desc");
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
		return (await res.json()) as CurseForgeSearchResult;
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
		const res = await fetch(`${CURSEFORGE_API_BASE}/mods/${modId}`, {
			headers: {
				"x-api-key": apiKey,
				Accept: "application/json",
			},
		});
		if (!res.ok) {
			throw new Error(`CurseForge API error: ${res.status}`);
		}
		const body = await res.json();
		return body.data as CurseForgeProject;
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function getCurseForgeProjectFiles(
	modId: number,
	loader?: string,
	gameVersion?: string,
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

		const res = await fetch(url.toString(), {
			headers: {
				"x-api-key": apiKey,
				Accept: "application/json",
			},
		});
		if (!res.ok) {
			throw new Error(`CurseForge API error: ${res.status}`);
		}
		const body = (await res.json()) as CurseForgeFilesResult;
		return body.data || [];
	} catch (err) {
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

export interface ModDownloadInfo {
	url: string;
	filename: string;
	projectTitle?: string;
	iconUrl?: string;
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
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<MrpackInfo | null> {
	try {
		const result = await invoke<MrpackInfo>("install_mrpack", {
			path,
			instanceName,
		});
		callback?.();
		return result;
	} catch (err) {
		showErrorParsed(err);
		onError?.(err);
		return null;
	}
}
