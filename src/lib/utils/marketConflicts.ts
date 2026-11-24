import type {
	DependencyConflict,
	DependencyOrigin,
	DependencyResolutionResult,
	DependencySource,
	InstalledModRef,
	ResolvedDependency,
} from "#lib/types/dependency.js";
import type { ModDto } from "#lib/types/types.js";

export type DetectedConflictType =
	"version_mismatch" | "incompatible" | "missing" | "disabled" | "unresolved";

/** A problem found while preparing an install, ready to be shown. */
export interface DetectedConflict {
	key: string;
	type: DetectedConflictType;
	/** Blocks the install because the game would fail to load. */
	fatal: boolean;
	projectId: string;
	source: DependencySource;
	title: string;
	iconUrl: string | null;
	/** Projects demanding this one, with the version they ask for. */
	requestedBy: DependencyOrigin[];
	/** Readable label of the installed copy: its version or file name. */
	installedVersion: string | null;
	installedFilename: string | null;
	reason: string | null;
}

/** Shape stored in `DependencyResolutionResult.conflicts` and shown by the modal. */
export interface InstallConflict {
	project_id: string;
	source: DependencySource;
	title: string;
	icon_url: string | null;
	type: DetectedConflictType;
	fatal: boolean;
	requested_by: DependencyOrigin[];
	/** Readable label of the installed copy (version number or file name). */
	installed_version: string | null;
	installed_filename: string | null;
	reason: string | null;
}

const identity = (source: string, id: string) => `${source}:${id}`;

/**
 * What a user recognises an installed file by: the version its metadata
 * recorded, or the file name when there is none. Provider version ids are
 * opaque, so they are never shown on their own.
 */
export function installedLabel(mod: ModDto): string {
	const version = mod.version?.trim();
	if (version) return version;
	return mod.filename.replace(/\.disabled$/i, "").replace(/\.jar$/i, "");
}

/**
 * Inventory sent with the resolution so the backend can label conflicts with
 * the installed version instead of its opaque version id.
 */
export function installedModRefs(mods: ModDto[]): InstalledModRef[] {
	return mods
		.filter(
			(mod) =>
				mod.project_id &&
				(mod.source === "modrinth" || mod.source === "curseforge"),
		)
		.map((mod) => ({
			source: mod.source as DependencySource,
			project_id: mod.project_id!,
			enabled: isEnabled(mod),
			filename: mod.filename,
			version: mod.version,
		}));
}

function isEnabled(mod: ModDto): boolean {
	return mod.enabled && !/\.disabled$/i.test(mod.filename);
}

function stem(filename: string): string {
	return filename.replace(/\.disabled$/i, "").toLowerCase();
}

function walk(
	nodes: ResolvedDependency[],
	visit: (node: ResolvedDependency) => void,
) {
	for (const node of nodes) {
		visit(node);
		walk(node.children, visit);
	}
}

/**
 * Compares what the resolver wants to install with the instance inventory so
 * the preview can warn about incompatibilities, missing or disabled
 * dependencies and dependencies installed with a different version.
 */
