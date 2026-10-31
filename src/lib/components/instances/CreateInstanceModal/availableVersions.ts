import {
	getAvailableVersions,
	getFabricVersions,
	getLoaderVersionCatalog,
	getQuiltVersions,
} from "$lib/api/cubicApi";
import {
	catalogToGrouped,
	compareVersions,
	type GroupedLoaderVersions,
} from "$lib/components/layout/VersionDownloader/versionCatalog";

export async function getSelectionCatalog(
	loader: string,
): Promise<GroupedLoaderVersions> {
	// Forge, NeoForge y OptiFine llegan ya agrupados y ordenados por Rust.
	if (loader === "forge" || loader === "neoforge" || loader === "optifine") {
		return catalogToGrouped(await getLoaderVersionCatalog(loader));
	}
	const gameVersions =
		loader === "vanilla"
			? (await getAvailableVersions()).map((v) => v.id)
			: (
					await (loader === "fabric"
						? getFabricVersions()
						: getQuiltVersions())
				)
					.map((v) => v.version)
					.sort(compareVersions);
	return { byGame: new Map(), gameVersions, stableGameVersions: [] };
}
