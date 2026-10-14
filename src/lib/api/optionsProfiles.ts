import { invoke } from "@tauri-apps/api/core";

/** Perfil global de `options.txt` (vista que envía el backend). */
export interface OptionsProfile {
	id: string;
	name: string;
	content: string;
	created_at: number;
	updated_at: number;
}

export interface ApplyOptionsResult {
	/** `true` si el options.txt de la instancia cambió. */
	changed: boolean;
	/** Ruta del respaldo creado, si lo hubo. */
	backup: string | null;
}

export const listOptionsProfiles = () =>
	invoke<OptionsProfile[]>("list_options_profiles");

export const createOptionsProfile = (name: string, content: string) =>
	invoke<OptionsProfile>("create_options_profile", { name, content });

export const updateOptionsProfile = (id: string, name: string, content: string) =>
	invoke<OptionsProfile>("update_options_profile", { id, name, content });

export const deleteOptionsProfile = (id: string) =>
	invoke<void>("delete_options_profile", { id });

/** Guarda el options.txt actual de una instancia como un perfil nuevo. */
export const createOptionsProfileFromInstance = (
	instanceId: string,
	name: string,
) =>
	invoke<OptionsProfile>("create_options_profile_from_instance", {
		instanceId,
		name,
	});

/** Importa un options.txt desde un archivo. Si `name` está vacío usa el del archivo. */
export const importOptionsProfileFromFile = (path: string, name: string) =>
	invoke<OptionsProfile>("import_options_profile_from_file", { path, name });

/**
 * Aplica un perfil a una instancia.
 *
 * Con `overwrite = false` no sobrescribe: lanza el error `INST_OPTIONS_EXISTS`
 * si ya hay un options.txt, para que el llamador confirme y reintente con `true`.
 */
export const applyOptionsProfile = (
	id: string,
	profileId: string,
	overwrite: boolean,
) =>
	invoke<ApplyOptionsResult>("apply_options_profile", {
		id,
		profileId,
		overwrite,
	});

/** Asigna (o quita con `profileId = null`) el perfil de options.txt de una instancia. */
export const setInstanceOptionsProfile = (
	id: string,
	profileId: string | null,
	autoApply: boolean,
) =>
	invoke<void>("set_instance_options_profile", { id, profileId, autoApply });

/** Extrae el código de error del JSON `{ code }` emitido por Rust. */
export function optionsErrorCode(err: unknown): string | null {
	if (typeof err !== "string") return null;
	try {
		const parsed = JSON.parse(err);
		return typeof parsed?.code === "string" ? parsed.code : null;
	} catch {
		return null;
	}
}
