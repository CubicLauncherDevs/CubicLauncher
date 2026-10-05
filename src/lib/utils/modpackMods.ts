import type { ModDto } from "$lib/types/types";

export type ModOwnership = "own" | "pack" | "all";

export function isPackProtected(mod?: ModDto): boolean {
	return mod?.pack_locked === true;
}

export function matchesModOwnership(
	mod: ModDto | undefined,
	filter: ModOwnership,
): boolean {
	return (
		filter === "all" ||
		(filter === "pack"
			? mod?.pack_name != null
			: mod?.pack_name == null && !isPackProtected(mod))
	);
}

/** Treat disabled files as the same target when checking pack ownership. */
export function targetsProtectedMod(
	mods: ModDto[],
	filename: string,
	projectId?: string,
): boolean {
	const normalized = filename.replace(/\.disabled$/i, "");
	return mods.some(
		(mod) =>
			isPackProtected(mod) &&
			(mod.filename.replace(/\.disabled$/i, "") === normalized ||
				(!!projectId && mod.project_id === projectId)),
	);
}
