import { invoke } from "@tauri-apps/api/core";

export interface JarFile {
	file: string;
	name: string;
}

export interface MinecraftJarConfig {
	replacement: JarFile | null;
	mods: (JarFile & { enabled: boolean })[];
}

export type JarAction =
	| { type: "add"; paths: string[] }
	| { type: "replace"; path: string }
	| { type: "restore" }
	| { type: "remove"; file: string }
	| { type: "toggle"; file: string; enabled: boolean }
	| { type: "move"; file: string; offset: -1 | 1 };

export function getMinecraftJar(instanceId: string) {
	return invoke<MinecraftJarConfig>("get_instance_minecraft_jar", {
		instanceId,
	});
}

export function minecraftJarAction(instanceId: string, action: JarAction) {
	return invoke<MinecraftJarConfig>("instance_minecraft_jar_action", {
		instanceId,
		action,
	});
}
