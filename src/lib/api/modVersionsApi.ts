import { invoke } from "@tauri-apps/api/core";

export interface ModVersionRef {
	source: "modrinth" | "curseforge";
	project_id: string;
	version_id: string;
}

export const replaceInstanceMod = (
	id: string,
	request: {
		filename: string;
		expected_sha1: string;
		target: ModVersionRef;
		downloads: ModVersionRef[];
	},
) => invoke<string>("replace_instance_mod", { id, request });
