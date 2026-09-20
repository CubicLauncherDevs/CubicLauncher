import { MAX_INSTANCE_NAME_LEN, isValidInstanceName } from "./instanceName";

/** Reserve names across both existing instances and the current migration batch. */
export function suggestMigrationNames(
	names: string[],
	existing: string[],
): string[] {
	const used = new Set(existing);
	return names.map((name) => {
		const base = name.trim().slice(0, MAX_INSTANCE_NAME_LEN) || "Imported";
		let candidate = base;
		let index = 2;
		while (used.has(candidate)) {
			const suffix = ` (${index++})`;
			candidate =
				base.slice(0, MAX_INSTANCE_NAME_LEN - suffix.length).trimEnd() +
				suffix;
		}
		used.add(candidate);
		return candidate;
	});
}

export function validMigrationNames(
	names: string[],
	existing: string[],
): boolean {
	const used = new Set(existing);
	return (
		names.length > 0 &&
		names.every((name) => {
			const trimmed = name.trim();
			if (!isValidInstanceName(trimmed) || used.has(trimmed))
				return false;
			used.add(trimmed);
			return true;
		})
	);
}
