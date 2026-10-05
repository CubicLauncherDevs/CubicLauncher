import type { ModDownloadInfo } from "$lib/api/cubicApi";
import type {
	DependencyRequest,
	DependencyResolutionResult,
	ResolvedDependency,
} from "$lib/types/dependency";
import type { ModDto } from "$lib/types/types";
import { t } from "$lib/i18n";

/** Required-only installation: never silently overwrite an existing local file. */
export function requiredDownloadQueue(
	resolution: DependencyResolutionResult,
	installed: ModDto[],
	root: DependencyRequest,
): ModDownloadInfo[] {
	const identity = (source: string, id: string) => `${source}:${id}`;
	const existing = new Map(
		installed
			.filter((mod) => mod.project_id)
			.map((mod) => [identity(mod.source, mod.project_id!), mod]),
	);
	const filenames = new Set(
		installed.map((mod) =>
			mod.filename.replace(/\.disabled$/i, "").toLowerCase(),
		),
	);
	const queued = new Map<string, ModDownloadInfo>();
	const visited = new Set<string>();
	const required = new Set<string>();
	let foundRoot = false;
	function walk(nodes: ResolvedDependency[]) {
		for (const node of nodes) {
			if (node.kind === "optional" || node.kind === "embedded") continue;
			const key = identity(node.source, node.project_id);
			const local = existing.get(key);
			if (node.kind === "incompatible") {
				if (local && local.enabled)
					throw new Error(
						t("market.quickInstall.conflict", { name: node.title }),
					);
				continue;
			}
			required.add(key);
			if (
				key === identity(root.source, root.project_id) &&
				node.version_id === root.version_id
			)
				foundRoot = true;
			if (visited.has(key)) continue;
			visited.add(key);
			if (local) {
				if (!local.enabled)
					throw new Error(
						t("market.quickInstall.disabledDependency", {
							name: local.name,
						}),
					);
			} else {
				if (!node.download_url || !node.filename || !node.version_id)
					throw new Error(
						t("market.quickInstall.noDownload", {
							name: node.title,
						}),
					);
				const filename = node.filename
					.replace(/\.disabled$/i, "")
					.toLowerCase();
				if (filenames.has(filename))
					throw new Error(
						t("market.quickInstall.fileExists", {
							name: node.filename,
						}),
					);
				filenames.add(filename);
				queued.set(key, {
					source: node.source,
					project_id: node.project_id,
					version_id: node.version_id,
					url: node.download_url,
					filename: node.filename,
					projectTitle: node.title,
					iconUrl: node.icon_url ?? undefined,
				});
			}
			walk(node.children);
		}
	}
	walk(resolution.tree);
	if (!foundRoot)
		throw new Error(
			t("market.quickInstall.noDownload", { name: root.project_id }),
		);
	if (
		resolution.conflicts.some((conflict) =>
			required.has(identity(conflict.source, conflict.project_id)),
		)
	) {
		throw new Error(t("market.quickInstall.dependencyConflict"));
	}
	return [...queued.values()];
}
