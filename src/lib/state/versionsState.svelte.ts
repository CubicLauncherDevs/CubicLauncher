import { getInstalledVersions } from "$lib/api/cubicApi";
import {
	getInstalledMcVersions,
	getInstalledLoaderVersions,
} from "$lib/utils/versionUtils";

export interface McVersions {
	vanilla: Set<string>;
	fabric: Set<string>;
	forge: Set<string>;
	neoforge: Set<string>;
	quilt: Set<string>;
	optifine: Set<string>;
}

export type LoaderVersions = Map<string, Set<string>>;

const state = $state<{
	rawVersions: string[] | null;
	mcVersions: McVersions | null;
	loaderVersions: LoaderVersions | null;
	loading: boolean;
	loaded: boolean;
	error: string | null;
}>({
	rawVersions: null,
	mcVersions: null,
	loaderVersions: null,
	loading: false,
	loaded: false,
	error: null,
});

export const versionsState = state;

let loadPromise: Promise<void> | null = null;
let revision = 0;

export async function loadInstalledVersions(force = false): Promise<void> {
	if (force) invalidateInstalledVersions();
	if (state.loading) {
		return loadPromise ?? Promise.resolve();
	}
	if (state.loaded && !force) {
		return Promise.resolve();
	}

	state.loading = true;
	state.error = null;

	loadPromise = (async () => {
		try {
			// An install may finish during a scan. Coalesce refreshes into a
			// trailing scan instead of publishing an already outdated result.
			while (true) {
				const requestedRevision = revision;
				const raw = await getInstalledVersions();
				if (requestedRevision !== revision) continue;
				state.rawVersions = raw;
				state.mcVersions = getInstalledMcVersions(raw);
				state.loaderVersions = getInstalledLoaderVersions(raw);
				state.loaded = true;
				break;
			}
		} catch (e) {
			state.error = e instanceof Error ? e.message : String(e);
			state.loaded = false;
		} finally {
			state.loading = false;
			loadPromise = null;
		}
	})();

	return loadPromise;
}

export function invalidateInstalledVersions(): void {
	revision++;
	state.loaded = false;
}

export function isVersionInstalled(versionId: string): boolean {
	return state.rawVersions ? state.rawVersions.includes(versionId) : false;
}
