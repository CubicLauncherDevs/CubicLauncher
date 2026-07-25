import {
	type InstanceDto,
	type ModDto,
	type DeviceCode,
	type MinecraftUser,
	type Settings,
	type MinecraftVersion,
	type FabricGameVersion,
	type LoaderVersion,
	type ForgeGameVersion,
	type NeoForgeGameVersion,
	type ModrinthSearchResult,
	type ModrinthVersion,
	type ModrinthProjectFull,
	type ModrinthVersionFull,
	type CurseForgeSearchResult,
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

function addInstalledLoaderVersion(
	map: Map<string, Set<string>>,
	loader: string,
	mcVersion: string,
	loaderVersion: string,
) {
	const key = `${loader}:${mcVersion}`;
	if (!map.has(key)) {
		map.set(key, new Set<string>());
	}
	map.get(key)!.add(loaderVersion);
}

/**
 * Devuelve un mapa con las versiones de loader instaladas, agrupadas por
 * loader y versión de Minecraft. La clave es `${loader}:${mcVersion}` y el
 * valor es el conjunto de versiones del loader instaladas para esa
 * combinación.
 */
export function getInstalledLoaderVersions(
	raw: string[],
): Map<string, Set<string>> {
	const result = new Map<string, Set<string>>();

	for (const v of raw) {
		if (v.includes("fabric-loader-")) {
			const clean = v.replace(/-fabric$/, "");
			const prefix = "fabric-loader-";
			const rest = clean.substring(prefix.length);
			const dash = rest.indexOf("-");
			if (dash === -1) continue;
			const loaderVersion = rest.substring(0, dash);
			const mcVersion = rest.substring(dash + 1);
			addInstalledLoaderVersion(
				result,
				"fabric",
				mcVersion,
				loaderVersion,
			);
		} else if (v.includes("quilt-loader-")) {
			const prefix = "quilt-loader-";
			const rest = v.substring(prefix.length);
			const dash = rest.indexOf("-");
			if (dash === -1) continue;
			const loaderVersion = rest.substring(0, dash);
			const mcVersion = rest.substring(dash + 1);
			addInstalledLoaderVersion(
				result,
				"quilt",
				mcVersion,
				loaderVersion,
			);
		} else if (v.includes("-neoforge-")) {
			const idx = v.indexOf("-neoforge-");
			const mcVersion = v.substring(0, idx);
			const loaderVersion = v.substring(idx + 10);
			addInstalledLoaderVersion(
				result,
				"neoforge",
				mcVersion,
				loaderVersion,
			);
		} else if (v.includes("-forge-")) {
			const idx = v.indexOf("-forge-");
			const mcVersion = v.substring(0, idx);
			const loaderVersion = v.substring(idx + 7);
			addInstalledLoaderVersion(
				result,
				"forge",
				mcVersion,
				loaderVersion,
			);
		}
	}

	return result;
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
): Promise<LoaderVersion[]> {
	try {
		return await invoke<LoaderVersion[]>("get_fabric_loader_versions", {
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
): Promise<LoaderVersion[]> {
	try {
		return await invoke<LoaderVersion[]>("get_quilt_loader_versions", {
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

export async function switchUser(idx: number): Promise<MinecraftUser | null> {
	try {
		return await invoke<MinecraftUser>("switch_user", { idx });
	} catch {
		return null;
	}
}

export async function removeUser(uuid: string): Promise<void> {
	await invoke("remove_user", { uuid });
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
	projectType: string = "mod",
): Promise<ModrinthSearchResult | null> {
	try {
		return await invoke<ModrinthSearchResult>("search_modrinth", {
			query,
			loader,
			gameVersion: gameVersion || null,
			category,
			index,
			limit,
			offset,
			projectType,
		});
	} catch (err) {
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
		const result = await invoke<ModrinthVersion[]>(
			"get_modrinth_project_versions",
			{
				projectId,
				loader: loader || null,
				gameVersion: gameVersion || null,
			},
		);
		return result ?? [];
	} catch (err) {
		showErrorParsed(err);
		return [];
	}
}

export async function getModrinthProject(
	projectId: string,
): Promise<ModrinthProjectFull | null> {
	try {
		return await invoke<ModrinthProjectFull>("get_modrinth_project", {
			projectId,
		});
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function getModrinthVersion(
	versionId: string,
): Promise<ModrinthVersionFull | null> {
	try {
		return await invoke<ModrinthVersionFull>("get_modrinth_version", {
			versionId,
		});
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
		return await invoke<Record<string, ModrinthVersionFull>>(
			"get_modrinth_latest_versions",
			{
				hashes,
				algorithm,
				loaders: loaders ?? [],
				gameVersions: gameVersions ?? [],
			},
		);
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

export async function searchCurseForge(
	query: string,
	loader: string,
	gameVersion?: string,
	category?: string | null,
	index: string = "popularity",
	limit: number = 24,
	offset: number = 0,
): Promise<CurseForgeSearchResult | null> {
	try {
		return await invoke<CurseForgeSearchResult>("search_curseforge", {
			query,
			loader,
			gameVersion: gameVersion || null,
			category: category || null,
			index,
			limit,
			offset,
		});
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export async function getCurseForgeProject(
	modId: number,
): Promise<CurseForgeProject | null> {
	try {
		return await invoke<CurseForgeProject>("get_curseforge_project", {
			modId,
		});
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
		const result = await invoke<CurseForgeFile[]>(
			"get_curseforge_project_files",
			{
				modId,
				loader: loader || null,
				gameVersion: gameVersion || null,
			},
		);
		return result ?? [];
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
		const result = await invoke<string>(
			"get_curseforge_file_download_url",
			{
				modId,
				fileId,
			},
		);
		return result || null;
	} catch (err) {
		showErrorParsed(err);
		return null;
	}
}

export const CURSEFORGE_HEADERS = {
	"x-api-key": "$2a$10$v4G8m2LV2QhjUu5l.G24Ieqdp4JTEEQ6bRsZjvpa0YncCVaDaqBP6",
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
