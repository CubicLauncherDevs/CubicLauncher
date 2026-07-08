import type { ModDto, ModrinthDependency, ModrinthProject, ModrinthProjectFull, ModrinthVersion, InstanceDto } from "./types";

export type MarketSource = "local" | "modrinth" | "curseforge";

export interface MarketProject {
	id: string;
	title: string;
	description: string;
	author: string;
	icon: string | null;
	source: MarketSource;
	downloadCount: number;
	followerCount?: number;

	// Remote-specific data
	modrinth?: ModrinthProject;
	curseforge?: never; // placeholder for future source support

	// Local-specific data
	installed?: ModDto;
	modrinthProjectId?: string;
	modrinthVersionId?: string;

	// UI state, not persisted
	installedVersion?: string;
	hasUpdate?: boolean;
	disabled?: boolean;
}

export interface MarketVersion {
	id: string;
	name: string;
	versionNumber: string;
	datePublished: string;
	loaders: string[];
	gameVersions: string[];
	isInstalled: boolean;
	primaryFileUrl: string;
	primaryFileName: string;
	dependencies: MarketDependency[];
}

export interface MarketDependency {
	projectId: string;
	versionId: string | null;
	dependencyType: string;
}

export interface MarketDetailData {
	project: MarketProject;
	fullProject?: ModrinthProjectFull;
	versions: MarketVersion[];
	selectedVersion: MarketVersion | null;
	readmeHtml: string;
	loading: boolean;
	error: string | null;
}

export function modrinthProjectToMarket(project: ModrinthProject): MarketProject {
	return {
		id: project.project_id,
		title: project.title,
		description: project.description,
		author: project.author,
		icon: project.icon_url,
		source: "modrinth",
		downloadCount: project.downloads,
		modrinth: project,
	};
}

export function localModToMarket(
	mod: ModDto,
	metadata?: { project_id?: string; version_id?: string },
): MarketProject {
	return {
		id: metadata?.project_id ?? `local-${mod.filename}`,
		title: mod.name,
		description: mod.description ?? "",
		author: mod.authors?.join(", ") ?? "",
		icon: mod.icon,
		source: metadata?.project_id ? "modrinth" : "local",
		downloadCount: 0,
		installed: mod,
		modrinthProjectId: metadata?.project_id,
		modrinthVersionId: metadata?.version_id,
		disabled: !mod.enabled,
	};
}

export function getMarketProjectId(project: MarketProject): string {
	return project.modrinthProjectId ?? project.id;
}

export function isMarketProjectInstalled(project: MarketProject): boolean {
	return !!project.installed;
}

export function parseInstanceVersion(instance: InstanceDto): {
	loader: string;
	gameVersion: string;
} {
	const version = instance.version;
	const lower = version.toLowerCase();

	if (lower.startsWith("fabric-loader-")) {
		const lastDash = version.lastIndexOf("-");
		const gameVersion = lastDash !== -1 ? version.slice(lastDash + 1) : version;
		return { loader: "fabric", gameVersion };
	}

	if (lower.startsWith("quilt-loader-")) {
		const lastDash = version.lastIndexOf("-");
		const gameVersion = lastDash !== -1 ? version.slice(lastDash + 1) : version;
		return { loader: "quilt", gameVersion };
	}

	if (lower.includes("-forge-")) {
		const idx = lower.indexOf("-forge-");
		return { loader: "forge", gameVersion: version.slice(0, idx) };
	}

	if (lower.includes("-neoforge-")) {
		const idx = lower.indexOf("-neoforge-");
		return { loader: "neoforge", gameVersion: version.slice(0, idx) };
	}

	return { loader: instance.loader.toLowerCase(), gameVersion: version };
}

export function modrinthVersionToMarket(
	version: ModrinthVersion,
	installedVersionId?: string,
): MarketVersion {
	const primaryFile = version.files.find((f) => f.primary) ?? version.files[0];

	return {
		id: version.id,
		name: version.name,
		versionNumber: version.version_number,
		datePublished: version.date_published,
		loaders: version.loaders,
		gameVersions: version.game_versions,
		isInstalled: version.id === installedVersionId,
		primaryFileUrl: primaryFile?.url ?? "",
		primaryFileName: primaryFile?.filename ?? "",
		dependencies: (version.dependencies ?? []).map(modrinthDependencyToMarket),
	};
}

export function modrinthDependencyToMarket(dep: ModrinthDependency): MarketDependency {
	return {
		projectId: dep.project_id ?? "",
		versionId: dep.version_id,
		dependencyType: dep.dependency_type,
	};
}
