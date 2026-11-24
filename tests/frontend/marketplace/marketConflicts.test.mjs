import { expect, test } from "bun:test";
import {
	detectInstalledConflicts,
	hasFatalConflict,
	toInstallConflicts,
} from "../../../src/lib/utils/marketConflicts.ts";

function node(id, overrides = {}) {
	return {
		source: "modrinth",
		project_id: id,
		version_id: "v2",
		title: id,
		icon_url: null,
		filename: `${id}.jar`,
		download_url: `https://example.test/${id}.jar`,
		kind: "required",
		depth: 0,
		children: [],
		...overrides,
	};
}

function mod(projectId, overrides = {}) {
	return {
		name: projectId,
		filename: `${projectId}.jar`,
		version: "1.0.0",
		description: null,
		authors: null,
		icon: null,
		enabled: true,
		sha1: "",
		file_size: 0,
		source: "modrinth",
		project_id: projectId,
		version_id: "v1",
		slug: null,
		...overrides,
	};
}

const resolution = (overrides = {}) => ({
	tree: [node("root", { children: [node("library")] })],
	conflicts: [],
	incompatible: [],
	...overrides,
});

test("an outdated required dependency is reported and blocks the install", () => {
	const conflicts = detectInstalledConflicts(resolution(), [
		mod("library", { version_id: "v1" }),
	]);
	expect(conflicts).toHaveLength(1);
	expect(conflicts[0].type).toBe("version_mismatch");
	// The metadata version when there is one, never the opaque provider id.
	expect(conflicts[0].installedVersion).toBe("1.0.0");
	expect(toInstallConflicts(conflicts)[0].installed_version).toBe("1.0.0");
	expect(hasFatalConflict(toInstallConflicts(conflicts))).toBe(true);
});

test("an installed mod without a metadata version is labelled by its file name", () => {
	const conflicts = detectInstalledConflicts(resolution(), [
		mod("library", { version: null, filename: "sodium-fabric-0.5.8.jar" }),
	]);
	expect(conflicts).toHaveLength(1);
	expect(conflicts[0].installedVersion).toBe("sodium-fabric-0.5.8");
	expect(conflicts[0].installedVersion).not.toMatch(/^[A-Za-z0-9]{8}$/);
});

test("a dependency already installed with the requested version is not a conflict", () => {
	const conflicts = detectInstalledConflicts(resolution(), [
		mod("library", { version_id: "v2" }),
	]);
	expect(conflicts).toHaveLength(0);
});

test("a disabled required dependency blocks and explains how to fix it", () => {
	const conflicts = detectInstalledConflicts(resolution(), [
		mod("library", {
			enabled: false,
			filename: "library.jar.disabled",
		}),
	]);
	expect(conflicts).toHaveLength(1);
	expect(conflicts[0].type).toBe("disabled");
	expect(conflicts[0].installedFilename).toBe("library.jar.disabled");
	expect(hasFatalConflict(toInstallConflicts(conflicts))).toBe(true);
});

test("a disabled optional dependency only warns", () => {
	const conflicts = detectInstalledConflicts(
		resolution({
			tree: [
				node("root", {
					children: [node("extras", { kind: "optional" })],
				}),
			],
		}),
		[mod("extras", { enabled: false })],
	);
	expect(conflicts).toHaveLength(1);
	expect(conflicts[0].type).toBe("disabled");
	expect(hasFatalConflict(toInstallConflicts(conflicts))).toBe(false);
});

test("declared incompatibilities with enabled mods become warnings", () => {
	const conflicts = detectInstalledConflicts(
		resolution({
			incompatible: [{ source: "modrinth", project_id: "optifine" }],
		}),
		[mod("optifine")],
	);
	expect(conflicts).toHaveLength(1);
	expect(conflicts[0].type).toBe("incompatible");
	expect(hasFatalConflict(toInstallConflicts(conflicts))).toBe(false);
});

test("incompatibilities with absent mods are ignored", () => {
	const conflicts = detectInstalledConflicts(
		resolution({
			incompatible: [{ source: "modrinth", project_id: "optifine" }],
		}),
		[],
	);
	expect(conflicts).toHaveLength(0);
});

test("an unresolved required dependency blocks and names the parents", () => {
	const conflicts = detectInstalledConflicts(
		resolution({
			tree: [
				node("root", {
					children: [
						node("missing", {
							version_id: null,
							filename: "",
							download_url: null,
							unresolved_reason: "No hay versión compatible",
						}),
					],
				}),
			],
			conflicts: [
				{
					project_id: "missing",
					source: "modrinth",
					title: "Missing mod",
					kind: "unresolved",
					requested_by: [
						{
							project_id: "root",
							source: "modrinth",
							version_id: "v2",
						},
					],
					requested_versions: [],
				},
			],
		}),
		[],
	);
	expect(conflicts).toHaveLength(1);
	expect(conflicts[0].type).toBe("unresolved");
	expect(conflicts[0].title).toBe("Missing mod");
	expect(conflicts[0].requestedBy[0].project_id).toBe("root");
	expect(hasFatalConflict(toInstallConflicts(conflicts))).toBe(true);
});

test("resolver version mismatches keep every parent that asked", () => {
	const conflicts = toInstallConflicts(
		detectInstalledConflicts(
			resolution({
				conflicts: [
					{
						project_id: "library",
						source: "modrinth",
						title: "Library",
						kind: "version_mismatch",
						requested_by: [
							{
								project_id: "root",
								source: "modrinth",
								version_id: "v1",
							},
							{
								project_id: "other",
								source: "modrinth",
								version_id: "v2",
							},
						],
						requested_versions: [],
					},
				],
			}),
			[],
		),
	);
	expect(conflicts).toHaveLength(1);
	expect(
		conflicts[0].requested_by.map((origin) => origin.project_id),
	).toEqual(["root", "other"]);
	expect(conflicts[0].type).toBe("version_mismatch");
	expect(conflicts[0].fatal).toBe(true);
});
