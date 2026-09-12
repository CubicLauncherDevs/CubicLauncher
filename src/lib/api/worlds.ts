import { Channel, invoke } from "@tauri-apps/api/core";

export interface WorldDto {
	folder: string;
	name: string;
	version: string | null;
	gameMode: number | null;
	hardcore: boolean;
	lastPlayed: number | null;
	icon: string | null;
	iconRevision: string | null;
	metadataError: boolean;
}

export interface WorldProgress {
	bytes: number;
	files: number;
}

export type WorldAction =
	| { type: "open"; folder: string | null }
	| { type: "openDatapacks"; folder: string }
	| { type: "import"; path: string }
	| { type: "export"; folder: string; path: string }
	| { type: "duplicate" | "delete" | "size" | "resetIcon"; folder: string }
	| { type: "rename"; folder: string; name: string };

export interface WorldResult {
	folder: string | null;
	size: number | null;
}

export function getInstanceWorlds(instanceId: string): Promise<WorldDto[]> {
	return invoke("get_instance_worlds", { instanceId });
}

export function worldAction(
	instanceId: string,
	action: WorldAction,
	onProgress: (progress: WorldProgress) => void,
): Promise<WorldResult> {
	const channel = new Channel<WorldProgress>();
	channel.onmessage = onProgress;
	return invoke("instance_world_action", {
		instanceId,
		action,
		onProgress: channel,
	});
}
