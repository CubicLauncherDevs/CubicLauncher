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
	username: string;
	user: MinecraftUser | null;
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
}

export interface JreStatus {
	version: number;
	installed: boolean;
	java_version: string | null;
}

export type AccountType = "Cracked" | "Microsoft";

export interface MinecraftUser {
	username: string;
	uuid: string;
	access_token: string;
	refresh_token: string | null;
	user_type: AccountType;
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
