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
	type MrpackInfo,
	type YggdrasilServerInfo,
} from "../types/types";

import { invoke } from "@tauri-apps/api/core";
import { showErrorParsed, showJreInstallPrompt } from "../state/state.svelte";
import {
	type VersionIntegrity,
	type VersionStatus,
} from "../utils/versionUtils";

// ─────────────────────────────────────────────────────────────
// Internal invoke helpers
// ─────────────────────────────────────────────────────────────

async function invokeWithFallback<T>(
	cmd: string,
	args?: Record<string, unknown> | null,
	fallback?: T,
): Promise<T | undefined> {
	try {
		return await invoke<T>(cmd, args ?? {});
	} catch (err) {
		showErrorParsed(err);
		return fallback;
	}
}

async function invokeWithCallback(
	cmd: string,
	args?: Record<string, unknown> | null,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<void> {
	try {
		await invoke(cmd, args ?? {});
		callback?.();
	} catch (err) {
		showErrorParsed(err);
		onError?.(err);
	}
}

async function invokeVoid(
	cmd: string,
	args?: Record<string, unknown> | null,
): Promise<void> {
	try {
		await invoke(cmd, args ?? {});
	} catch (err) {
		showErrorParsed(err);
	}
}

async function invokeThrowing(
	cmd: string,
	args?: Record<string, unknown> | null,
): Promise<void> {
	try {
		await invoke(cmd, args ?? {});
	} catch (err) {
		showErrorParsed(err);
		throw err;
	}
}

// ─────────────────────────────────────────────────────────────
// Settings
// ─────────────────────────────────────────────────────────────

export async function getSettings(): Promise<Settings | null> {
	return (await invokeWithFallback<Settings>("get_settings", null)) ?? null;
}

export async function updateSettings(settings: Settings): Promise<void> {
	return invokeVoid("update_settings", { newSettings: settings });
}

export async function getRecommendedRam(): Promise<{
	total_gb: number;
	recommended_gb: number;
}> {
	const fallback = { total_gb: 8, recommended_gb: 2 };
	return (
		(await invokeWithFallback<{ total_gb: number; recommended_gb: number }>(
			"get_recommended_ram",
			null,
			fallback,
		)) ?? fallback
	);
}

// ─────────────────────────────────────────────────────────────
// Instances
// ─────────────────────────────────────────────────────────────

export async function killInstance(
	uuid: string,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<void> {
	return invokeWithCallback("kill_instance", { uuid }, callback, onError);
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
	return invokeWithCallback("delete_instance", { id }, callback, onError);
}

export async function renameInstance(
	id: string,
	newName: string,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<void> {
	return invokeWithCallback(
		"rename_instance",
		{ id, newName },
		callback,
		onError,
	);
}

export async function updateInstance(
	id: string,
	newName?: string,
	newVersion?: string,
	newIcon?: string | null,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<void> {
	return invokeWithCallback(
		"update_instance",
		{ id, newName, newVersion, newIcon },
		callback,
		onError,
	);
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

export async function openInstanceDir(
	id: string,
	subDir?: string,
): Promise<void> {
	return invokeVoid("open_instance_dir", {
		id,
		subDir: subDir ?? null,
	});
}

export async function reinstallVersion(versionId: string): Promise<void> {
	return invokeVoid("reinstall_version", { version: versionId });
}

export async function resetInstanceIcon(instanceId: string): Promise<void> {
	return invokeVoid("reset_instance_icon", { instanceId });
}

export async function uploadCustomIcon(
	instanceId: string,
	sourcePath: string,
): Promise<string | null> {
	return (
		(await invokeWithFallback<string>("upload_custom_icon", {
			instanceId,
			sourcePath,
		})) ?? null
	);
}

// ─────────────────────────────────────────────────────────────
// Versions
// ─────────────────────────────────────────────────────────────

export async function getInstalledVersions(): Promise<string[]> {
	return (
		(await invokeWithFallback<string[]>("get_installed_versions", null)) ??
		[]
	);
}

export async function checkVersionIntegrity(
	versionId: string,
): Promise<VersionIntegrity | null> {
	return (
		(await invokeWithFallback<VersionIntegrity>("check_version_integrity", {
			versionId,
		})) ?? null
	);
}

export async function getInstalledVersionsWithStatus(): Promise<
	VersionStatus[]
> {
	return (
		(await invokeWithFallback<VersionStatus[]>(
			"get_installed_versions_with_status",
			null,
		)) ?? []
	);
}

export async function getAvailableVersions(): Promise<MinecraftVersion[]> {
	return (
		(await invokeWithFallback<MinecraftVersion[]>(
			"get_available_versions",
			null,
		)) ?? []
	);
}

export async function refreshAvailableVersions(): Promise<MinecraftVersion[]> {
	return (
		(await invokeWithFallback<MinecraftVersion[]>(
			"refresh_versions",
			null,
		)) ?? []
	);
}

export async function addToQueue(version: string): Promise<void> {
	return invokeVoid("add_to_queue", { version });
}

// ─────────────────────────────────────────────────────────────
// Loaders
// ─────────────────────────────────────────────────────────────

export async function getFabricVersions(): Promise<FabricGameVersion[]> {
	return (
		(await invokeWithFallback<FabricGameVersion[]>(
			"get_fabric_versions",
			null,
		)) ?? []
	);
}

export async function getFabricLoaderVersions(
	gameVersion: string,
): Promise<LoaderVersion[]> {
	return (
		(await invokeWithFallback<LoaderVersion[]>(
			"get_fabric_loader_versions",
			{ gameVersion },
		)) ?? []
	);
}

export async function downloadFabric(
	gameVersion: string,
	loaderVersion?: string,
): Promise<string | null> {
	return (
		(await invokeWithFallback<string>("download_fabric", {
			gameVersion,
			loaderVersion: loaderVersion ?? null,
		})) ?? null
	);
}

export async function getForgeVersions(): Promise<ForgeGameVersion[]> {
	return (
		(await invokeWithFallback<ForgeGameVersion[]>(
			"get_forge_versions",
			null,
		)) ?? []
	);
}

export async function refreshForgeVersions(): Promise<ForgeGameVersion[]> {
	return (
		(await invokeWithFallback<ForgeGameVersion[]>(
			"refresh_forge_versions",
			null,
		)) ?? []
	);
}

export async function downloadForge(
	gameVersion: string,
	forgeVersion: string,
): Promise<void> {
	return invokeVoid("download_forge", { gameVersion, forgeVersion });
}

export async function getNeoForgeVersions(): Promise<NeoForgeGameVersion[]> {
	return (
		(await invokeWithFallback<NeoForgeGameVersion[]>(
			"get_neoforge_versions",
			null,
		)) ?? []
	);
}

export async function refreshNeoForgeVersions(): Promise<
	NeoForgeGameVersion[]
> {
	return (
		(await invokeWithFallback<NeoForgeGameVersion[]>(
			"refresh_neoforge_versions",
			null,
		)) ?? []
	);
}

export async function downloadNeoForge(
	gameVersion: string,
	neoforgeVersion: string,
): Promise<string | null> {
	return (
		(await invokeWithFallback<string>("download_neoforge", {
			gameVersion,
			neoforgeVersion,
		})) ?? null
	);
}

export async function getQuiltVersions(): Promise<FabricGameVersion[]> {
	return (
		(await invokeWithFallback<FabricGameVersion[]>(
			"get_quilt_versions",
			null,
		)) ?? []
	);
}

export async function refreshQuiltVersions(): Promise<FabricGameVersion[]> {
	return (
		(await invokeWithFallback<FabricGameVersion[]>(
			"refresh_quilt_versions",
			null,
		)) ?? []
	);
}

export async function getQuiltLoaderVersions(
	gameVersion: string,
): Promise<LoaderVersion[]> {
	return (
		(await invokeWithFallback<LoaderVersion[]>(
			"get_quilt_loader_versions",
			{ gameVersion },
		)) ?? []
	);
}

export async function downloadQuilt(
	gameVersion: string,
	loaderVersion?: string,
): Promise<string | null> {
	return (
		(await invokeWithFallback<string>("download_quilt", {
			gameVersion,
			loaderVersion: loaderVersion ?? null,
		})) ?? null
	);
}

// ─────────────────────────────────────────────────────────────
// Mods / ResourcePacks / ShaderPacks
// ─────────────────────────────────────────────────────────────

export async function getInstanceMods(id: string): Promise<ModDto[]> {
	return (
		(await invokeWithFallback<ModDto[]>("get_instance_mods", { id })) ?? []
	);
}

export async function toggleInstanceMod(
	id: string,
	filename: string,
	enable: boolean,
): Promise<void> {
	return invokeVoid("toggle_instance_mod", { id, filename, enable });
}

export async function getInstanceResourcePacks(id: string): Promise<ModDto[]> {
	return (
		(await invokeWithFallback<ModDto[]>("get_instance_resourcepacks", {
			id,
		})) ?? []
	);
}

export async function getInstanceShaderPacks(
	instanceId: string,
): Promise<ModDto[]> {
	return (
		(await invokeWithFallback<ModDto[]>("get_instance_shaderpacks", {
			id: instanceId,
		})) ?? []
	);
}

export async function deleteInstanceFile(
	id: string,
	subDir: string,
	filename: string,
): Promise<void> {
	return invokeVoid("delete_instance_file", { id, subDir, filename });
}

export async function addInstanceFile(
	id: string,
	subDir: string,
	sourcePath: string,
): Promise<void> {
	return invokeThrowing("add_instance_file", { id, subDir, sourcePath });
}

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
	return invokeThrowing("download_mods", { instanceId, mods });
}

export async function downloadResourcePacks(
	instanceId: string,
	packs: ModDownloadInfo[],
): Promise<void> {
	return invokeThrowing("download_resourcepacks", { instanceId, packs });
}

export async function downloadShaderPacks(
	instanceId: string,
	packs: ModDownloadInfo[],
): Promise<void> {
	return invokeThrowing("download_shaderpacks", { instanceId, packs });
}

// ─────────────────────────────────────────────────────────────
// Mrpack
// ─────────────────────────────────────────────────────────────

export async function parseMrpack(path: string): Promise<MrpackInfo | null> {
	return (
		(await invokeWithFallback<MrpackInfo>("parse_mrpack", { path })) ?? null
	);
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

// ─────────────────────────────────────────────────────────────
// Market — Modrinth
// ─────────────────────────────────────────────────────────────

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
	return (
		(await invokeWithFallback<ModrinthSearchResult>("search_modrinth", {
			query,
			loader,
			gameVersion: gameVersion || null,
			category,
			index,
			limit,
			offset,
			projectType,
		})) ?? null
	);
}

export async function getModrinthProjectVersions(
	projectId: string,
	loader?: string,
	gameVersion?: string,
): Promise<ModrinthVersion[]> {
	return (
		(await invokeWithFallback<ModrinthVersion[]>(
			"get_modrinth_project_versions",
			{
				projectId,
				loader: loader || null,
				gameVersion: gameVersion || null,
			},
		)) ?? []
	);
}

export async function getModrinthProject(
	projectId: string,
): Promise<ModrinthProjectFull | null> {
	return (
		(await invokeWithFallback<ModrinthProjectFull>("get_modrinth_project", {
			projectId,
		})) ?? null
	);
}

export async function getModrinthVersion(
	versionId: string,
): Promise<ModrinthVersionFull | null> {
	return (
		(await invokeWithFallback<ModrinthVersionFull>("get_modrinth_version", {
			versionId,
		})) ?? null
	);
}

export async function getModrinthLatestVersions(
	hashes: string[],
	algorithm: string = "sha1",
	loaders?: string[],
	gameVersions?: string[],
): Promise<Record<string, ModrinthVersionFull> | null> {
	return (
		(await invokeWithFallback<Record<string, ModrinthVersionFull>>(
			"get_modrinth_latest_versions",
			{
				hashes,
				algorithm,
				loaders: loaders ?? [],
				gameVersions: gameVersions ?? [],
			},
		)) ?? null
	);
}

export async function downloadMrpack(
	url: string,
	versionId: string,
): Promise<string | null> {
	return (
		(await invokeWithFallback<string>("download_mrpack", {
			url,
			versionId,
		})) ?? null
	);
}

// ─────────────────────────────────────────────────────────────
// Market — CurseForge
// ─────────────────────────────────────────────────────────────

export const CURSEFORGE_HEADERS = {
	"x-api-key": "$2a$10$v4G8m2LV2QhjUu5l.G24Ieqdp4JTEEQ6bRsZjvpa0YncCVaDaqBP6",
};

export async function searchCurseForge(
	query: string,
	loader: string,
	gameVersion?: string,
	category?: string | null,
	index: string = "popularity",
	limit: number = 24,
	offset: number = 0,
): Promise<CurseForgeSearchResult | null> {
	return (
		(await invokeWithFallback<CurseForgeSearchResult>("search_curseforge", {
			query,
			loader,
			gameVersion: gameVersion || null,
			category: category || null,
			index,
			limit,
			offset,
		})) ?? null
	);
}

export async function getCurseForgeProject(
	modId: number,
): Promise<CurseForgeProject | null> {
	return (
		(await invokeWithFallback<CurseForgeProject>("get_curseforge_project", {
			modId,
		})) ?? null
	);
}

export async function getCurseForgeProjectFiles(
	modId: number,
	loader?: string,
	gameVersion?: string,
): Promise<CurseForgeFile[]> {
	return (
		(await invokeWithFallback<CurseForgeFile[]>(
			"get_curseforge_project_files",
			{
				modId,
				loader: loader || null,
				gameVersion: gameVersion || null,
			},
		)) ?? []
	);
}

export async function getCurseForgeFileDownloadUrl(
	modId: number,
	fileId: number,
): Promise<string | null> {
	return (
		(await invokeWithFallback<string>("get_curseforge_file_download_url", {
			modId,
			fileId,
		})) ?? null
	);
}

// ─────────────────────────────────────────────────────────────
// Auth
// ─────────────────────────────────────────────────────────────

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
	return (
		(await invokeWithFallback<MinecraftUser | null>(
			"get_current_user",
			null,
		)) ?? null
	);
}

export async function logout(): Promise<void> {
	return invokeVoid("logout");
}

export async function switchUser(idx: number): Promise<MinecraftUser | null> {
	return (
		(await invokeWithFallback<MinecraftUser>("switch_user", { idx })) ??
		null
	);
}

export async function removeUser(uuid: string): Promise<void> {
	return invokeVoid("remove_user", { uuid });
}

export async function getUserList(): Promise<MinecraftUser[]> {
	return (
		(await invokeWithFallback<MinecraftUser[]>("get_user_list", null)) ?? []
	);
}

// ─────────────────────────────────────────────────────────────
// Yggdrasil Auth
// ─────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────
// JRE
// ─────────────────────────────────────────────────────────────

export async function getJreStatus(version: number): Promise<JreStatus | null> {
	return (
		(await invokeWithFallback<JreStatus>("get_jre_status", { version })) ??
		null
	);
}

export async function getJreVersions(): Promise<JreStatus[]> {
	return (
		(await invokeWithFallback<JreStatus[]>("get_jre_versions", null)) ?? []
	);
}

export async function installJre(version: number): Promise<void> {
	return invokeThrowing("install_jre", { version });
}

export async function uninstallJre(version: number): Promise<void> {
	return invokeThrowing("uninstall_jre", { version });
}

// ─────────────────────────────────────────────────────────────
// Discord
// ─────────────────────────────────────────────────────────────

export async function initDiscordPresence(): Promise<void> {
	return invokeVoid("init_discord_presence");
}

export async function shutdownDiscordPresence(): Promise<void> {
	return invokeVoid("shutdown_discord_presence");
}

// ─────────────────────────────────────────────────────────────
// Misc
// ─────────────────────────────────────────────────────────────

export async function openUrl(url: string): Promise<void> {
	return invokeVoid("open_url", { url });
}

export async function getInstanceScreenshotDir(
	instanceId: string,
): Promise<string> {
	return await invoke<string>("get_instance_screenshot_dir", { instanceId });
}

export async function getDownloadQueue(): Promise<
	{
		version: string;
		status: string;
		current: number;
		total: number;
	}[]
> {
	return (
		(await invokeWithFallback<
			{
				version: string;
				status: string;
				current: number;
				total: number;
			}[]
		>("get_download_queue", null)) ?? []
	);
}
