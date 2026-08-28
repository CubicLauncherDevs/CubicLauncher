export type DependencySource = "modrinth" | "curseforge";
export type DependencyKind =
	"required" | "optional" | "embedded" | "incompatible";

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
	children: ResolvedDependency[];
}

export interface RequestedVersion {
	version_id: string;
	requested_by: string;
}

export interface DependencyConflict {
	project_id: string;
	source: DependencySource;
	requested_versions: RequestedVersion[];
}

export interface DependencyResolutionResult {
	tree: ResolvedDependency[];
	conflicts: DependencyConflict[];
}
