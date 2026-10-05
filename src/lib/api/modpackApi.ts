import { invoke } from "@tauri-apps/api/core";

export interface PackState {
	schema_version: 1;
	source: "modrinth" | "curseforge" | "local";
	project_id: string | null;
	version_id: string | null;
	name: string;
	version: string;
	game_version: string;
	locked: boolean;
	needs_inventory?: boolean;
	files: Record<string, { sha1: string; downloads: string[]; size: number }>;
}
export interface PackVersion {
	id: string;
	name: string;
	version: string;
}
export interface PackUpdatePreview {
	token: string;
	name: string;
	version: string;
	game_version: string;
	added: string[];
	removed: string[];
	changed: string[];
	conflicts: string[];
}
export type PackResolutions = Record<string, "keep" | "replace">;

export const getInstanceModpack = (id: string) =>
	invoke<PackState | null>("get_instance_modpack", { id });

export const setModpackLocked = (id: string, locked: boolean) =>
	invoke<PackState>("set_modpack_locked", { id, locked });

export const restoreModpackInventory = (id: string) =>
	invoke<PackState>("restore_modpack_inventory", { id });

export const listModpackVersions = (id: string) =>
	invoke<PackVersion[]>("list_modpack_versions", { id });

export const previewModpackUpdate = (
	id: string,
	versionId: string | null,
	path: string | null,
) =>
	invoke<PackUpdatePreview>("preview_modpack_update", {
		id,
		versionId,
		path,
	});

export const applyModpackUpdate = (
	id: string,
	token: string,
	resolutions: PackResolutions,
) => invoke<void>("apply_modpack_update", { id, token, resolutions });

export const cancelModpackUpdate = (id: string, token: string) =>
	invoke<void>("cancel_modpack_update", { id, token });
