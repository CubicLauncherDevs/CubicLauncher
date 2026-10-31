import type {
	LoaderDisplayVersion,
	LoaderVersionCatalog,
} from "$lib/types/types";

export function compareVersions(a: string, b: string): number {
	const aParts = a.split(".").map((n) => parseInt(n, 10) || 0);
	const bParts = b.split(".").map((n) => parseInt(n, 10) || 0);
	for (let i = 0; i < Math.max(aParts.length, bParts.length); i++) {
		const difference = (bParts[i] ?? 0) - (aParts[i] ?? 0);
		if (difference) return difference;
	}
	return b.localeCompare(a, undefined, { numeric: true });
}

export function sortLoaderVersions(
	items: LoaderDisplayVersion[],
): LoaderDisplayVersion[] {
	return items.sort(
		(a, b) =>
			Number(b.stable) - Number(a.stable) ||
			compareVersions(a.display_version, b.display_version),
	);
}

export interface GroupedLoaderVersions {
	byGame: Map<string, LoaderDisplayVersion[]>;
	gameVersions: string[];
	stableGameVersions: string[];
}

/**
 * Adapta el catálogo que Rust ya agrupa y ordena. No reordena ni copia versiones:
 * solo arma el mapa que esperan las vistas.
 */
export function catalogToGrouped(
	catalog: LoaderVersionCatalog,
): GroupedLoaderVersions {
	const byGame = new Map<string, LoaderDisplayVersion[]>();
	const stableGameVersions: string[] = [];
	for (const game of catalog.games) {
		byGame.set(game.game_version, game.versions);
		if (game.has_stable) stableGameVersions.push(game.game_version);
	}
	return {
		byGame,
		gameVersions: catalog.games.map((game) => game.game_version),
		stableGameVersions,
	};
}

/** Per-modal cache. Share pending requests; failed requests can be retried. */
export function createCatalogCache<T>() {
	const requests = new Map<string, Promise<T>>();
	return {
		get(key: string, load: () => Promise<T>, refresh = false): Promise<T> {
			const cached = requests.get(key);
			if (cached && !refresh) return cached;
			const request = Promise.resolve()
				.then(load)
				.catch((error) => {
					if (requests.get(key) === request) requests.delete(key);
					throw error;
				});
			requests.set(key, request);
			return request;
		},
		clear() {
			requests.clear();
		},
	};
}
