import type {
	ModDto,
	ModrinthDependency,
	ModrinthProject,
	ModrinthProjectFull,
	ModrinthVersion,
	CurseForgeProject,
	CurseForgeFile,
	InstanceDto,
} from "./types";

export type MarketSource = "local" | "modrinth" | "curseforge";
export type ContentType = "mods" | "resourcepacks" | "shaderpacks";

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
	curseforge?: CurseForgeProject;

	// Local-specific data
	installed?: ModDto;
	modrinthProjectId?: string;
	modrinthVersionId?: string;
	curseforgeProjectId?: string;
	curseforgeVersionId?: string;

	// UI state, not persisted
	installedVersion?: string;
	hasUpdate?: boolean;
	disabled?: boolean;
	hasRemoteData?: boolean;
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

export function modrinthProjectToMarket(
	project: ModrinthProject,
): MarketProject {
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

export function curseforgeProjectToMarket(
	project: CurseForgeProject,
): MarketProject {
	return {
		id: String(project.id),
		title: project.name,
		description: project.summary,
		author: project.authors.map((a) => a.name).join(", "),
		icon: project.logo?.url ?? null,
		source: "curseforge",
		downloadCount: project.downloadCount,
		curseforge: project,
	};
}

export function curseforgeVersionToMarket(
	file: CurseForgeFile,
	installedFileId?: string | number,
): MarketVersion {
	return {
		id: String(file.id),
		name: file.fileName,
		versionNumber: file.fileName,
		datePublished: file.fileDate,
		loaders: file.modLoaders.map((l) =>
			l.toLowerCase().replace("modloader-", "").replace("-", ""),
		),
		gameVersions: file.gameVersions,
		isInstalled:
			installedFileId !== undefined &&
			file.id === Number(installedFileId),
		primaryFileUrl: file.downloadUrl ?? "",
		primaryFileName: file.fileName,
		dependencies: [],
	};
}

export function localModToMarket(mod: ModDto): MarketProject {
	const isNumericId = mod.project_id ? /^\d+$/.test(mod.project_id) : false;
	const source: MarketSource = (mod.source as MarketSource) ?? "local";

	return {
		id: mod.project_id ?? `local-${mod.filename}`,
		title: mod.name,
		description: mod.description ?? "",
		author: mod.authors?.join(", ") ?? "",
		icon: mod.icon,
		source,
		downloadCount: 0,
		installed: mod,
		modrinthProjectId:
			source === "modrinth" ? (mod.project_id ?? undefined) : undefined,
		modrinthVersionId: undefined,
		curseforgeProjectId:
			source === "curseforge" ? (mod.project_id ?? undefined) : undefined,
		curseforgeVersionId: undefined,
		disabled: !mod.enabled,
		hasRemoteData: source !== "local",
		curseforge:
			mod.slug && isNumericId
				? ({ slug: mod.slug } as unknown as CurseForgeProject)
				: undefined,
		modrinth:
			mod.slug && !isNumericId
				? ({ slug: mod.slug } as unknown as ModrinthProject)
				: undefined,
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
		const gameVersion =
			lastDash !== -1 ? version.slice(lastDash + 1) : version;
		return { loader: "fabric", gameVersion };
	}

	if (lower.startsWith("quilt-loader-")) {
		const lastDash = version.lastIndexOf("-");
		const gameVersion =
			lastDash !== -1 ? version.slice(lastDash + 1) : version;
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
	const primaryFile =
		version.files.find((f) => f.primary) ?? version.files[0];

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
		dependencies: (version.dependencies ?? []).map(
			modrinthDependencyToMarket,
		),
	};
}

export function modrinthDependencyToMarket(
	dep: ModrinthDependency,
): MarketDependency {
	return {
		projectId: dep.project_id ?? "",
		versionId: dep.version_id,
		dependencyType: dep.dependency_type,
	};
}
