import type { PackResolutions, PackUpdatePreview } from "$lib/api/modpackApi";

interface UpdateApi {
	preview: (
		id: string,
		versionId: string | null,
		path: string | null,
	) => Promise<PackUpdatePreview>;
	apply: (
		id: string,
		token: string,
		resolutions: PackResolutions,
	) => Promise<void>;
	cancel: (id: string, token: string) => Promise<void>;
}

/** Own staging for one instance, including responses arriving after unmount. */
export function createModpackUpdateSession(
	id: string,
	api: UpdateApi,
	onCleanupError: (error: unknown) => void,
) {
	let generation = 0;
	let disposed = false;
	let current: PackUpdatePreview | null = null;
	let applying: { token: string; abandoned: boolean } | null = null;

	function release(token: string) {
		void api.cancel(id, token).catch(onCleanupError);
	}

	function clear() {
		generation++;
		if (applying) applying.abandoned = true;
		if (current && current.token !== applying?.token)
			release(current.token);
		current = null;
	}

	return {
		clear,
		dispose() {
			disposed = true;
			clear();
		},
		async preview(versionId: string | null, path: string | null) {
			if (disposed || applying) return null;
			clear();
			const request = generation;
			const result = await api.preview(id, versionId, path);
			if (disposed || request !== generation) {
				release(result.token);
				return null;
			}
			current = result;
			return result;
		},
		async apply(token: string, resolutions: PackResolutions) {
			if (disposed || applying || current?.token !== token)
				throw new Error("Modpack preview is no longer available");
			const operation = { token, abandoned: false };
			applying = operation;
			try {
				await api.apply(id, token, resolutions);
				// Successful apply consumes its staging in the backend.
				if (current?.token === token) current = null;
			} catch (error) {
				// Never cancel staging until an in-flight apply has settled.
				if (operation.abandoned) release(token);
				throw error;
			} finally {
				applying = null;
			}
		},
	};
}
