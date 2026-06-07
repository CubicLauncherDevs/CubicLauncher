import {
	type InstanceDto,
	type ModDto,
	type DeviceCode,
	type MinecraftUser,
	type Settings,
	type MinecraftVersion,
	type FabricGameVersion,
	type ModrinthSearchResult,
	type ModrinthVersion,
	type JreStatus,
	type McVersion,
} from "../types/types";
import { invoke } from "@tauri-apps/api/core";
import { showError } from "../state/state.svelte";

export async function killInstance(
	uuid: string,
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<void> {
	try {
		await invoke("kill_instance", { uuid: uuid });
		callback?.();
	} catch (err) {
		console.error(`Error al matar instancia con id:${uuid}:`, err);
		showError("Error", `No se pudo detener la instancia con id: ${uuid}`);
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
		console.error(`Error al crear instancia ${name}:`, err);
		showError("Error", `No se pudo crear la instancia ${name}: ${err}`);
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
		console.error(`Error al eliminar instancia ${id}:`, err);
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
		console.error(`Error al renombrar instancia ${id}:`, err);
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
		console.error(`Error al actualizar instancia ${id}:`, err);
		onError?.(err);
	}
}

export async function getInstalledVersions(): Promise<string[]> {
	try {
		return await invoke<string[]>("get_installed_versions");
	} catch (err) {
		console.error("Error al obtener versiones instaladas:", err);
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
	return { loader: "vanilla", version: raw, type: "" };
}

export function getInstalledMcVersions(raw: string[]): {
	vanilla: Set<string>;
	fabric: Set<string>;
} {
	const vanilla = new Set<string>();
	const fabric = new Set<string>();
	for (const v of raw) {
		const parsed = parseInstalledVersion(v);
		if (parsed.loader === "vanilla") vanilla.add(parsed.version);
		else if (parsed.loader === "fabric") fabric.add(parsed.version);
	}
	return { vanilla, fabric };
}

export async function getInstanceMods(id: string): Promise<ModDto[]> {
	try {
		return await invoke<ModDto[]>("get_instance_mods", { id });
	} catch (err) {
		console.error(`Error al obtener mods de instancia ${id}:`, err);
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
		console.error(`Error al hacer toggle de mod ${filename}:`, err);
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
		console.error(`Error al lanzar instancia ${instance.name}:`, err);
		showError(
			"Error de lanzamiento",
			`No se pudo iniciar ${instance.name}: ${err}`,
		);
		onError?.(err);
	}
}

export async function fetchAll(
	callback?: () => void,
	onError?: (err: unknown) => void,
): Promise<InstanceDto[]> {
	try {
		const dtos = await invoke<InstanceDto[]>("get_instances");
		console.log(dtos);
		callback?.();
		return dtos;
	} catch (err) {
		console.error("Error en fetchAll:", err);
		onError?.(err);
		return [];
	}
}

export async function getSettings(): Promise<Settings | null> {
	try {
		return await invoke<Settings>("get_settings");
	} catch (err) {
		console.error("Error al obtener settings:", err);
		return null;
	}
}

export async function updateSettings(settings: Settings): Promise<void> {
	try {
		await invoke("update_settings", { newSettings: settings });
		console.log(settings);
	} catch (err) {
		console.error("Error al actualizar settings:", err);
	}
}
export async function getAvailableVersions(): Promise<MinecraftVersion[]> {
	try {
		return await invoke<MinecraftVersion[]>("get_available_versions");
	} catch (err) {
		console.error("Error al obtener versiones disponibles:", err);
		return [];
	}
}

export async function refreshAvailableVersions(): Promise<MinecraftVersion[]> {
	try {
		return await invoke<MinecraftVersion[]>("refresh_versions");
	} catch (err) {
		console.error("Error al refrescar versiones:", err);
		return [];
	}
}

export async function addToQueue(version: string): Promise<void> {
	try {
		await invoke("add_to_queue", { version });
	} catch (err) {
		console.error(`Error al agregar ${version} a la cola:`, err);
	}
}

export async function getFabricVersions(): Promise<FabricGameVersion[]> {
	try {
		return await invoke<FabricGameVersion[]>("get_fabric_versions");
	} catch (err) {
		console.error("Error al obtener versiones de Fabric:", err);
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
		console.error(`Error al descargar Fabric para ${gameVersion}:`, err);
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

export async function initDiscordPresence(): Promise<void> {
	try {
		await invoke("init_discord_presence");
	} catch (err) {
		console.error("Error al iniciar Discord Presence:", err);
	}
}

export async function openUrl(url: string): Promise<void> {
	await invoke("open_url", { url });
}

export async function getInstanceResourcePacks(id: string): Promise<ModDto[]> {
	try {
		return await invoke<ModDto[]>("get_instance_resourcepacks", { id });
	} catch (err) {
		console.error(
			`Error al obtener resource packs de instancia ${id}:`,
			err,
		);
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
		console.error(`Error al eliminar archivo ${filename}:`, err);
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
		console.error(`Error al añadir archivo ${sourcePath}:`, err);
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
		console.error("Error al obtener la cola de descargas:", err);
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
): Promise<ModrinthSearchResult | null> {
	try {
		const facets = [];
		if (loader.toLowerCase() !== "vanilla") {
			facets.push([`categories:${loader.toLowerCase()}`]);
		}
		if (gameVersion) {
			facets.push([`versions:${gameVersion}`]);
		}
		facets.push(["project_type:mod"]);

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
		console.error("Error searching Modrinth:", err);
		showError("Modrinth Error", `Could not search for mods: ${err}`);
		return null;
	}
}

export async function getModrinthProjectVersions(
	projectId: string,
	loader: string,
	gameVersion?: string,
): Promise<ModrinthVersion[]> {
	try {
		const loadersJson = JSON.stringify([loader.toLowerCase()]);
		const url = new URL(
			`https://api.modrinth.com/v2/project/${projectId}/version`,
		);
		url.searchParams.append("loaders", loadersJson);
		if (gameVersion) {
			url.searchParams.append("game_versions", JSON.stringify([gameVersion]));
		}

		const res = await fetch(url.toString());
		if (!res.ok) {
			throw new Error(`Modrinth API error: ${res.status}`);
		}
		return (await res.json()) as ModrinthVersion[];
	} catch (err) {
		console.error(`Error getting versions for ${projectId}:`, err);
		return [];
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
		console.error(
			`Error downloading mods for instance ${instanceId}:`,
			err,
		);
		showError("Download Error", `Could not download mods: ${err}`);
		throw err;
	}
}

export async function getJreStatus(version: number): Promise<JreStatus | null> {
	try {
		return await invoke<JreStatus>("get_jre_status", { version });
	} catch (err) {
		console.error(`Error getting JRE ${version} status:`, err);
		return null;
	}
}

export async function getJreVersions(): Promise<JreStatus[]> {
	try {
		return await invoke<JreStatus[]>("get_jre_versions");
	} catch (err) {
		console.error("Error getting JRE versions:", err);
		return [];
	}
}

export async function getInstallingJres(): Promise<number[]> {
	try {
		return await invoke<number[]>("get_installing_jres");
	} catch (err) {
		console.error("Error getting installing JREs:", err);
		return [];
	}
}

export async function installJre(version: number): Promise<void> {
	try {
		await invoke("install_jre", { version });
	} catch (err) {
		console.error(`Error installing JRE ${version}:`, err);
		throw err;
	}
}

export async function uninstallJre(version: number): Promise<void> {
	try {
		await invoke("uninstall_jre", { version });
	} catch (err) {
		console.error(`Error uninstalling JRE ${version}:`, err);
		throw err;
	}
}
