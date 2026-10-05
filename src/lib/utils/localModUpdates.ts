import type { ModVersionRef } from "$lib/api/modVersionsApi";
import type {
	DependencyResolutionResult,
	ResolvedDependency,
} from "$lib/types/dependency";
import type { MarketVersion } from "$lib/types/market";
import type { ModDto } from "$lib/types/types";
import { t } from "$lib/i18n";

export function canUpdateOwnMod(
	mod?: ModDto,
): mod is ModDto & { source: ModVersionRef["source"]; project_id: string } {
	return (
		!!mod &&
		mod.pack_name == null &&
		!mod.pack_locked &&
		!!mod.project_id &&
		!!mod.sha1 &&
		(mod.source === "modrinth" || mod.source === "curseforge")
	);
}

/** Follow the same release channel as the detail view and never downgrade a known version. */
export function latestModUpdate(
	versions: MarketVersion[],
	loader: string,
	gameVersion: string,
) {
	const compatible = versions.filter(
		(version) =>
			version.gameVersions.includes(gameVersion) &&
			version.loaders.some(
				(value) => value.toLowerCase() === loader.toLowerCase(),
			),
	);
	const current = compatible.find((version) => version.isInstalled);
	const candidates = compatible.filter(
		(version) =>
			!version.versionType ||
			version.versionType === "release" ||
			version.versionType === current?.versionType,
	);
	const latest = candidates.sort(
		(a, b) =>
			(Date.parse(b.datePublished) || 0) -
			(Date.parse(a.datePublished) || 0),
	)[0];
	if (!latest) return { status: "skipped" as const, version: null };
	if (
		latest.isInstalled ||
		(current &&
			Date.parse(latest.datePublished) <=
				Date.parse(current.datePublished))
	) {
		return { status: "current" as const, version: null };
	}
	return { status: "update" as const, version: latest };
}

/** The native replacer rechecks every file and skips identical dependencies. */
export function requiredUpdateVersions(
	resolution: DependencyResolutionResult,
	root: ModVersionRef,
	installed: ModDto[],
): ModVersionRef[] {
	const refs = new Map<string, ModVersionRef>();
	const key = (source: string, id: string) => `${source}:${id}`;
	function walk(nodes: ResolvedDependency[]) {
		for (const node of nodes) {
			if (node.kind === "optional" || node.kind === "embedded") continue;
			if (node.kind === "incompatible") {
				if (
					installed.some(
						(mod) =>
							mod.source === node.source &&
							mod.project_id === node.project_id &&
							mod.enabled,
					)
				) {
					throw new Error(
						t("market.quickInstall.conflict", { name: node.title }),
					);
				}
				continue;
			}
			if (!node.version_id)
				throw new Error(t("market.modVersions.missingTarget"));
			const id = key(node.source, node.project_id);
			const previous = refs.get(id);
			if (previous && previous.version_id !== node.version_id)
				throw new Error(t("market.quickInstall.dependencyConflict"));
			if (previous) continue;
			refs.set(id, {
				source: node.source,
				project_id: node.project_id,
				version_id: node.version_id,
			});
			walk(node.children);
		}
	}
	walk(resolution.tree);
	if (
		refs.get(key(root.source, root.project_id))?.version_id !==
		root.version_id
	)
		throw new Error(t("market.modVersions.missingTarget"));
	if (
		resolution.conflicts.some((conflict) =>
			refs.has(key(conflict.source, conflict.project_id)),
		)
	)
		throw new Error(t("market.quickInstall.dependencyConflict"));
	return [...refs.values()];
}
