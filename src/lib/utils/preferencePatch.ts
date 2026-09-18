/** Keep reactive preference objects stable and notify only changed fields. */
export function patchPreferences<T extends object>(
	target: T,
	patch: Partial<T>,
): boolean {
	let changed = false;
	for (const key of Object.keys(patch) as (keyof T)[]) {
		const value = patch[key] as T[keyof T];
		if (!Object.is(target[key], value)) {
			target[key] = value;
			changed = true;
		}
	}
	return changed;
}
