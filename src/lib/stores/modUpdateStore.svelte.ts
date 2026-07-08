import {
	getModrinthLatestVersions,
	getInstanceModsMetadata,
	getInstanceMods,
} from "$lib/api/cubicApi";
import type { ModUpdateInfo, ModFileSource } from "$lib/types/types";

export interface ModUpdateState {
	checking: boolean;
	updates: ModUpdateInfo[];
}

const state = $state<ModUpdateState>({
	checking: false,
	updates: [],
});

export function useModUpdates() {
	async function checkUpdates(instanceId: string): Promise<ModUpdateInfo[]> {
		state.checking = true;
		state.updates = [];
		try {
			const mods = await getInstanceMods(instanceId);
			if (!mods || mods.length === 0) return [];

			const metadata = await getInstanceModsMetadata(instanceId);
			if (!metadata) return [];

			const entries = Object.entries(metadata);
			if (entries.length === 0) return [];

			const hashes: string[] = [];
			const hashToSource: Record<string, ModFileSource> = {};
			for (const [filename, src] of entries) {
				const mod = mods.find((m) => m.filename === filename);
				if (mod) {
					hashes.push(filename);
					hashToSource[filename] = { ...src, filename };
				}
			}

			if (hashes.length === 0) return [];

			const loaders = ["fabric", "forge", "neoforge", "quilt"];
			const result = await getModrinthLatestVersions(
				hashes,
				"sha1",
				loaders,
			);
			if (!result) return [];

			const updates: ModUpdateInfo[] = [];
			for (const [hash, version] of Object.entries(result)) {
				const src = hashToSource[hash];
				if (!src) continue;
				const isNewer = version.id !== src.version_id;
				const mod = mods.find((m) => m.filename === src.filename);
				updates.push({
					filename: src.filename,
					projectTitle:
						mod?.name ?? src.filename.replace(/\.jar$/, ""),
					iconUrl: mod?.icon ?? null,
					currentVersion: src.version_id,
					latestVersion: isNewer ? version.version_number : null,
					latestVersionId: isNewer ? version.id : null,
					upToDate: !isNewer,
					modrinthSource: src,
				});
			}

			state.updates = updates;
			return updates;
		} finally {
			state.checking = false;
		}
	}

	function clearUpdates() {
		state.updates = [];
	}

	return {
		get state() {
			return state;
		},
		checkUpdates,
		clearUpdates,
	};
}
