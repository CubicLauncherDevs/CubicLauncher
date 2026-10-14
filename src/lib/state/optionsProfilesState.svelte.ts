import {
	listOptionsProfiles,
	type OptionsProfile,
} from "$lib/api/optionsProfiles";

const state = $state<{
	profiles: OptionsProfile[];
	loaded: boolean;
	loading: boolean;
}>({
	profiles: [],
	loaded: false,
	loading: false,
});

export const optionsProfilesState = state;

let loadPromise: Promise<void> | null = null;

/** Carga (o recarga) la lista de perfiles globales de options.txt. */
export async function loadOptionsProfiles(force = false): Promise<void> {
	if (state.loading) return loadPromise ?? Promise.resolve();
	if (state.loaded && !force) return;

	state.loading = true;
	loadPromise = (async () => {
		try {
			state.profiles = await listOptionsProfiles();
			state.loaded = true;
		} finally {
			state.loading = false;
			loadPromise = null;
		}
	})();
	return loadPromise;
}

export function invalidateOptionsProfiles(): void {
	state.loaded = false;
}

export function findOptionsProfile(
	id: string | null | undefined,
): OptionsProfile | null {
	if (!id) return null;
	return state.profiles.find((p) => p.id === id) ?? null;
}
