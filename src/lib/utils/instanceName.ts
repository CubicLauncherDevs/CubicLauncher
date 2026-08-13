/**
 * Validación y sanitización de nombres de instancia.
 *
 * Debe mantenerse sincronizada con la validación del backend
 * (`src-tauri/src/services/instance_manager/data.rs`).
 */
export const MAX_INSTANCE_NAME_LEN = 24;

const FORBIDDEN_CHARS = ["/", "\\", "<", ">", ":", '"', "|", "?", "*"];
const FORBIDDEN_CHARS_REGEX = /[\\/<>:"|?*]/g;

export function isValidInstanceName(name: string): boolean {
	const trimmed = name.trim();
	if (!trimmed) return false;
	if (trimmed.length > MAX_INSTANCE_NAME_LEN) return false;
	if (!/[\x20-\x7E]+/.test(trimmed)) return false;
	if (trimmed.includes("..")) return false;
	if (trimmed.split("").some((c) => FORBIDDEN_CHARS.includes(c)))
		return false;
	return true;
}

export function sanitizeInstanceName(name: string): string {
	let clean = name
		.normalize("NFD")
		.replace(/[\u0300-\u036f]/g, "")
		.replace(/[^\x20-\x7E]/g, "")
		.replace(FORBIDDEN_CHARS_REGEX, "")
		.replace(/\.\./g, "")
		.replace(/\s+/g, " ")
		.trim();

	if (!clean) clean = "Imported";
	if (clean.length > MAX_INSTANCE_NAME_LEN) {
		clean = clean.slice(0, MAX_INSTANCE_NAME_LEN).trim();
	}
	return clean;
}
