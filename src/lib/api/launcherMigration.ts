import { Channel, invoke } from "@tauri-apps/api/core";

export type MigrationProvider = "official" | "multimc";
export interface MigrationCandidate {
	id: string;
	name: string;
	suggested_name: string;
	source: string;
	root: string;
	version: {
		mc_version: string;
		loader: "Vanilla" | Record<string, string>;
	} | null;
	error: string | null;
	shared_directory: boolean;
	reinstalls_components: boolean;
}
export interface MigrationScan {
	token: string;
	candidates: MigrationCandidate[];
	issues: string[];
}
export interface MigrationProgress {
	id: string;
	index: number;
	count: number;
	copied: number;
	total: number;
	file: string;
	phase: "preparing" | "copying" | "installing";
}
export interface MigrationResult {
	id: string;
	name: string;
	status: "success" | "error" | "cancelled";
	error: string | null;
}
export const scanLauncherMigration = (
	provider: MigrationProvider,
	root: string | null,
) => invoke<MigrationScan>("scan_launcher_migration", { provider, root });

export const cancelLauncherMigration = (token: string) =>
	invoke<void>("cancel_launcher_migration", { token });

export function migrateLauncherInstances(
	token: string,
	selections: { id: string; name: string }[],
	onProgress: (progress: MigrationProgress) => void,
) {
	const channel = new Channel<MigrationProgress>();
	channel.onmessage = onProgress;
	return invoke<MigrationResult[]>("migrate_launcher_instances", {
		token,
		selections,
		onProgress: channel,
	});
}
