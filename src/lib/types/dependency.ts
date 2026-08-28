export type DependencySource = "modrinth" | "curseforge";
export type DependencyKind =
	"required" | "optional" | "embedded" | "incompatible";

export interface DependencyRequest {
	source: DependencySource;
	projectId: string;
	versionId: string | null;
	kind: DependencyKind;
}

export interface ResolvedDependency {
	projectId: string;
	versionId: string | null;
	source: DependencySource;
	title: string;
	iconUrl: string | null;
	filename: string;
	downloadUrl: string | null;
	kind: DependencyKind;
	depth: number;
	children: ResolvedDependency[];
}

export interface RequestedVersion {
	versionId: string;
	requestedBy: string;
}

export interface DependencyConflict {
	projectId: string;
	source: DependencySource;
	requestedVersions: RequestedVersion[];
}

export interface DependencyResolutionResult {
	tree: ResolvedDependency[];
	conflicts: DependencyConflict[];
}
