import type { McVersion } from "../types/types";

export interface VersionIntegrity {
	version_id: string;
	dependencies: string[];
	missing: string[];
	complete: boolean;
}

export interface VersionStatus {
	version_id: string;
	complete: boolean;
	missing_deps: string[];
}

export function parseInstalledVersion(raw: string): McVersion {
	if (raw.includes("fabric")) {
		const clean = raw
			.replace(/^fabric-loader-[\d.]+-/, "")
			.replace(/-fabric$/, "");
		return { loader: "fabric", version: clean, type: "" };
	}
	if (raw.includes("-neoforge-")) {
		const idx = raw.indexOf("-neoforge-");
		const mcVersion = raw.substring(0, idx);
		const neoforgeVersion = raw.substring(idx + 10);
		return {
			loader: "neoforge",
			version: `${mcVersion}-neoforge-${neoforgeVersion}`,
			type: "",
		};
	}
	if (raw.includes("-forge-")) {
		const idx = raw.indexOf("-forge-");
		const mcVersion = raw.substring(0, idx);
		const forgeVersion = raw.substring(idx + 7);
		return {
			loader: "forge",
			version: `${mcVersion}-forge-${forgeVersion}`,
			type: "",
		};
	}
	if (raw.includes("quilt-loader-")) {
		const clean = raw.replace(/^quilt-loader-[\d.]+-/, "");
		return { loader: "quilt", version: clean, type: "" };
	}
	return { loader: "vanilla", version: raw, type: "" };
}

export function getInstalledMcVersions(raw: string[]): {
	vanilla: Set<string>;
	fabric: Set<string>;
	forge: Set<string>;
	neoforge: Set<string>;
	quilt: Set<string>;
} {
	const vanilla = new Set<string>();
	const fabric = new Set<string>();
	const forge = new Set<string>();
	const neoforge = new Set<string>();
	const quilt = new Set<string>();
	for (const v of raw) {
		const parsed = parseInstalledVersion(v);
		if (parsed.loader === "vanilla") vanilla.add(parsed.version);
		else if (parsed.loader === "fabric") fabric.add(parsed.version);
		else if (parsed.loader === "forge") forge.add(parsed.version);
		else if (parsed.loader === "neoforge") neoforge.add(parsed.version);
		else if (parsed.loader === "quilt") quilt.add(parsed.version);
	}
	return { vanilla, fabric, forge, neoforge, quilt };
}

function addInstalledLoaderVersion(
	map: Map<string, Set<string>>,
	loader: string,
	mcVersion: string,
	loaderVersion: string,
) {
	const key = `${loader}:${mcVersion}`;
	if (!map.has(key)) {
		map.set(key, new Set<string>());
	}
	map.get(key)!.add(loaderVersion);
}

/**
 * Devuelve un mapa con las versiones de loader instaladas, agrupadas por
 * loader y versión de Minecraft. La clave es `${loader}:${mcVersion}` y el
 * valor es el conjunto de versiones del loader instaladas para esa
 * combinación.
 */
export function getInstalledLoaderVersions(
	raw: string[],
): Map<string, Set<string>> {
	const result = new Map<string, Set<string>>();

	for (const v of raw) {
		if (v.includes("fabric-loader-")) {
			const clean = v.replace(/-fabric$/, "");
			const prefix = "fabric-loader-";
			const rest = clean.substring(prefix.length);
			const dash = rest.indexOf("-");
			if (dash === -1) continue;
			const loaderVersion = rest.substring(0, dash);
			const mcVersion = rest.substring(dash + 1);
			addInstalledLoaderVersion(
				result,
				"fabric",
				mcVersion,
				loaderVersion,
			);
		} else if (v.includes("quilt-loader-")) {
			const prefix = "quilt-loader-";
			const rest = v.substring(prefix.length);
			const dash = rest.indexOf("-");
			if (dash === -1) continue;
			const loaderVersion = rest.substring(0, dash);
			const mcVersion = rest.substring(dash + 1);
			addInstalledLoaderVersion(
				result,
				"quilt",
				mcVersion,
				loaderVersion,
			);
		} else if (v.includes("-neoforge-")) {
			const idx = v.indexOf("-neoforge-");
			const mcVersion = v.substring(0, idx);
			const loaderVersion = v.substring(idx + 10);
			addInstalledLoaderVersion(
				result,
				"neoforge",
				mcVersion,
				loaderVersion,
			);
		} else if (v.includes("-forge-")) {
			const idx = v.indexOf("-forge-");
			const mcVersion = v.substring(0, idx);
			const loaderVersion = v.substring(idx + 7);
			addInstalledLoaderVersion(
				result,
				"forge",
				mcVersion,
				loaderVersion,
			);
		}
	}

	return result;
}