export function detectInstalledConflicts(
	resolution: DependencyResolutionResult,
	installed: ModDto[],
): DetectedConflict[] {
	const byProject = new Map<string, ModDto>();
	const byFilename = new Map<string, ModDto>();
	for (const mod of installed) {
		if (mod.project_id)
			byProject.set(identity(mod.source, mod.project_id), mod);
		byFilename.set(stem(mod.filename), mod);
	}
	const active = installed.filter(isEnabled);
	const activeByProject = new Map<string, ModDto>();
	for (const mod of active) {
		if (mod.project_id)
			activeByProject.set(identity(mod.source, mod.project_id), mod);
	}

	const conflicts: DetectedConflict[] = [];
	const seen = new Set<string>();
	/** Projects already explained by the inventory check. */
	const reported = new Set<string>();
	const push = (conflict: DetectedConflict) => {
		if (seen.has(conflict.key)) return;
		seen.add(conflict.key);
		conflicts.push(conflict);
		reported.add(conflict.projectId);
	};

	walk(resolution.tree, (node) => {
		if (node.kind === "incompatible" || node.kind === "embedded") return;
		const key = identity(node.source, node.project_id);
		const local = byProject.get(key) ?? byFilename.get(stem(node.filename));
		if (!local) return;
		if (!isEnabled(local)) {
			push({
				key: `disabled:${key}`,
				type: "disabled",
				fatal: node.kind === "required",
				projectId: node.project_id,
				source: node.source,
				title: node.title,
				iconUrl: node.icon_url,
				requestedBy: [],
				installedVersion: installedLabel(local),
				installedFilename: local.filename,
				reason: null,
			});
			return;
		}
		const sameVersion =
			!node.version_id ||
			!local.version_id ||
			local.version_id === node.version_id;
		if (!sameVersion) {
			push({
				key: `version:${key}`,
				type: "version_mismatch",
				fatal: node.kind === "required",
				projectId: node.project_id,
				source: node.source,
				title: node.title,
				iconUrl: node.icon_url,
				requestedBy: [],
				installedVersion: installedLabel(local),
				installedFilename: local.filename,
				reason: null,
			});
		}
	});

	const resolveTitle = (origin: DependencyOrigin) =>
		activeByProject.get(identity(origin.source, origin.project_id))?.name ??
		origin.project_id;
	const parentsOf = (conflict: DependencyConflict) =>
		(conflict.requested_by ?? [])
			.filter((origin) => origin.project_id !== conflict.project_id)
			.map((origin) => ({
				project_id: resolveTitle(origin),
				source: origin.source,
				version_id: origin.version_id ?? null,
			}));

	const required = new Set<string>();
	walk(resolution.tree, (node) => {
		if (node.kind === "required")
			required.add(identity(node.source, node.project_id));
	});

	for (const incompatible of resolution.incompatible ?? []) {
		const local = activeByProject.get(
			identity(incompatible.source, incompatible.project_id),
		);
		if (!local) continue;
		push({
			key: `incompatible:${identity(incompatible.source, incompatible.project_id)}`,
			type: "incompatible",
			// Disabling the installed mod is enough to avoid the crash.
			fatal: false,
			projectId: incompatible.project_id,
			source: incompatible.source,
			title: local.name,
			iconUrl: local.icon,
			requestedBy: [],
			installedVersion: installedLabel(local),
			installedFilename: local.filename,
			reason: null,
		});
	}

	for (const conflict of resolution.conflicts ?? []) {
		const key = identity(conflict.source, conflict.project_id);
		if (conflict.kind === "unresolved") {
			if (activeByProject.has(key)) continue;
			push({
				key: `unresolved:${key}`,
				type: "unresolved",
				fatal: required.has(key),
				projectId: conflict.project_id,
				source: conflict.source,
				title: conflict.title ?? conflict.project_id,
				iconUrl: conflict.icon_url ?? null,
				requestedBy: parentsOf(conflict),
				installedVersion: null,
				installedFilename: null,
				reason: conflict.reason ?? null,
			});
			continue;
		}
		if (reported.has(conflict.project_id)) continue;
		push({
			key: `version_mismatch:${key}`,
			type: "version_mismatch",
			fatal: required.has(key),
			projectId: conflict.project_id,
			source: conflict.source,
			title: conflict.title ?? conflict.project_id,
			iconUrl: conflict.icon_url ?? null,
			requestedBy: parentsOf(conflict),
			installedVersion: null,
			installedFilename: null,
			reason: conflict.reason ?? null,
		});
	}

	// A required dependency with no compatible version must always block, even
	// if the resolver did not report it in its own conflict list.
	walk(resolution.tree, (node) => {
		if (node.kind !== "required" || !node.unresolved_reason) return;
		push({
			key: `unresolved:${identity(node.source, node.project_id)}`,
			type: "unresolved",
			fatal: true,
			projectId: node.project_id,
			source: node.source,
			title: node.title,
			iconUrl: node.icon_url,
			requestedBy: [],
			installedVersion: null,
			installedFilename: null,
			reason: node.unresolved_reason,
		});
	});

	return conflicts;
}

export function hasFatalConflict(conflicts: InstallConflict[]): boolean {
	return conflicts.some((conflict) => conflict.fatal);
}

/** Converts the detected problems to the shape stored in the resolution. */
export function toInstallConflicts(
	conflicts: DetectedConflict[],
): InstallConflict[] {
	return conflicts.map((conflict) => ({
		project_id: conflict.projectId,
		source: conflict.source,
		title: conflict.title,
		icon_url: conflict.iconUrl,
		type: conflict.type,
		fatal: conflict.fatal,
		requested_by: conflict.requestedBy,
		installed_version: conflict.installedVersion,
		installed_filename: conflict.installedFilename,
		reason: conflict.reason,
	}));
}

/**
 * Resolver conflicts combined with the instance inventory. Everything that
 * consumes `conflicts` (quick install, mod updates, the preview) reads this.
 */
export function detectInstallConflicts(
	resolution: DependencyResolutionResult,
	installed: ModDto[],
): InstallConflict[] {
	return toInstallConflicts(detectInstalledConflicts(resolution, installed));
}

export function conflictLabel(type: DetectedConflictType): string {
	return `market.detail.dependencies.conflictType.${type}`;
}
