export type DependencySource = "modrinth" | "curseforge";
export type DependencyKind =
	"required" | "optional" | "embedded" | "incompatible";

export type ConflictType = "version_mismatch" | "unresolved";

export interface DependencyRequest {
	source: DependencySource;
	project_id: string;
	version_id: string | null;
	kind: DependencyKind;
}

export interface ResolvedDependency {
	project_id: string;
	version_id: string | null;
	source: DependencySource;
	title: string;
	icon_url: string | null;
	filename: string;
	download_url: string | null;
	kind: DependencyKind;
	depth: number;
	unresolved_reason?: string | null;
	game_versions?: string[];
	loaders?: string[];
	children: ResolvedDependency[];
}

/** Installed copy of a project, sent so conflicts can name its version. */
export interface InstalledModRef {
	source: DependencySource;
	project_id: string;
	enabled: boolean;
	filename: string;
	version?: string | null;
}

export interface RequestedVersion {
	version_id: string;
	requested_by: string;
}

/** Project that asks for another one, and the version it ends up with. */
export interface DependencyOrigin {
	project_id: string;
	source: DependencySource;
	version_id?: string | null;
}

export interface DependencyConflict {
	project_id: string;
	source: DependencySource;
	title?: string;
	icon_url?: string | null;
	kind?: ConflictType;
	/** `project_id` is the parent's title whenever it could be resolved. */
	requested_by: DependencyOrigin[];
	requested_versions?: RequestedVersion[];
	reason?: string | null;
	/** Label of the installed copy, e.g. its version number or file name. */
	installed_version?: string | null;
	type?: string;
	fatal?: boolean;
	installed_version_id?: string | null;
	installed_filename?: string | null;
}

export interface DependencyResolutionResult {
	tree: ResolvedDependency[];
	conflicts: DependencyConflict[];
	incompatible?: DependencyOrigin[];
}
