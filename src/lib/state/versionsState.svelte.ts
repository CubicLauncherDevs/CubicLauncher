import {
	getInstalledVersions,
	getInstalledMcVersions,
	getInstalledLoaderVersions,
} from "$lib/api/cubicApi";

export interface McVersions {
	vanilla: Set<string>;
	fabric: Set<string>;
	forge: Set<string>;
	neoforge: Set<string>;
	quilt: Set<string>;
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

export async function loadInstalledVersions(force = false): Promise<void> {
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
			const raw = await getInstalledVersions();
			state.rawVersions = raw;
			state.mcVersions = getInstalledMcVersions(raw);
			state.loaderVersions = getInstalledLoaderVersions(raw);
			state.loaded = true;
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
	state.loaded = false;
}

export function isVersionInstalled(versionId: string): boolean {
	return state.rawVersions ? state.rawVersions.includes(versionId) : false;
}
