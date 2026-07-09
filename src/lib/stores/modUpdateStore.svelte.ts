import { getModrinthLatestVersions, getInstanceMods } from "$lib/api/cubicApi";
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

			// Collect SHA1s from mods that have a modrinth source
			const sha1s: string[] = [];
			const sha1ToMod: Record<
				string,
				{ mod: (typeof mods)[0]; source: ModFileSource }
			> = {};
			for (const mod of mods) {
				if (mod.source === "modrinth" && mod.project_id && mod.sha1) {
					sha1s.push(mod.sha1);
					sha1ToMod[mod.sha1] = {
						mod,
						source: {
							project_id: mod.project_id,
							version_id: mod.version ?? "",
							filename: mod.filename,
						},
					};
				}
			}

			if (sha1s.length === 0) return [];

			const loaders = ["fabric", "forge", "neoforge", "quilt"];
			const result = await getModrinthLatestVersions(
				sha1s,
				"sha1",
				loaders,
			);
			if (!result) return [];

			const updates: ModUpdateInfo[] = [];
			for (const [hash, version] of Object.entries(result)) {
				const entry = sha1ToMod[hash];
				if (!entry) continue;
				const isNewer = version.id !== entry.source.version_id;
				updates.push({
					filename: entry.mod.filename,
					projectTitle: entry.mod.name,
					iconUrl: entry.mod.icon ?? null,
					currentVersion: entry.source.version_id,
					latestVersion: isNewer ? version.version_number : null,
					latestVersionId: isNewer ? version.id : null,
					upToDate: !isNewer,
					modrinthSource: entry.source,
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
