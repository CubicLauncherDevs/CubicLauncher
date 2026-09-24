import { Channel, invoke } from "@tauri-apps/api/core";

export interface InstancesDirInfo {
	current_dir: string;
	custom_dir: string;
	default_dir: string;
	instance_count: number;
}

export interface InstancesMoveProgress {
	instance_index: number;
	instance_count: number;
	instance_name: string;
	bytes_current: number;
	bytes_total: number;
	file: string;
	/** "hardlink" o "copy": copia significa que el traslado perdió la optimización. */
	strategy: "hardlink" | "copy";
}

export interface InstancesMoveResult {
	moved: number;
	failed: string[];
}

export const getInstancesDirInfo = () =>
	invoke<InstancesDirInfo>("get_instances_dir_info");

export function changeInstancesDir(
	newDir: string,
	onProgress: (progress: InstancesMoveProgress) => void,
) {
	const channel = new Channel<InstancesMoveProgress>();
	channel.onmessage = onProgress;
	return invoke<InstancesMoveResult>("change_instances_dir", {
		newDir,
		onProgress: channel,
	});
}

export function resetInstancesDir(
	onProgress: (progress: InstancesMoveProgress) => void,
) {
	const channel = new Channel<InstancesMoveProgress>();
	channel.onmessage = onProgress;
	return invoke<InstancesMoveResult>("reset_instances_dir", {
		onProgress: channel,
	});
}

export const cancelInstancesDirChange = () =>
	invoke<void>("cancel_instances_dir_change");
