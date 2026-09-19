import {
	getAvailableVersions,
	getFabricVersions,
	getForgeVersions,
	getNeoForgeVersions,
	getOptiFineVersions,
	getQuiltVersions,
} from "$lib/api/cubicApi";
import {
	compareVersions,
	groupLoaderVersions,
	type GroupedLoaderVersions,
} from "$lib/components/layout/VersionDownloader/versionCatalog";

export async function getSelectionCatalog(
	loader: string,
): Promise<GroupedLoaderVersions> {
	switch (loader) {
		case "forge":
			return groupLoaderVersions(
				(await getForgeVersions()).map((v) => ({
					...v,
					display_version: v.forge_version,
				})),
			);
		case "neoforge":
			return groupLoaderVersions(
				(await getNeoForgeVersions()).map((v) => ({
					...v,
					display_version: v.neoforge_version,
				})),
			);
		case "optifine":
			return groupLoaderVersions(
				(await getOptiFineVersions()).map((v) => ({
					...v,
					display_version: v.optifine_version,
				})),
			);
		default: {
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
	}
}
