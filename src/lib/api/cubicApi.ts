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
	type CurseForgeModpackInfo,
	type JreStatus,
	type MrpackInfo,
	type InstanceImportPlan,
	type YggdrasilServerInfo,
	type MinecraftProfileResponse,
	type SkinClosetEntry,
} from "../types/types";

import { invoke } from "@tauri-apps/api/core";
import { showErrorParsed, showJreInstallPrompt } from "../state/state.svelte";
import {
	type VersionIntegrity,
	type VersionStatus,
} from "../utils/versionUtils";
import {
	type DependencyRequest,
	type DependencyResolutionResult,
} from "../types/dependency";

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

async function invokeThrowingSilent(
	cmd: string,
	args?: Record<string, unknown> | null,
): Promise<void> {
	await invoke(cmd, args ?? {});
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
	min_gb: number;
	max_gb: number;
}> {
	const fallback = { total_gb: 8, min_gb: 1, max_gb: 2 };
	return (
		(await invokeWithFallback<{
			total_gb: number;
			min_gb: number;
			max_gb: number;
		}>("get_recommended_ram", null, fallback)) ?? fallback
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

export async function pinInstance(
	id: string,
	pinned: boolean,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<void> {
	return invokeWithCallback(
		"pin_instance",
		{ id, pinned },
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

export async function exportInstanceZip(
	instanceId: string,
	dest: string,
): Promise<string | null> {
	return (
		(await invokeWithFallback<string>("export_instance_zip", {
			id: instanceId,
			dest,
		})) ?? null
	);
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

export async function getCurseForgeProjectDescription(
	modId: number,
): Promise<string | null> {
	return (
		(await invokeWithFallback<string>(
			"get_curseforge_project_description",
			{ modId },
		)) ?? null
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
	fileName?: string,
): Promise<string | null> {
	return (
		(await invokeWithFallback<string>("get_curseforge_file_download_url", {
			modId,
			fileId,
			fileName,
		})) ?? null
	);
}

export async function searchCurseForgeModpacks(
	query: string,
	loader: string,
	gameVersion?: string,
	category?: string | null,
	index: string = "popularity",
	limit: number = 10,
	offset: number = 0,
): Promise<CurseForgeSearchResult | null> {
	return (
		(await invokeWithFallback<CurseForgeSearchResult>(
			"search_curseforge_modpacks",
			{
				query,
				loader,
				gameVersion: gameVersion || null,
				category: category || null,
				index,
				limit,
				offset,
			},
		)) ?? null
	);
}

export async function downloadCurseForgeModpack(
	url: string,
	fileId: number,
): Promise<string | null> {
	return (
		(await invokeWithFallback<string>("download_curseforge_modpack", {
			url,
			fileId,
		})) ?? null
	);
}

export async function parseCurseForgeModpack(
	path: string,
): Promise<CurseForgeModpackInfo | null> {
	return (
		(await invokeWithFallback<CurseForgeModpackInfo>(
			"parse_curseforge_modpack",
			{ path },
		)) ?? null
	);
}

export async function installCurseForgeModpack(
	path: string,
	instanceName: string,
	projectId?: number,
	fileId?: number,
	iconUrl?: string,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<CurseForgeModpackInfo | null> {
	try {
		const result = await invoke<CurseForgeModpackInfo>(
			"install_curseforge_modpack",
			{
				path,
				instanceName,
				projectId: projectId ?? null,
				fileId: fileId ?? null,
				iconUrl: iconUrl ?? null,
			},
		);
		callback?.();
		return result;
	} catch (err) {
		showErrorParsed(err);
		onError?.(err);
		return null;
	}
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
// Profile (skins & capes)
// ─────────────────────────────────────────────────────────────

export async function getMinecraftProfile(
	uuid: string,
): Promise<MinecraftProfileResponse | null> {
	return (
		(await invokeWithFallback<MinecraftProfileResponse>(
			"get_minecraft_profile",
			{ uuid },
		)) ?? null
	);
}

export async function getSkinPreviewData(filePath: string): Promise<string> {
	return invoke<string>("read_skin_preview_data", { filePath });
}

export async function uploadSkinFile(
	uuid: string,
	filePath: string,
	model: "slim" | "classic",
): Promise<void> {
	return invokeThrowingSilent("upload_skin_file", {
		uuid,
		filePath,
		model,
	});
}

export async function uploadSkinUrl(
	uuid: string,
	skinUrl: string,
	variant: "slim" | "classic",
): Promise<void> {
	return invokeThrowingSilent("upload_skin_url", {
		uuid,
		skinUrl,
		variant,
	});
}

export async function getSkinCloset(uuid: string): Promise<SkinClosetEntry[]> {
	return (
		(await invokeWithFallback<SkinClosetEntry[]>("get_skin_closet", {
			uuid,
		})) ?? []
	);
}

export async function syncSkinCloset(uuid: string): Promise<SkinClosetEntry[]> {
	return (
		(await invokeWithFallback<SkinClosetEntry[]>("sync_skin_closet", {
			uuid,
		})) ?? []
	);
}

export async function removeSkinFromCloset(
	uuid: string,
	entryId: string,
): Promise<void> {
	return invokeThrowing("remove_skin_from_closet", { uuid, entryId });
}

export async function renameSkinInCloset(
	uuid: string,
	entryId: string,
	alias: string,
): Promise<void> {
	return invokeThrowing("rename_skin_in_closet", { uuid, entryId, alias });
}

export async function equipSkinFromCloset(
	uuid: string,
	entryId: string,
): Promise<void> {
	return invokeThrowing("equip_skin_from_closet", { uuid, entryId });
}

export async function equipCape(uuid: string, capeId: string): Promise<void> {
	return invokeThrowingSilent("equip_cape", { uuid, capeId });
}

export async function unequipCape(uuid: string): Promise<void> {
	return invokeThrowingSilent("unequip_cape", { uuid });
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
// Instance archive import (MultiMC/Prism, extensible)
// ─────────────────────────────────────────────────────────────

export async function detectInstanceZip(
	path: string,
): Promise<InstanceImportPlan | null> {
	return (
		(await invokeWithFallback<InstanceImportPlan>("detect_instance_zip", {
			path,
		})) ?? null
	);
}

export async function importInstanceZip(
	previewToken: string,
	name: string,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<InstanceDto | null> {
	try {
		const result = await invoke<InstanceDto>("import_instance_zip", {
			previewToken,
			name,
		});
		callback?.();
		return result;
	} catch (err) {
		showErrorParsed(err);
		onError?.(err);
		return null;
	}
}

export async function cancelInstanceImport(
	previewToken: string,
): Promise<void> {
	try {
		await invoke("cancel_instance_import", { previewToken });
	} catch (err) {
		console.error("Error cancelando preview de importación:", err);
	}
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

export async function resolveModDependencies(
	requests: DependencyRequest[],
	loader: string,
	gameVersion: string,
): Promise<DependencyResolutionResult> {
	return (
		(await invokeWithFallback<DependencyResolutionResult>(
			"resolve_mod_dependencies",
			{
				requests,
				loader,
				gameVersion,
			},
		)) ?? { tree: [], conflicts: [] }
	);
}
