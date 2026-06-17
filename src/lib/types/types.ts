export interface InstanceDto {
	name: string;
	version: string;
	loader: string;
	last_played: number;
	status: InstState;
	cover_image: string | null;
	icon: string | null;
	uuid: string;
	path: string;
}

export enum InstState {
	Started = "started",
	Error = "error",
	Starting = "starting",
	Off = "off",
}

export interface ModDto {
	name: string;
	filename: string;
	version: string | null;
	description: string | null;
	authors: string[] | null;
	icon: string | null;
	enabled: boolean;
}
export interface InstancesPollingPayload {
	running: string[];
	all: InstanceDto[];
	count: number;
}

export interface Settings {
	user: MinecraftUser[];
	active_user_idx: number;
	min_memory: number;
	max_memory: number;
	jre8_path: string;
	jre8_managed: boolean;
	jre17_path: string;
	jre17_managed: boolean;
	jre21_path: string;
	jre21_managed: boolean;
	jre25_path: string;
	jre25_managed: boolean;
	language: string;
	auto_updates: boolean;
	close_launcher_on_play: boolean;
	show_snapshots: boolean;
	show_alpha: boolean;
	jvm_args: string;
	env_vars: Record<string, string>;
	theme: string;
	discord_presence: boolean;
	show_tutorial: boolean;
}

export interface JreStatus {
	version: number;
	installed: boolean;
	java_version: string | null;
}

export type AccountType = "Cracked" | "Microsoft" | "Yggdrasil";

export interface MinecraftUser {
	username: string;
	uuid: string;
	access_token: string;
	refresh_token: string | null;
	user_type: AccountType;
	yggdrasil_server_url?: string | null;
}

export interface YggdrasilServerInfo {
	server_name: string;
	skin_domains: string[];
	non_email_login: boolean;
}

export interface DeviceCode {
	user_code: string;
	device_code: string;
	verification_uri: string;
	expires_in: number;
	interval: number;
}

export type NotificationType = "error" | "info" | "success" | "warning";

export interface Notification {
	id: string;
	type: NotificationType;
	title: string;
	message: string;
	timeout?: number;
	progress?: number;
	totalMb?: number;
}

export interface McVersion {
	loader: "vanilla" | "fabric" | "forge" | "neoforge";
	version: string;
	type: string;
}

export interface MinecraftVersion {
	id: string;
	type: string;
	url: string;
	time: string;
	releaseTime: string;
}

export interface FabricGameVersion {
	version: string;
	stable: boolean;
}

export interface ForgeGameVersion {
	version_id: string;
	game_version: string;
	forge_version: string;
}

export type AppEvent =
	| {
			type: "InstanceStarted";
			data: {
				id: string;
			};
	  }
	| {
			type: "InstanceDeleted";
			data: {
				id: string;
			};
	  }
	| {
			type: "InstanceEdited";
			data: {
				id: string;
			};
	  }
	| {
			type: "InstanceCreated";
			data: {
				id: string;
				dto: InstanceDto;
			};
	  }
	| {
			type: "DProgress";
			data: {
				version: string;
				current: number;
				total: number;
				d_type: string;
			};
	  }
	| {
			type: "DFinish";
			data: {
				version: string;
			};
	  }
	| {
			type: "DError";
			data: {
				version: string;
				message: string;
			};
	  }
	| {
			type: "DFinishRuntime";
			data: {
				version: string;
			};
	  }
	| {
			type: "DEnqueue";
			data: {
				version: string;
			};
	  }
	| {
			type: "STChanged";
	  }
	| {
			type: "ThemeChanged";
			data: {
				id: string;
			};
	  };

export interface ConsoleOutputPayload {
	id: string;
	line: string;
	stream: "stdout" | "stderr";
}

export interface ThemeEntry {
	id: string;
	name: string;
	author: string;
	version: string;
	type: string;
}

export interface ModrinthFile {
	url: string;
	filename: string;
	primary: boolean;
}

export interface ModrinthDependency {
	dependency_type: string;
	project_id: string | null;
	version_id: string | null;
}

export interface ModrinthVersion {
	id: string;
	name: string;
	version_number: string;
	game_versions: string[];
	loaders: string[];
	date_published: string;
	files: ModrinthFile[];
	dependencies: ModrinthDependency[];
}

export interface ModrinthProject {
	project_id: string;
	slug: string;
	author: string;
	title: string;
	description: string;
	categories: string[];
	versions: string[];
	downloads: number;
	icon_url: string | null;
}

export interface ModrinthSearchResult {
	hits: ModrinthProject[];
	offset: number;
	limit: number;
	total_hits: number;
}

export type ModSource = "modrinth" | "curseforge";

export interface CurseForgeAuthor {
	name: string;
}

export interface CurseForgeLogo {
	url: string;
}

export interface CurseForgeCategory {
	id: number;
	name: string;
	slug: string;
}

export interface CurseForgeProject {
	id: number;
	name: string;
	slug: string;
	summary: string;
	logo: CurseForgeLogo | null;
	categories: CurseForgeCategory[];
	authors: CurseForgeAuthor[];
	downloadCount: number;
	gameId: number;
	dateCreated: string;
	dateModified: string;
	isAvailable: boolean;
	latestFiles: CurseForgeFile[];
	latestFilesIndexes: {
		gameVersion: string;
		fileId: number;
		filename: string;
		releaseType: number;
		modLoader: string;
	}[];
}

export interface CurseForgeSearchResult {
	data: CurseForgeProject[];
	pagination: {
		index: number;
		pageSize: number;
		resultCount: number;
		totalCount: number;
	};
}

export interface CurseForgeFile {
	id: number;
	modId: number;
	fileName: string;
	fileDate: string;
	downloadUrl: string | null;
	fileLength: number;
	gameVersions: string[];
	modLoaders: string[];
	isAvailable: boolean;
	releaseType: number;
}

export interface CurseForgeFilesResult {
	data: CurseForgeFile[];
	pagination: {
		index: number;
		pageSize: number;
		resultCount: number;
		totalCount: number;
	};
}

export interface CurseForgeModLoader {
	id: number;
	name: string;
	slug: string;
}

export interface MrpackInfo {
	name: string;
	version_id: string;
	summary: string | null;
	minecraft_version: string | null;
	loader: string | null;
	loader_version: string | null;
	file_count: number;
	version_id_for_instance?: string | null;
}

export interface CurseForgeGameVersion {
	id: number;
	name: string;
	slug: string;
}


