import { afterEach, beforeEach, expect, mock, spyOn, test } from "bun:test";
import { plugin, Transpiler } from "bun";
import { heapStats } from "bun:jsc";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { compileModule } from "svelte/compiler";
import { flushSync } from "svelte";
import { effect_root } from "svelte/internal/client";

// Bun strips types before Svelte compiles runes; browser conditions select the
// client runtime, so effects can run without a DOM or a mounted component.
const lib = resolve(import.meta.dir, "../../../src/lib");
const transpiler = new Transpiler({ loader: "ts", target: "browser" });
plugin({
	name: "market-state-runes",
	setup(build) {
		build.onResolve({ filter: /^\$lib\// }, ({ path }) => ({
			path: Bun.resolveSync(path.replace(/^\$lib/, lib), import.meta.dir),
		}));
		build.onLoad(
			{ filter: /marketState\.svelte\.ts$/ },
			async ({ path }) => ({
				contents: compileModule(
					transpiler.transformSync(await readFile(path, "utf8")),
					{
						filename: path,
						generate: "client",
					},
				).js.code,
				loader: "js",
			}),
		);
	},
});

function deferred() {
	let resolve;
	let reject;
	const promise = new Promise((yes, no) => {
		resolve = yes;
		reject = no;
	});
	return { promise, resolve, reject };
}

function file(filename, overrides = {}) {
	return {
		filename,
		name: filename,
		version: "1.0",
		description: "Shared project",
		authors: ["Author"],
		icon: "https://example.test/icon.png",
		enabled: true,
		sha1: filename,
		file_size: 100,
		source: "modrinth",
		project_id: "shared",
		slug: "shared-project",
		...overrides,
	};
}

function result(source, id = "shared", total = 1) {
	return source === "modrinth"
		? {
				hits: [
					{
						project_id: id,
						title: id,
						description: "Remote metadata",
						author: "Author",
						icon_url: null,
						downloads: 10,
						slug: id,
						categories: [],
						versions: [],
					},
				],
				total_hits: total,
				offset: 0,
				limit: 20,
			}
		: {
				data: [
					{
						id: Number(id),
						name: id,
						summary: "Remote metadata",
						authors: [{ name: "Author" }],
						logo: null,
						downloadCount: 10,
						slug: id,
						categories: [],
						latestFiles: [],
						latestFilesIndexes: [],
						dateCreated: "",
						dateModified: "",
						isAvailable: true,
					},
				],
				pagination: {
					totalCount: total,
					index: 0,
					pageSize: 20,
					resultCount: 1,
				},
			};
}

function page(provider, ids, total) {
	const response = result(provider, ids[0] ?? "1", total);
	const key = provider === "modrinth" ? "hits" : "data";
	response[key] = ids.map((id) => result(provider, id)[key][0]);
	return response;
}

let disk;
let refreshLocal;
const unregisterRefresh = mock();
const scan = mock(async (_uuid) => structuredClone(disk));
const getInstanceModpack = mock(async (_uuid) => null);
const replaceInstanceMod = mock(async (_uuid, _request) => "updated.jar");
const searchModrinth = mock(async (..._args) => result("modrinth"));
const searchCurseForge = mock(async (..._args) => result("curseforge", "42"));
const getModrinthProject = mock(async (_id) => null);
const getModrinthProjectVersions = mock(async (..._args) => []);
const getCurseForgeProject = mock(async (_id) => null);
const getCurseForgeProjectFiles = mock(async (..._args) => []);
const getCurseForgeProjectDescription = mock(async (_id) => "description");
const deleteInstanceFile = mock(async (_uuid, _dir, filename) => {
	disk = disk.filter((entry) => entry.filename !== filename);
});
const toggleInstanceMod = mock(async (_uuid, filename, enabled) => {
	const entry = disk.find((entry) => entry.filename === filename);
	if (!entry) throw new Error(`Missing file: ${filename}`);
	entry.enabled = enabled;
	entry.filename = enabled
		? filename.replace(/\.disabled$/, "")
		: `${filename}.disabled`;
});
const api = {
	addInstanceFile: mock(async (_uuid, _dir, path) => {
		disk.push(file(path.split(/[\\/]/).pop()));
	}),
	getInstanceMods: scan,
	getInstanceModIcons: mock(async (_uuid, files) =>
		files.map((file) => ({
			...file,
			icon: "data:image/png;base64,fixture",
		})),
	),
	getInstanceResourcePacks: scan,
	getInstanceShaderPacks: scan,
	searchModrinth,
	searchCurseForge,
	getModrinthProject,
	getModrinthProjectVersions,
	getCurseForgeProject,
	getCurseForgeProjectFiles,
	getCurseForgeProjectDescription,
	deleteInstanceFile,
	toggleInstanceMod,
	downloadMods: mock(async () => {}),
	downloadResourcePacks: mock(async () => {}),
	downloadShaderPacks: mock(async () => {}),
	resolveModDependencies: mock(async () => ({ tree: [], conflicts: [] })),
};
mock.module(`${lib}/api/cubicApi`, () => api);
mock.module(`${lib}/api/modpackApi`, () => ({ getInstanceModpack }));
mock.module(`${lib}/api/modVersionsApi`, () => ({ replaceInstanceMod }));
mock.module(`${lib}/api/launcherService`, () => ({
	registerModsRefreshCallback: (_uuid, callback) => {
		refreshLocal = callback;
		return unregisterRefresh;
	},
}));
mock.module(`${lib}/state/state.svelte`, () => ({ showWarning: mock() }));
mock.module(`${lib}/i18n`, () => ({ t: (key) => key }));

const { createMarketState } =
	await import("../../../src/lib/state/marketState.svelte.ts");
const { getMarketProjectId, localModToMarket } =
	await import("../../../src/lib/types/market");
const { InstState } = await import("../../../src/lib/types/types");
const instance = {
	uuid: "instance",
	name: "Test",
	version: "1.21.1",
	loader: "fabric",
	status: InstState.Off,
	last_played: 0,
	cover_image: null,
	icon: null,
	path: "/fixture",
	overrides: null,
	pinned: false,
};
let state;
let dispose;
async function settle() {
	await Bun.sleep(0);
	flushSync();
}
async function start(content = "mods") {
	dispose = effect_root(() => {
		state = createMarketState(instance, content);
	});
	flushSync();
	await settle();
}
async function source(value) {
	state.setSource(value);
	await Bun.sleep(230);
	await settle();
}
function localIds() {
	return state.items.map((item) => item.id).sort();
}

// Compare plain data: Bun's nested partial matcher mishandles Svelte proxies.
function snapshot(value) {
	return JSON.parse(JSON.stringify(value));
}

beforeEach(() => {
	instance.status = InstState.Off;
	instance.version = "1.21.1";
	unregisterRefresh.mockClear();
	getInstanceModpack.mockReset().mockResolvedValue(null);
	replaceInstanceMod.mockReset().mockResolvedValue("updated.jar");
	disk = [file("a.jar"), file("b.jar")];
	for (const fn of new Set(Object.values(api))) fn.mockClear();
	scan.mockReset().mockImplementation(async () => structuredClone(disk));
	getModrinthProjectVersions.mockReset().mockResolvedValue([]);
	getCurseForgeProjectFiles.mockReset().mockResolvedValue([]);
	api.resolveModDependencies
		.mockReset()
		.mockResolvedValue({ tree: [], conflicts: [] });
	api.downloadMods.mockReset().mockResolvedValue(undefined);
	api.downloadResourcePacks.mockReset().mockResolvedValue(undefined);
	api.downloadShaderPacks.mockReset().mockResolvedValue(undefined);
	searchModrinth
		.mockReset()
		.mockImplementation(async () => result("modrinth"));
	searchCurseForge
		.mockReset()
		.mockImplementation(async () => result("curseforge", "42"));
});
afterEach(() => {
	if (dispose) {
		state.destroy();
		dispose();
		dispose = undefined;
	}
	flushSync();
});

test("local mods default to own; ownership composes with status, provider and search", async () => {
	getInstanceModpack.mockResolvedValue({ name: "Adventure" });
	disk = [
		file("own.jar", { pack_name: null }),
		file("pack.jar", { pack_name: "Adventure", pack_locked: true }),
		file("extra.jar.disabled", {
			pack_name: "Adventure",
			pack_locked: false,
			enabled: false,
			source: "local",
		}),
	];
	await start();
	await source("local");
	expect(localIds()).toEqual(["local-own.jar"]);
	state.setLocalOwnership("pack");
	expect(localIds()).toEqual(["local-extra.jar.disabled", "local-pack.jar"]);
	state.setLocalStatus("disabled");
	expect(localIds()).toEqual(["local-extra.jar.disabled"]);
	state.setLocalSource("modrinth");
	expect(localIds()).toEqual([]);
	state.setLocalSource("local");
	state.setQuery("extra");
	expect(localIds()).toEqual(["local-extra.jar.disabled"]);
	state.setQuery("missing");
	expect(localIds()).toEqual([]);
});

test("normal instances keep mods with absent or null pack ownership in the default list", async () => {
	disk = [
		file("unknown.jar"),
		file("own.jar", { pack_name: null, pack_locked: false }),
	];
	await start();
	await source("local");
	expect(state.hasModpack).toBe(false);
	state.setLocalOwnership("pack");
	expect(localIds()).toEqual(["local-own.jar", "local-unknown.jar"]);
	state.selectAllLocal();
	expect([...state.checkedFiles].sort()).toEqual(["own.jar", "unknown.jar"]);
});

test("pack ownership controls remain available for a pack with no installed mods", async () => {
	disk = [];
	getInstanceModpack.mockResolvedValue({ name: "Empty pack", files: {} });
	await start();
	await source("local");
	expect(state.hasModpack).toBe(true);
	state.setLocalOwnership("pack");
	expect(state.filters.localOwnership).toBe("pack");
	expect(state.items).toHaveLength(0);
});

test("refreshing a normal instance drops the effect of a previous pack filter", async () => {
	getInstanceModpack.mockResolvedValue({ name: "Pack" });
	await start();
	await source("local");
	state.setLocalOwnership("pack");
	expect(state.items).toHaveLength(0);
	getInstanceModpack.mockResolvedValue(null);
	await state.refresh();
	expect(state.hasModpack).toBe(false);
	expect(localIds()).toEqual(["local-a.jar", "local-b.jar"]);
});

test("corrupt mod metadata surfaces scan errors and blocks install preparation", async () => {
	scan.mockRejectedValue(Error("Corrupt pack metadata"));
	await start();
	await source("local");
	expect(state.error).toContain("Corrupt pack metadata");
	expect(state.items).toHaveLength(0);
	await expect(
		state.prepareInstall(
			{ id: "project", source: "modrinth" },
			{ id: "v2" },
		),
	).rejects.toThrow("Corrupt pack metadata");
	expect(api.resolveModDependencies).not.toHaveBeenCalled();
	expect(api.downloadMods).not.toHaveBeenCalled();
});

test("legacy protected mods with unknown ownership are not inferred as own or pack", async () => {
	getInstanceModpack.mockResolvedValue({
		name: "Pack",
		needs_inventory: true,
	});
	disk = [
		file("unknown.jar", { pack_locked: true }),
		file("unidentified.jar", { pack_name: null, pack_locked: true }),
		file("own.jar", { pack_name: null, pack_locked: false }),
		file("pack.jar", { pack_name: "Pack", pack_locked: true }),
	];
	await start();
	await source("local");
	expect(localIds()).toEqual(["local-own.jar"]);
	state.setLocalOwnership("pack");
	expect(localIds()).toEqual(["local-pack.jar"]);
	state.setLocalOwnership("all");
	expect(localIds()).toEqual([
		"local-own.jar",
		"local-pack.jar",
		"local-unidentified.jar",
		"local-unknown.jar",
	]);
	state.selectAllLocal();
	expect([...state.checkedFiles]).toEqual(["own.jar"]);
	// Recovery identifies one file as pack-owned and the other as a user mod.
	disk[0].pack_name = "Pack";
	disk[1].pack_locked = false;
	refreshLocal();
	await settle();
	state.setLocalOwnership("pack");
	expect(localIds()).toEqual(["local-pack.jar", "local-unknown.jar"]);
	state.setLocalOwnership("own");
	expect(localIds()).toEqual(["local-own.jar", "local-unidentified.jar"]);
});

test("selection excludes protected files, clears across ownership filters, and refresh reconciles locks", async () => {
	getInstanceModpack.mockResolvedValue({ name: "Pack" });
	disk = [
		file("own.jar"),
		file("pack.jar", { pack_name: "Pack", pack_locked: true }),
	];
	await start();
	await source("local");
	state.selectAllLocal();
	expect([...state.checkedFiles]).toEqual(["own.jar"]);
	state.setLocalOwnership("all");
	expect(state.checkedFiles.size).toBe(0);
	state.toggleChecked("pack.jar");
	state.selectAllLocal();
	expect([...state.checkedFiles]).toEqual(["own.jar"]);
	state.selectProject("local-pack.jar");
	disk[1].pack_locked = false;
	refreshLocal();
	await settle();
	expect(state.selectedProject.installed.pack_locked).toBe(false);
	state.selectAllLocal();
	expect([...state.checkedFiles].sort()).toEqual(["own.jar", "pack.jar"]);
	disk[1].pack_locked = true;
	refreshLocal();
	await settle();
	expect([...state.checkedFiles]).toEqual(["own.jar"]);
	expect(state.selectedProject.installed.pack_locked).toBe(true);
});

test("explicit batch targets and disabled-suffix imports cannot modify protected pack mods", async () => {
	getInstanceModpack.mockResolvedValue({ name: "Pack" });
	disk = [
		file("own.jar"),
		file("pack.jar", { pack_name: "Pack", pack_locked: true }),
	];
	await start();
	await source("local");
	state.setLocalOwnership("all");
	await state.manageLocal("delete", ["own.jar", "pack.jar"]);
	expect(deleteInstanceFile).toHaveBeenCalledTimes(1);
	expect(disk.map((mod) => mod.filename)).toEqual(["pack.jar"]);
	expect(state.localOperationReport.failures[0].error).toContain(
		"modpack.protected",
	);
	await state.manageLocal("disable", ["pack.jar"]);
	expect(toggleInstanceMod).not.toHaveBeenCalled();
	await state.importLocal(["/fixture/pack.jar.disabled"]);
	expect(api.addInstanceFile).not.toHaveBeenCalled();
	expect(state.localOperationReport.failures[0].error).toContain(
		"modpack.protected",
	);
});

test("install confirmation checks fresh pack metadata including dependency targets", async () => {
	await start();
	const project = state.items[0];
	disk = [
		file("protected.jar", {
			project_id: "dependency",
			pack_name: "Pack",
			pack_locked: true,
		}),
	];
	const log = spyOn(console, "error").mockImplementation(() => {});
	try {
		await expect(
			state.confirmInstall(project, [
				{
					url: "https://example.test/mod.jar",
					filename: "new-name.jar",
					project_id: "dependency",
				},
			]),
		).rejects.toThrow("modpack.protected");
		expect(api.downloadMods).not.toHaveBeenCalled();
	} finally {
		log.mockRestore();
	}
});

test("resource and shader pack lists are not filtered by mod ownership", async () => {
	disk = [file("pack.zip", { pack_name: "Pack" })];
	await start("resourcepacks");
	await source("local");
	expect(localIds()).toEqual(["local-pack.zip"]);
});

test("same-project shader files have distinct selectable IDs and details use the provider ID", async () => {
	disk = [file("a.zip"), file("b.zip")];
	await start("shaderpacks");
	await source("local");
	expect(localIds()).toEqual(["local-a.zip", "local-b.zip"]);
	for (const entry of disk) {
		state.selectProject(`local-${entry.filename}`);
		await settle();
		expect(state.selectedProject?.installed?.filename).toBe(entry.filename);
		expect(getMarketProjectId(state.selectedProject)).toBe("shared");
		expect(getModrinthProject).toHaveBeenLastCalledWith(
			"shared",
			expect.any(AbortSignal),
		);
		expect(getModrinthProjectVersions).toHaveBeenLastCalledWith(
			"shared",
			"",
			"1.21.1",
			expect.any(AbortSignal),
		);
	}
	await state.toggleEnabled(state.selectedProject);
	expect(toggleInstanceMod).not.toHaveBeenCalled();
	expect(disk.every((entry) => entry.enabled)).toBe(true);
});

test("installed filters search filenames and select only matching enabled files", async () => {
	disk = [
		file("sodium.jar", { name: "Renderer", authors: ["CaffeineMC"] }),
		file("other.jar.disabled", { name: "Renderer", enabled: false }),
		file("local.jar", { source: "local", name: "Local file" }),
	];
	await start();
	await source("local");
	state.setQuery("SODIUM.JAR");
	expect(state.items.map((item) => item.installed.filename)).toEqual([
		"sodium.jar",
	]);
	state.selectAllLocal();
	expect([...state.checkedFiles]).toEqual(["sodium.jar"]);
	state.setQuery("Renderer");
	expect(state.checkedFiles.size).toBe(0);
	state.setLocalStatus("disabled");
	state.selectAllLocal();
	expect([...state.checkedFiles]).toEqual(["other.jar.disabled"]);
	expect(state.localCount).toBe(3);
	state.clearFilters();
	expect(state.total).toBe(2);
	state.setQuery("caffeinemc");
	expect(state.items).toHaveLength(1);
});

test("bulk actions limit concurrency, coalesce enrichment and scan only once", async () => {
	disk = Array.from({ length: 40 }, (_, i) => file(`${i}.jar`));
	await start();
	await source("local");
	state.selectAllLocal();
	const scans = scan.mock.calls.length;
	let active = 0;
	let peak = 0;
	const implementation = deleteInstanceFile.getMockImplementation();
	deleteInstanceFile.mockImplementation(async (uuid, dir, filename) => {
		active++;
		peak = Math.max(peak, active);
		refreshLocal();
		await Bun.sleep(1);
		await implementation(uuid, dir, filename);
		active--;
	});
	try {
		const operation = state.manageLocal("delete");
		expect(state.localOperationBusy).toBe(true);
		await state.manageLocal("delete"); // A second click cannot start another pool.
		await operation;
		expect(peak).toBe(4);
		expect(deleteInstanceFile).toHaveBeenCalledTimes(40);
		expect(scan.mock.calls.length - scans).toBe(1);
		expect(state.localCount).toBe(0);
		expect(state.checkedFiles.size).toBe(0);
		expect(snapshot(state.localOperationReport)).toEqual({
			total: 40,
			completed: 40,
			succeeded: 40,
			failures: [],
		});
	} finally {
		deleteInstanceFile.mockImplementation(implementation);
	}
});

test("partial batch failures remain selected and can be retried", async () => {
	await start();
	await source("local");
	state.selectAllLocal();
	deleteInstanceFile.mockRejectedValueOnce(new Error("File locked"));
	await state.manageLocal("delete");
	expect(state.items).toHaveLength(1);
	expect([...state.checkedFiles]).toEqual(["a.jar"]);
	expect(state.localOperationReport.succeeded).toBe(1);
	expect(snapshot(state.localOperationReport.failures)).toEqual([
		{ filename: "a.jar", error: "Error: File locked" },
	]);
	await state.manageLocal("delete");
	expect(state.items).toHaveLength(0);
	expect(state.localOperationReport.failures).toHaveLength(0);
});

test("bulk enable skips already enabled mods and preserves the open detail after rename", async () => {
	disk = [file("a.jar.disabled", { enabled: false }), file("b.jar")];
	await start();
	await source("local");
	state.selectProject("local-a.jar.disabled");
	state.selectAllLocal();
	await state.manageLocal("enable");
	expect(toggleInstanceMod).toHaveBeenCalledTimes(1);
	expect(toggleInstanceMod).toHaveBeenCalledWith(
		"instance",
		"a.jar.disabled",
		true,
		true,
	);
	expect(state.selectedProject.installed.filename).toBe("a.jar");
	expect(state.checkedFiles.size).toBe(0);
});

test("enabling a mod never replaces a second file with the same base name", async () => {
	disk = [file("a.jar"), file("a.jar.disabled", { enabled: false })];
	await start();
	await source("local");
	state.toggleChecked("a.jar.disabled");
	await state.manageLocal("enable");
	expect(toggleInstanceMod).not.toHaveBeenCalled();
	expect(disk).toHaveLength(2);
	expect(state.localOperationReport.failures[0].filename).toBe(
		"a.jar.disabled",
	);
	expect([...state.checkedFiles]).toEqual(["a.jar.disabled"]);
});

for (const contentType of ["resourcepacks", "shaderpacks"]) {
	test(`${contentType}: multi-file import skips collisions and invalid formats, and never toggles game activation`, async () => {
		disk = [file("existing.zip")];
		await start(contentType);
		await source("local");
		const scans = scan.mock.calls.length;
		await state.importLocal([
			"/tmp/new.zip",
			"/tmp/new.zip",
			"/other/new.zip",
			"/tmp/existing.zip",
			"/tmp/mod.jar",
		]);
		expect(api.addInstanceFile).toHaveBeenCalledTimes(1);
		expect(api.addInstanceFile).toHaveBeenCalledWith(
			"instance",
			contentType,
			"/tmp/new.zip",
			false,
		);
		expect(state.localOperationReport.total).toBe(4);
		expect(state.localOperationReport.failures).toHaveLength(3);
		expect(scan.mock.calls.length - scans).toBe(1);
		state.selectAllLocal();
		await state.manageLocal("disable");
		expect(toggleInstanceMod).not.toHaveBeenCalled();
		await state.manageLocal("delete");
		expect(state.localCount).toBe(0);
	});
}

test("closing the instance view stops queued operations and prevents late rescans", async () => {
	disk = Array.from({ length: 12 }, (_, i) => file(`${i}.jar`));
	await start();
	await source("local");
	state.selectAllLocal();
	const gate = deferred();
	for (let i = 0; i < 4; i++)
		deleteInstanceFile.mockImplementationOnce(() => gate.promise);
	const scans = scan.mock.calls.length;
	const operation = state.manageLocal("delete");
	expect(deleteInstanceFile).toHaveBeenCalledTimes(4);
	state.destroy();
	expect(state.localOperationReport).toBeNull();
	gate.resolve();
	await operation;
	expect(deleteInstanceFile).toHaveBeenCalledTimes(4);
	expect(scan.mock.calls.length).toBe(scans);
	expect(state.items).toHaveLength(0);
});

test("busy instances reject import and bulk operations", async () => {
	await start();
	await source("local");
	state.selectAllLocal();
	instance.status = InstState.Started;
	try {
		await state.importLocal(["/tmp/new.jar"]);
		await state.manageLocal("delete");
		await state.manageLocal("disable");
		expect(api.addInstanceFile).not.toHaveBeenCalled();
		expect(deleteInstanceFile).not.toHaveBeenCalled();
		expect(toggleInstanceMod).not.toHaveBeenCalled();
	} finally {
		instance.status = InstState.Off;
	}
});

for (const count of [50, 100, 200]) {
	test(`${count} mods: only visible icons load and unchanged scans preserve every row`, async () => {
		disk = Array.from({ length: count }, (_, i) =>
			file(`mod-${String(i).padStart(3, "0")}.jar`, {
				icon: null,
				icon_revision: String(i),
			}),
		);
		await start();
		await source("local");
		expect(scan).toHaveBeenCalledWith("instance", false);
		expect(api.getInstanceModIcons).not.toHaveBeenCalled();
		const before = state.items;
		state.ensureRange(0, 15);
		await Bun.sleep(150);
		expect(api.getInstanceModIcons).toHaveBeenCalledTimes(1);
		expect(api.getInstanceModIcons.mock.calls[0][1]).toHaveLength(16);
		expect(state.items).toBe(before); // Icon arrival does not reconstruct the list.
		expect(state.getLocalIcon(state.items[0])).toBe(
			"data:image/png;base64,fixture",
		);
		expect(state.getLocalIcon(state.items[20])).toBeNull();
		const startTime = performance.now();
		await state.refresh();
		expect(state.items).toBe(before);
		const rescanMs = performance.now() - startTime;
		const searchStart = performance.now();
		for (let i = 0; i < 100; i++) state.setQuery(i % 2 ? "mod-0" : "mod-1");
		state.setQuery("");
		const searchMs = performance.now() - searchStart;
		console.log(
			`[installed UI] n=${count}: mock rescan=${rescanMs.toFixed(2)}ms, 100 filtered queries=${searchMs.toFixed(2)}ms; 16 icons requested, ${count} row objects reused`,
		);
		const rows = state.items;
		disk[10].description = "Changed externally";
		await state.refresh();
		expect(state.items.filter((item, i) => item !== rows[i])).toHaveLength(
			1,
		);
	});
}

test("fast scrolling coalesces icon requests to the final visible range", async () => {
	disk = Array.from({ length: 200 }, (_, i) =>
		file(`mod-${String(i).padStart(3, "0")}.jar`, {
			icon: null,
			icon_revision: String(i),
		}),
	);
	await start();
	await source("local");
	for (let i = 0; i < 120; i++) state.ensureRange(i, i + 15);
	await Bun.sleep(150);
	expect(api.getInstanceModIcons).toHaveBeenCalledTimes(1);
	expect(
		api.getInstanceModIcons.mock.calls[0][1].map((item) => item.filename),
	).toEqual(
		state.items.slice(119, 135).map((item) => item.installed.filename),
	);
});

test("icons from an older file revision are ignored, then the visible replacement is loaded", async () => {
	disk = [file("a.jar", { icon: null, icon_revision: "old" })];
	await start();
	await source("local");
	const pending = deferred();
	api.getInstanceModIcons.mockImplementationOnce(() => pending.promise);
	state.ensureRange(0, 0);
	await Bun.sleep(90);
	disk[0].icon_revision = "new";
	await state.refresh();
	pending.resolve([
		{ filename: "a.jar", revision: "old", icon: "stale-icon" },
	]);
	await settle();
	expect(state.getLocalIcon(state.items[0])).toBeNull();
	await Bun.sleep(150);
	expect(api.getInstanceModIcons).toHaveBeenCalledTimes(2);
	expect(api.getInstanceModIcons.mock.calls[1][1]).toEqual([
		{ filename: "a.jar", revision: "new" },
	]);
	expect(state.getLocalIcon(state.items[0])).toBe(
		"data:image/png;base64,fixture",
	);
});

test("late icon responses cannot repopulate a closed market", async () => {
	disk = [file("a.jar", { icon: null, icon_revision: "1" })];
	await start();
	await source("local");
	const project = state.items[0];
	const pending = deferred();
	api.getInstanceModIcons.mockImplementationOnce(() => pending.promise);
	state.ensureRange(0, 0);
	await Bun.sleep(90);
	state.destroy();
	pending.resolve([{ filename: "a.jar", revision: "1", icon: "late" }]);
	await settle();
	expect(state.getLocalIcon(project)).toBeNull();
	expect(api.getInstanceModIcons).toHaveBeenCalledTimes(1);
});

test("enrichment preserves local IDs and selection, including CurseForge detail routing", async () => {
	disk = [
		file("a.jar", { source: "local", project_id: null }),
		file("b.jar"),
	];
	await start();
	await source("local");
	state.selectProject("local-a.jar");
	disk[0] = file("a.jar", {
		source: "curseforge",
		project_id: "42",
		name: "Enriched name",
	});
	refreshLocal();
	await settle();
	expect(localIds()).toEqual(["local-a.jar", "local-b.jar"]);
	expect(state.selectedId).toBe("local-a.jar");
	expect(state.selectedProject?.title).toBe("Enriched name");
	expect(state.selectedProject?.curseforgeProjectId).toBe("42");
	expect(getMarketProjectId(state.selectedProject)).toBe("42");
	state.selectProject(state.selectedId);
	await settle();
	expect(getCurseForgeProject).toHaveBeenLastCalledWith(
		42,
		expect.any(AbortSignal),
	);
	expect(getCurseForgeProjectFiles).toHaveBeenLastCalledWith(
		42,
		"fabric",
		"1.21.1",
		expect.any(AbortSignal),
	);
	expect(getCurseForgeProjectDescription).toHaveBeenLastCalledWith(
		42,
		expect.any(AbortSignal),
	);
	expect(
		getMarketProjectId(
			localModToMarket(
				file("offline.jar", { source: "local", project_id: null }),
			),
		),
	).toBe("local-offline.jar");
});

test("uninstall removes only the selected shader file and keeps its sibling's metadata and remote installed marker", async () => {
	disk = [file("a.zip"), file("b.zip")];
	await start("shaderpacks");
	await source("local");
	state.selectProject("local-a.zip");
	expect(state.selectedProject?.installed?.filename).toBe("a.zip");
	const scans = scan.mock.calls.length;
	await state.uninstall(state.selectedProject);
	await settle();
	expect(deleteInstanceFile).toHaveBeenCalledTimes(1);
	expect(deleteInstanceFile).toHaveBeenCalledWith(
		"instance",
		"shaderpacks",
		"a.zip",
	);
	expect(scan.mock.calls.length).toBeGreaterThan(scans);
	expect(disk.map((entry) => entry.filename)).toEqual(["b.zip"]);
	expect(localIds()).toEqual(["local-b.zip"]);
	expect(snapshot(state.items[0])).toMatchObject({
		installed: disk[0],
		modrinthProjectId: "shared",
		slug: "shared-project",
		description: "Shared project",
	});
	expect(state.total).toBe(1);
	expect(state.selectedProject).toBeNull();
	await source("modrinth");
	expect(snapshot(state.items[0])).toMatchObject({
		id: "shared",
		modrinthProjectId: "shared",
		installed: disk[0],
	});
});

test("toggle renames only the selected mod and selection survives both directions and rescans", async () => {
	await start();
	await source("local");
	state.selectProject("local-a.jar");
	expect(state.selectedProject?.installed?.filename).toBe("a.jar");
	for (const enabled of [false, true]) {
		const previousFilename = enabled ? "a.jar.disabled" : "a.jar";
		const filename = enabled ? "a.jar" : "a.jar.disabled";
		const scans = scan.mock.calls.length;
		await state.toggleEnabled(state.selectedProject);
		await settle();
		expect(toggleInstanceMod).toHaveBeenLastCalledWith(
			"instance",
			previousFilename,
			enabled,
		);
		expect(scan.mock.calls.length).toBeGreaterThan(scans);
		expect(state.selectedId).toBe(`local-${filename}`);
		expect(state.selectedProject).toMatchObject({
			disabled: !enabled,
			installed: { filename, enabled },
			modrinthProjectId: "shared",
		});
		expect(
			state.items.find((item) => item.id === "local-b.jar")?.installed,
		).toEqual(file("b.jar"));
		refreshLocal();
		await settle();
		expect(state.selectedProject?.installed?.filename).toBe(filename);
		expect(localIds()).toEqual([`local-${filename}`, "local-b.jar"].sort());
	}
	expect(toggleInstanceMod).toHaveBeenCalledTimes(2);
});

test("toggle selection follows the renamed file when a newer scan supersedes its rescan", async () => {
	await start();
	await source("local");
	state.selectProject("local-a.jar");
	const toggleScan = deferred();
	const winningScan = deferred();
	const scans = scan.mock.calls.length;
	scan.mockImplementationOnce(
		() => toggleScan.promise,
	).mockImplementationOnce(() => winningScan.promise);

	const action = state.toggleEnabled(state.selectedProject);
	await settle();
	expect(toggleInstanceMod).toHaveBeenLastCalledWith(
		"instance",
		"a.jar",
		false,
	);
	expect(disk[0].filename).toBe("a.jar.disabled");
	expect(scan.mock.calls.length).toBe(scans + 1);
	refreshLocal();
	expect(scan.mock.calls.length).toBe(scans + 1);

	// The operation's scan finishes first, but the newer scan owns the update.
	toggleScan.resolve(structuredClone(disk));
	await settle();
	expect(scan.mock.calls.length).toBe(scans + 2);
	expect(state.selectedId).toBe("local-a.jar");
	expect(state.selectedProject?.installed?.filename).toBe("a.jar");

	winningScan.resolve(structuredClone(disk));
	await action;
	await settle();
	expect(state.selectedId).toBe("local-a.jar.disabled");
	expect(state.selectedProject?.installed?.filename).toBe("a.jar.disabled");
	expect(localIds()).toEqual(["local-a.jar.disabled", "local-b.jar"]);
	expect(
		snapshot(
			state.items.find((item) => item.id === "local-b.jar")?.installed,
		),
	).toEqual(file("b.jar"));
});

test("the same base filename with and without .disabled stays independently selectable", async () => {
	disk = [file("a.jar"), file("a.jar.disabled", { enabled: false })];
	await start();
	await source("local");
	expect(localIds()).toEqual(["local-a.jar", "local-a.jar.disabled"]);
	for (const entry of disk) {
		state.selectProject(`local-${entry.filename}`);
		refreshLocal();
		await settle();
		expect(state.selectedProject?.installed?.filename).toBe(entry.filename);
		expect(state.selectedProject?.disabled).toBe(!entry.enabled);
	}
	await state.uninstall(state.selectedProject);
	expect(deleteInstanceFile).toHaveBeenLastCalledWith(
		"instance",
		"mods",
		"a.jar.disabled",
	);
	expect(localIds()).toEqual(["local-a.jar"]);
	expect(snapshot(state.items[0].installed)).toEqual(file("a.jar"));
	expect(disk).toEqual([file("a.jar")]);
});

for (const operation of ["uninstall", "toggleEnabled"]) {
	test(`${operation}: a reported but swallowed native failure preserves the selected file`, async () => {
		disk = [file("a.jar"), file("a.jar.disabled", { enabled: false })];
		await start();
		await source("local");
		state.selectProject("local-a.jar");
		const before = snapshot(state.items);
		const native =
			operation === "uninstall" ? deleteInstanceFile : toggleInstanceMod;
		native.mockResolvedValueOnce(undefined);
		await state[operation](state.selectedProject);
		await settle();
		expect(snapshot(state.items)).toEqual(before);
		expect(state.selectedId).toBe("local-a.jar");
		expect(state.selectedProject.installed.filename).toBe("a.jar");
	});

	test(`${operation}: backend failure leaves local files, metadata and selection untouched`, async () => {
		await start();
		await source("local");
		state.selectProject("local-a.jar");
		await settle();
		const current = () =>
			snapshot({
				items: state.items,
				selectedId: state.selectedId,
				selectedProject: state.selectedProject,
				total: state.total,
				disk,
			});
		const before = current();
		const scans = scan.mock.calls.length;
		const pending = deferred();
		const native =
			operation === "uninstall" ? deleteInstanceFile : toggleInstanceMod;
		native.mockImplementationOnce(() => pending.promise);
		const log = spyOn(console, "error").mockImplementation(() => {});
		try {
			const action = state[operation](state.selectedProject);
			await settle();
			expect(native).toHaveBeenCalledTimes(1);
			expect(native.mock.lastCall).toEqual(
				operation === "uninstall"
					? ["instance", "mods", "a.jar"]
					: ["instance", "a.jar", false],
			);
			expect(current()).toEqual(before);
			pending.reject(new Error("Native operation failed"));
			await action;
			await settle();
			expect(current()).toEqual(before);
			expect(scan.mock.calls.length).toBe(scans);
		} finally {
			log.mockRestore();
		}
	});
}

test("a newer silent scan invalidates older enrichment results", async () => {
	await start();
	await source("local");
	state.selectProject("local-b.jar");
	const old = deferred();
	const latest = deferred();
	scan.mockImplementationOnce(() => old.promise).mockImplementationOnce(
		() => latest.promise,
	);
	refreshLocal();
	refreshLocal();
	latest.resolve([file("b.jar", { name: "Latest metadata" })]);
	await settle();
	old.resolve([file("a.jar"), file("b.jar", { name: "Stale metadata" })]);
	await settle();
	expect(localIds()).toEqual(["local-b.jar"]);
	expect(state.selectedProject?.title).toBe("Latest metadata");
	expect(state.error).toBeNull();
});

for (const provider of ["modrinth", "curseforge"]) {
	const id = provider === "modrinth" ? "shared" : "42";
	const search = provider === "modrinth" ? searchModrinth : searchCurseForge;

	test(`${provider}: typing supersedes a pending request and ignores its late response`, async () => {
		await start();
		if (provider !== "modrinth") await source(provider);
		const old = deferred();
		const latest = deferred();
		search
			.mockImplementationOnce(() => old.promise)
			.mockImplementationOnce(() => latest.promise);
		void state.refresh();
		state.setQuery("sod");
		state.setQuery("sodium");
		const calls = search.mock.calls.length;
		expect(state.loading).toBe(true);
		await Bun.sleep(280);
		expect(search.mock.calls.length).toBe(calls + 1);
		expect(search.mock.lastCall[0]).toBe("sodium");
		expect(search.mock.lastCall[4]).toBe("relevance");
		old.resolve(result(provider, "999"));
		await settle();
		expect(state.loading).toBe(true);
		expect(state.items).toHaveLength(0);
		latest.resolve(result(provider, id));
		await settle();
		expect(state.loading).toBe(false);
		expect(state.items.map((item) => item.id)).toEqual([id]);
	});

	test(`${provider}: filter changes invalidate errors even during the debounce window`, async () => {
		await start();
		if (provider !== "modrinth") await source(provider);
		const pending = deferred();
		search.mockImplementationOnce(() => pending.promise);
		void state.refresh();
		state.setCategory("optimization");
		pending.reject(new Error("Obsolete request"));
		await settle();
		expect(state.error).toBeNull();
		expect(state.loading).toBe(true);
		await Bun.sleep(280);
		await settle();
		expect(search.mock.lastCall[3]).toBe(
			provider === "modrinth" ? "optimization" : "6814",
		);
		expect(state.items.map((item) => item.id)).toEqual([id]);
	});

	test(`${provider}: automatic ranking respects explicit sorting and submit flushes the debounce`, async () => {
		await start();
		if (provider !== "modrinth") await source(provider);
		expect(search.mock.lastCall[4]).toBe("downloads");
		state.setQuery("  sodium  ");
		await state.refresh();
		expect(search.mock.lastCall[0]).toBe("sodium");
		expect(search.mock.lastCall[4]).toBe("relevance");
		state.setSort("newest");
		state.setQuery("iris");
		await state.refresh();
		expect(search.mock.lastCall[4]).toBe("newest");
		state.clearFilters();
		await state.refresh();
		expect(state.filters.query).toBe("iris");
		expect(search.mock.lastCall[4]).toBe("relevance");
		state.setQuery("");
		await state.refresh();
		expect(search.mock.lastCall[4]).toBe("downloads");
		const calls = search.mock.calls.length;
		await Bun.sleep(280);
		expect(search.mock.calls.length).toBe(calls);
	});

	test(`${provider}: pagination keeps at most 300 projects and refetches older pages without losing scroll extent`, async () => {
		await start();
		if (provider !== "modrinth") await source(provider);
		search.mockImplementation(async (...args) =>
			page(
				provider,
				Array.from({ length: 20 }, (_, i) => String(args[6] + i + 1)),
				340,
			),
		);
		await state.refresh();
		for (let i = 0; i < 16; i++) {
			state.loadMore();
			await settle();
		}
		expect(search.mock.lastCall[6]).toBe(320);
		expect(state.items).toHaveLength(300);
		expect(state.itemCount).toBe(340);
		expect(state.cachedPageCount).toBe(15);
		expect(state.getItem(0)).toBeUndefined();
		expect(state.getItem(339).id).toBe("340");
		state.ensureRange(0, 19);
		await settle();
		expect(search.mock.lastCall[6]).toBe(0);
		expect(state.getItem(0).id).toBe("1");
		expect(state.itemCount).toBe(340);
		expect(state.items).toHaveLength(300);
		expect(new Set(state.items.map((item) => item.id)).size).toBe(300);
		expect(state.hasMore).toBe(false);
	});

	test(`${provider}: overlapping pages deduplicate cards without repeating offsets; empty pages stop loading`, async () => {
		await start();
		if (provider !== "modrinth") await source(provider);
		search.mockResolvedValueOnce(page(provider, ["1", "2"], 10));
		await state.refresh();
		search.mockResolvedValueOnce(page(provider, ["2", "3"], 10));
		state.loadMore();
		await settle();
		expect(search.mock.lastCall[6]).toBe(2);
		expect(state.items.map((item) => item.id)).toEqual(["1", "2", "3"]);
		search.mockResolvedValueOnce(page(provider, [], 10));
		state.loadMore();
		await settle();
		expect(search.mock.lastCall[6]).toBe(4);
		expect(state.hasMore).toBe(false);
		const calls = search.mock.calls.length;
		state.loadMore();
		expect(search.mock.calls.length).toBe(calls);
	});

	test(`${provider}: a failed page can be retried at the same offset without losing results`, async () => {
		await start();
		if (provider !== "modrinth") await source(provider);
		search.mockResolvedValueOnce(page(provider, ["1"], 2));
		await state.refresh();
		search.mockResolvedValueOnce(null);
		state.loadMore();
		await settle();
		expect(state.error).not.toBeNull();
		expect(state.items.map((item) => item.id)).toEqual(["1"]);
		const calls = search.mock.calls.length;
		state.loadMore();
		expect(search.mock.calls.length).toBe(calls);
		search.mockResolvedValueOnce(page(provider, ["2"], 2));
		await state.retry();
		expect(search.mock.lastCall[6]).toBe(1);
		expect(state.items.map((item) => item.id)).toEqual(["1", "2"]);
		expect(state.error).toBeNull();
		expect(state.hasMore).toBe(false);
	});
	for (const mode of ["reset", "loadMore"]) {
		test(`${provider}: pending ${mode} cannot overwrite Local or its pagination`, async () => {
			await start();
			if (provider !== "modrinth") await source(provider);
			search.mockResolvedValueOnce(result(provider, id, 40));
			await state.refresh();
			const pending = deferred();
			search.mockImplementationOnce(() => pending.promise);
			if (mode === "reset") void state.refresh();
			else state.loadMore();
			expect(
				mode === "reset" ? state.loadingRemote : state.loadingMore,
			).toBe(true);
			state.setSource("local");
			expect(state.loadingRemote).toBe(false);
			expect(state.loadingMore).toBe(false);
			expect(state.items).toHaveLength(0);
			expect(state.total).toBe(0);
			await Bun.sleep(230);
			await settle();
			const calls = search.mock.calls.length;
			state.loadMore();
			pending.resolve(result(provider, id, 99));
			await settle();
			expect(search.mock.calls.length).toBe(calls);
			expect(localIds()).toEqual(["local-a.jar", "local-b.jar"]);
			expect(state.total).toBe(2);
			expect(state.hasMore).toBe(false);
			expect(state.error).toBeNull();
		});
	}
	test(`${provider}: stale errors are ignored in Local`, async () => {
		await start();
		if (provider !== "modrinth") await source(provider);
		const old = deferred();
		search.mockImplementationOnce(() => old.promise);
		void state.refresh();
		await source("local");
		old.reject(new Error("obsolete search failure"));
		await settle();
		expect(state.error).toBeNull();
		expect(localIds()).toEqual(["local-a.jar", "local-b.jar"]);
	});
	test(`${provider}: source debounce blocks loadMore and stale finally cannot finish a newer search`, async () => {
		await start();
		if (provider !== "modrinth") await source(provider);
		const old = deferred();
		const latest = deferred();
		search
			.mockImplementationOnce(() => old.promise)
			.mockImplementationOnce(() => latest.promise);
		void state.refresh();
		await source("local");
		state.setSource(provider);
		const calls = search.mock.calls.length;
		state.loadMore();
		await settle();
		expect(search.mock.calls.length).toBe(calls);
		await Bun.sleep(230);
		await settle();
		expect(search.mock.calls.length).toBe(calls + 1);
		expect(search.mock.lastCall?.[6]).toBe(0);
		expect(state.loadingRemote).toBe(true);
		old.resolve(
			result(provider, provider === "modrinth" ? "obsolete" : "99"),
		);
		await settle();
		expect(state.loadingRemote).toBe(true);
		expect(state.items).toHaveLength(0);
		expect(state.total).toBe(0);
		latest.resolve(result(provider, id));
		await settle();
		expect(state.loadingRemote).toBe(false);
		expect(state.loadingMore).toBe(false);
		expect(state.items.map((item) => item.id)).toEqual([id]);
		expect(state.total).toBe(1);
		expect(state.error).toBeNull();
	});
}

test("local queries filter immediately and source changes retain the query", async () => {
	disk = [file("a.jar", { name: "Sodium" }), file("b.jar", { name: "Iris" })];
	await start();
	await source("local");
	const scans = scan.mock.calls.length;
	state.setQuery("  sodium  ");
	expect(state.items.map((item) => item.title)).toEqual(["Sodium"]);
	expect(scan.mock.calls.length).toBe(scans);
	await source("curseforge");
	expect(searchCurseForge.mock.lastCall[0]).toBe("sodium");
});

test("closing or switching details prevents a late response from replacing the current project", async () => {
	await start();
	await source("local");
	const old = deferred();
	getModrinthProject.mockImplementationOnce(() => old.promise);
	state.selectProject("local-a.jar");
	state.selectProject(null);
	getModrinthProject.mockResolvedValueOnce({
		id: "latest",
		body: "Latest description",
	});
	state.selectProject("local-b.jar");
	await settle();
	old.resolve({ id: "obsolete", body: "Old description" });
	await settle();
	expect(state.selectedId).toBe("local-b.jar");
	expect(state.detail.fullProject.id).toBe("latest");
	expect(state.detail.loading).toBe(false);
});

test("destroy cancels scheduled searches and ignores in-flight results", async () => {
	await start();
	const pending = deferred();
	searchModrinth.mockImplementationOnce(() => pending.promise);
	void state.refresh();
	state.setQuery("sodium");
	const calls = searchModrinth.mock.calls.length;
	state.destroy();
	pending.resolve(result("modrinth"));
	await Bun.sleep(280);
	await settle();
	expect(searchModrinth.mock.calls.length).toBe(calls);
	expect(state.items).toHaveLength(0);
});

for (const operation of ["confirmInstall", "uninstall", "toggleEnabled"]) {
	test(`${operation}: completing after destroy cannot rescan or repopulate the market`, async () => {
		await start();
		await source("local");
		const project =
			operation === "confirmInstall"
				? { ...state.items[0], installed: undefined }
				: state.items[0];
		const pending = deferred();
		const fn =
			operation === "confirmInstall"
				? api.downloadMods
				: operation === "uninstall"
					? deleteInstanceFile
					: toggleInstanceMod;
		fn.mockImplementationOnce(() => pending.promise);
		const action =
			operation === "confirmInstall"
				? state.confirmInstall(project, [
						{
							url: "https://example.test/a.jar",
							filename: "a.jar",
							project_id: "shared",
							version_id: "v1",
						},
					])
				: state[operation](project);
		const scans = scan.mock.calls.length;
		state.destroy();
		state.destroy();
		pending.resolve();
		await action;
		refreshLocal();
		await state.refresh();
		state.setQuery("sodium");
		state.setSource("curseforge");
		state.loadMore();
		await settle();
		expect(unregisterRefresh).toHaveBeenCalledTimes(1);
		expect(scan.mock.calls.length).toBe(scans);
		expect(state.items).toHaveLength(0);
		expect(state.cachedPageCount).toBe(0);
		expect(state.itemCount).toBe(0);
		expect(state.loading).toBe(false);
	});
}

function modVersion(id, hash, overrides = {}) {
	return {
		id,
		name: id,
		version_number: id,
		game_versions: ["1.21.1"],
		loaders: ["fabric"],
		date_published:
			id === "v1" ? "2025-01-01T00:00:00Z" : "2025-02-01T00:00:00Z",
		version_type: "release",
		dependencies: [],
		files: [
			{
				primary: true,
				filename: `${id}.jar`,
				url: `https://example.test/${id}.jar`,
				hashes: { sha1: hash },
			},
		],
		...overrides,
	};
}

async function selectOwnMod() {
	disk = [
		file("old.jar.disabled", {
			sha1: "a".repeat(40),
			enabled: false,
			version_id: "v1",
		}),
		file("other.jar", { project_id: "other" }),
	];
	getModrinthProjectVersions.mockResolvedValueOnce([
		modVersion("v2", "b".repeat(40)),
		modVersion("v1", "a".repeat(40)),
	]);
	await start();
	await source("local");
	state.selectProject("local-old.jar.disabled");
	await settle();
	return state.selectedProject;
}

test("installed version is identified by hash rather than a stale cached version ID", async () => {
	disk = [file("old.jar", { sha1: "a".repeat(40), version_id: "v2" })];
	getModrinthProjectVersions.mockResolvedValueOnce([
		modVersion("v2", "b".repeat(40)),
		modVersion("v1", "a".repeat(40)),
	]);
	await start();
	await source("local");
	state.selectProject("local-old.jar");
	await settle();
	expect(state.selectedVersion.id).toBe("v1");
	expect(
		state.detail.versions.filter((v) => v.isInstalled).map((v) => v.id),
	).toEqual(["v1"]);
});

test("changing a disabled own mod delegates atomic replacement and follows its new filename", async () => {
	const project = await selectOwnMod();
	replaceInstanceMod.mockImplementationOnce(async (_id, request) => {
		expect(request.filename).toBe("old.jar.disabled");
		expect(request.expected_sha1).toBe("a".repeat(40));
		expect(request.target).toEqual({
			source: "modrinth",
			project_id: "shared",
			version_id: "v2",
		});
		disk[0] = file("v2.jar.disabled", {
			enabled: false,
			sha1: "b".repeat(40),
			version_id: "v2",
		});
		return "v2.jar.disabled";
	});
	await state.confirmInstall(project, [
		{
			source: "modrinth",
			project_id: "shared",
			version_id: "v2",
			filename: "v2.jar",
			url: "https://example.test/v2.jar",
		},
	]);
	expect(api.downloadMods).not.toHaveBeenCalled();
	expect(deleteInstanceFile).not.toHaveBeenCalled();
	expect(state.selectedProject.installed.filename).toBe("v2.jar.disabled");
	expect(state.selectedProject.installed.enabled).toBe(false);
	expect(localIds()).toEqual(["local-other.jar", "local-v2.jar.disabled"]);
	expect(state.localOperationBusy).toBe(false);
});

test("a failed version replacement preserves the installed list and surfaces the error", async () => {
	const project = await selectOwnMod();
	replaceInstanceMod.mockRejectedValueOnce(
		Error("Download checksum mismatch"),
	);
	await expect(
		state.confirmInstall(project, [
			{
				project_id: "shared",
				version_id: "v2",
				filename: "v2.jar",
				url: "https://example.test/v2.jar",
			},
		]),
	).rejects.toThrow("checksum mismatch");
	expect(localIds()).toEqual(["local-old.jar.disabled", "local-other.jar"]);
	expect(deleteInstanceFile).not.toHaveBeenCalled();
	expect(state.localOperationBusy).toBe(false);
});

test("version changes reject incompatible targets and pack-owned mods even after unlocking", async () => {
	const project = await selectOwnMod();
	await expect(
		state.confirmInstall(project, [
			{
				project_id: "shared",
				version_id: "unknown",
				filename: "wrong.jar",
				url: "https://example.test/wrong.jar",
			},
		]),
	).rejects.toThrow("market.modVersions.incompatible");
	project.installed.pack_name = "Pack";
	project.installed.pack_locked = false;
	await expect(
		state.confirmInstall(project, [
			{
				project_id: "shared",
				version_id: "v2",
				filename: "v2.jar",
				url: "https://example.test/v2.jar",
			},
		]),
	).rejects.toThrow("modpack.protected");
	expect(replaceInstanceMod).not.toHaveBeenCalled();
});

test("replacement preview includes the root and dependencies whose installed version differs", async () => {
	const project = await selectOwnMod();
	disk.push(
		file("library.jar", { project_id: "library", version_id: "lib-old" }),
	);
	api.resolveModDependencies.mockResolvedValueOnce({
		conflicts: [],
		tree: [
			{
				source: "modrinth",
				project_id: "shared",
				version_id: "v2",
				kind: "required",
				children: [
					{
						source: "modrinth",
						project_id: "library",
						version_id: "lib-new",
						kind: "required",
						children: [],
					},
				],
			},
		],
	});
	const result = await state.prepareInstall(
		project,
		state.detail.versions.find((v) => v.id === "v2"),
	);
	expect(result.installedProjectIds.has("shared")).toBe(false);
	expect(result.installedProjectIds.has("library")).toBe(false);
});

test("finishing a replacement after destroy does not refresh the previous instance", async () => {
	const project = await selectOwnMod();
	const pending = deferred();
	replaceInstanceMod.mockImplementationOnce(() => pending.promise);
	const operation = state.confirmInstall(project, [
		{
			project_id: "shared",
			version_id: "v2",
			filename: "v2.jar",
			url: "https://example.test/v2.jar",
		},
	]);
	await settle();
	expect(replaceInstanceMod).toHaveBeenCalledTimes(1);
	const scans = scan.mock.calls.length;
	state.destroy();
	pending.resolve("v2.jar.disabled");
	await operation;
	expect(scan.mock.calls.length).toBe(scans);
	expect(state.items).toHaveLength(0);
});

function dependency(id, overrides = {}) {
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

async function quickFixture(content = "mods") {
	disk = [];
	searchModrinth.mockResolvedValue(result("modrinth", "quick"));
	getModrinthProjectVersions.mockResolvedValue([
		modVersion("v2", "b".repeat(40)),
	]);
	api.resolveModDependencies.mockResolvedValue({
		tree: [dependency("quick")],
		conflicts: [],
	});
	await start(content);
	return state.items[0];
}

test("quick install downloads the newest compatible version and required dependencies without selecting details", async () => {
	const project = await quickFixture();
	disk = [
		file("provided.jar", {
			project_id: "provided",
			pack_name: "Pack",
			pack_locked: true,
		}),
	];
	getModrinthProjectVersions.mockResolvedValue([
		modVersion("v3", "c".repeat(40), {
			date_published: "2026-03-01T00:00:00Z",
			loaders: ["forge"],
		}),
		modVersion("v1", "a".repeat(40)),
		modVersion("v2", "b".repeat(40)),
	]);
	api.resolveModDependencies.mockResolvedValue({
		conflicts: [],
		tree: [
			dependency("quick", {
				children: [
					dependency("library"),
					dependency("provided", {
						children: [dependency("nested")],
					}),
					dependency("optional", {
						kind: "optional",
						children: [dependency("optional-child")],
					}),
					dependency("embedded", { kind: "embedded" }),
					dependency("absent", { kind: "incompatible" }),
				],
			}),
		],
	});
	api.downloadMods.mockImplementation(async (_id, queue) => {
		disk.push(
			...queue.map((entry) =>
				file(entry.filename, {
					source: entry.source,
					project_id: entry.project_id,
					version_id: entry.version_id,
				}),
			),
		);
	});
	const filters = snapshot(state.filters);
	const revision = state.resultsRevision;
	const searches = searchModrinth.mock.calls.length;
	await state.installQuick(project);
	expect(api.resolveModDependencies.mock.calls[0][0]).toEqual([
		{
			source: "modrinth",
			project_id: "quick",
			version_id: "v2",
			kind: "required",
		},
	]);
	expect(
		api.downloadMods.mock.calls[0][1].map((entry) => entry.project_id),
	).toEqual(["quick", "library", "nested"]);
	expect(state.selectedId).toBeNull();
	expect(getModrinthProject).not.toHaveBeenCalled();
	expect(snapshot(state.filters)).toEqual(filters);
	expect(state.resultsRevision).toBe(revision);
	expect(searchModrinth.mock.calls.length).toBe(searches);
	expect(state.items[0].installed.project_id).toBe("quick");
	expect(state.quickInstallBusy).toBe(false);
});

test("install preview resolves the newest compatible version without downloading or navigating", async () => {
	const project = await quickFixture();
	getModrinthProjectVersions.mockResolvedValue([
		modVersion("v3", "c".repeat(40), {
			date_published: "2026-03-01T00:00:00Z",
			loaders: ["forge"],
		}),
		modVersion("v1", "a".repeat(40), {
			date_published: "2025-01-01T00:00:00Z",
		}),
		modVersion("v2", "b".repeat(40), {
			date_published: "2025-02-01T00:00:00Z",
		}),
	]);
	api.resolveModDependencies.mockResolvedValue({
		conflicts: [],
		tree: [dependency("quick", { children: [dependency("library")] })],
	});
	const filters = snapshot(state.filters);
	const searches = searchModrinth.mock.calls.length;
	const preview = await state.resolveInstallPreview(project);
	expect(preview.version.id).toBe("v2");
	expect(preview.tree.map((node) => node.project_id)).toEqual(["quick"]);
	expect(preview.conflicts).toEqual([]);
	expect(state.selectedId).toBeNull();
	expect(api.downloadMods).not.toHaveBeenCalled();
	expect(getModrinthProject).not.toHaveBeenCalled();
	expect(snapshot(state.filters)).toEqual(filters);
	expect(searchModrinth.mock.calls.length).toBe(searches);
});

test("install preview reports when no version is compatible", async () => {
	const project = await quickFixture();
	getModrinthProjectVersions.mockResolvedValue([
		modVersion("v1", "a".repeat(40), { loaders: ["forge"] }),
	]);
	await expect(state.resolveInstallPreview(project)).rejects.toThrow(
		"market.quickInstall.noCompatibleVersion",
	);
	expect(state.selectedId).toBeNull();
});

test("quick install reports missing compatible versions on the card and remains retryable", async () => {
	const project = await quickFixture();
	getModrinthProjectVersions.mockResolvedValueOnce([
		modVersion("v2", "b", { game_versions: ["1.20.1"] }),
	]);
	await expect(state.installQuick(project)).rejects.toThrow(
		"market.quickInstall.noCompatibleVersion",
	);
	expect(state.quickInstallError(project)).toBe(
		"market.quickInstall.noCompatibleVersion",
	);
	expect(state.error).toBeNull();
	expect(state.selectedId).toBeNull();
	expect(api.downloadMods).not.toHaveBeenCalled();
	await state.installQuick(project);
	expect(api.downloadMods).toHaveBeenCalledTimes(1);
	expect(state.quickInstallError(project)).toBeNull();
});

test("quick install coalesces double clicks and blocks competing mutations while preparing", async () => {
	const project = await quickFixture();
	const versions = deferred();
	getModrinthProjectVersions.mockImplementationOnce(() => versions.promise);
	const first = state.installQuick(project);
	const second = state.installQuick(project);
	expect(first).toBe(second);
	await settle();
	expect(state.quickInstallBusy).toBe(true);
	expect(state.quickInstallStatus(project)).toBe("preparing");
	await expect(
		state.installQuick({
			...project,
			id: "another",
			modrinthProjectId: "another",
		}),
	).rejects.toThrow("market.modVersions.busy");
	await expect(state.prepareInstall(project, { id: "v2" })).rejects.toThrow(
		"market.modVersions.busy",
	);
	versions.resolve([modVersion("v2", "b")]);
	await first;
	expect(getModrinthProjectVersions).toHaveBeenCalledTimes(1);
	expect(api.downloadMods).toHaveBeenCalledTimes(1);
});

test("quick install errors restore the button without marking the mod as installed", async () => {
	const project = await quickFixture();
	api.downloadMods.mockRejectedValueOnce(Error("Download failed"));
	await expect(state.installQuick(project)).rejects.toThrow(
		"Download failed",
	);
	expect(state.quickInstallError(project)).toBe("Download failed");
	expect(state.items[0].installed).toBeUndefined();
	expect(state.quickInstallBusy).toBe(false);
	await state.installQuick(project);
	expect(api.downloadMods).toHaveBeenCalledTimes(2);
});

test("quick install survives catalog navigation without changing the selected provider", async () => {
	const project = await quickFixture();
	const resolution = deferred();
	api.resolveModDependencies.mockImplementationOnce(() => resolution.promise);
	const installing = state.installQuick(project);
	await settle();
	state.setSource("curseforge");
	await settle();
	resolution.resolve({ tree: [dependency("quick")], conflicts: [] });
	await installing;
	expect(state.filters.source).toBe("curseforge");
	expect(state.selectedId).toBeNull();
	expect(api.downloadMods.mock.calls[0][1][0].source).toBe("modrinth");
});

test("destroying the market cancels quick-install version lookup before downloading", async () => {
	const project = await quickFixture();
	const versions = deferred();
	getModrinthProjectVersions.mockImplementationOnce(() => versions.promise);
	const installing = state.installQuick(project);
	await settle();
	const signal = getModrinthProjectVersions.mock.calls[0][3];
	state.destroy();
	expect(signal.aborted).toBe(true);
	versions.resolve([modVersion("v2", "b")]);
	await installing;
	expect(api.resolveModDependencies).not.toHaveBeenCalled();
	expect(api.downloadMods).not.toHaveBeenCalled();
	expect(state.items).toHaveLength(0);
});

test("destroying during a confirmed quick download does not reactivate the old catalog", async () => {
	const project = await quickFixture();
	const downloaded = deferred();
	api.downloadMods.mockImplementationOnce(() => downloaded.promise);
	const installing = state.installQuick(project);
	await settle();
	expect(state.quickInstallStatus(project)).toBe("installing");
	const scans = scan.mock.calls.length;
	state.destroy();
	downloaded.resolve();
	await installing;
	expect(scan.mock.calls.length).toBe(scans);
	expect(state.items).toHaveLength(0);
});

test("quick install refuses conflicts, disabled dependencies and filename collisions", async () => {
	const project = await quickFixture();
	disk = [
		file("library.jar.disabled", { project_id: "library", enabled: false }),
	];
	api.resolveModDependencies.mockResolvedValue({
		tree: [dependency("quick", { children: [dependency("library")] })],
		conflicts: [],
	});
	await expect(state.installQuick(project)).rejects.toThrow(
		"market.quickInstall.disabledDependency",
	);
	disk = [file("quick.jar", { source: "local", project_id: null })];
	api.resolveModDependencies.mockResolvedValue({
		tree: [dependency("quick")],
		conflicts: [],
	});
	await expect(state.installQuick(project)).rejects.toThrow(
		"market.quickInstall.fileExists",
	);
	disk = [];
	api.resolveModDependencies.mockResolvedValue({
		tree: [dependency("quick")],
		conflicts: [
			{ source: "modrinth", project_id: "quick", requested_versions: [] },
		],
	});
	await expect(state.installQuick(project)).rejects.toThrow(
		"market.quickInstall.dependencyConflict",
	);
	expect(api.downloadMods).not.toHaveBeenCalled();
});

test("quick install checks running status again after asynchronous resolution", async () => {
	const project = await quickFixture();
	const resolution = deferred();
	api.resolveModDependencies.mockImplementationOnce(() => resolution.promise);
	const installing = state.installQuick(project);
	await settle();
	instance.status = InstState.Started;
	resolution.resolve({ tree: [dependency("quick")], conflicts: [] });
	await expect(installing).rejects.toThrow("errors.INST_BUSY");
	expect(api.downloadMods).not.toHaveBeenCalled();
});

test("quick CurseForge installs use compatible available files and provider-qualified dependencies", async () => {
	await quickFixture();
	await source("curseforge");
	const project = state.items[0];
	disk = [
		file("modrinth-library.jar", { source: "modrinth", project_id: "99" }),
	];
	getCurseForgeProjectFiles.mockResolvedValue([
		{
			id: 2,
			fileName: "root.jar",
			fileDate: "2026-01-01",
			gameVersions: ["1.21.1", "Fabric"],
			isAvailable: true,
			releaseType: 1,
			dependencies: [],
		},
		{
			id: 3,
			fileName: "unavailable.jar",
			fileDate: "2026-02-01",
			gameVersions: ["1.21.1", "Fabric"],
			isAvailable: false,
			releaseType: 1,
			dependencies: [],
		},
	]);
	api.resolveModDependencies.mockResolvedValue({
		tree: [
			dependency("42", {
				source: "curseforge",
				version_id: "2",
				children: [dependency("99", { source: "curseforge" })],
			}),
		],
		conflicts: [],
	});
	await state.installQuick(project);
	expect(api.resolveModDependencies.mock.calls[0][0][0]).toEqual({
		source: "curseforge",
		project_id: "42",
		version_id: "2",
		kind: "required",
	});
	expect(
		api.downloadMods.mock.calls[0][1].map((entry) => entry.project_id),
	).toEqual(["42", "99"]);
	expect(getCurseForgeProject).not.toHaveBeenCalled();
	expect(getCurseForgeProjectDescription).not.toHaveBeenCalled();
});

test("resource packs install directly without resolving mod dependencies", async () => {
	const project = await quickFixture("resourcepacks");
	getModrinthProjectVersions.mockResolvedValue([
		modVersion("v2", "b", {
			loaders: [],
			files: [
				{
					primary: true,
					filename: "pack.zip",
					url: "https://example.test/pack.zip",
				},
			],
		}),
	]);
	await state.installQuick(project);
	expect(api.downloadResourcePacks.mock.calls[0][1][0].filename).toBe(
		"pack.zip",
	);
	expect(api.resolveModDependencies).not.toHaveBeenCalled();
	expect(state.selectedId).toBeNull();
});

async function installedUpdateFixture(mods) {
	disk = mods;
	getModrinthProjectVersions.mockResolvedValue([
		modVersion("v2", "b".repeat(40)),
		modVersion("v1", "a".repeat(40)),
	]);
	api.resolveModDependencies.mockImplementation(async ([request]) => ({
		tree: [
			dependency(request.project_id, {
				source: request.source,
				version_id: request.version_id,
			}),
		],
		conflicts: [],
	}));
	replaceInstanceMod.mockImplementation(async (_id, request) => {
		const index = disk.findIndex(
			(mod) => mod.filename === request.filename,
		);
		const old = disk[index];
		const filename = `${request.target.project_id}-v2.jar${old.enabled ? "" : ".disabled"}`;
		disk[index] = {
			...old,
			filename,
			sha1: "b".repeat(40),
			version_id: request.target.version_id,
			version: "2.0",
		};
		return filename;
	});
	await start();
	await source("local");
}

function ownMod(id, overrides = {}) {
	return file(`${id}-v1.jar`, {
		project_id: id,
		sha1: "a".repeat(40),
		version_id: "v1",
		...overrides,
	});
}

test("update all includes filtered-out own mods and excludes pack and unidentified files", async () => {
	await installedUpdateFixture([
		ownMod("first"),
		ownMod("second", {
			enabled: false,
			filename: "second-v1.jar.disabled",
		}),
		ownMod("current", { sha1: "b".repeat(40), version_id: "v2" }),
		ownMod("pack", { pack_name: "Pack", pack_locked: false }),
		file("unknown.jar", { source: "local", project_id: null }),
	]);
	state.setQuery("no results");
	expect(state.items).toHaveLength(0);
	expect(state.updatableModCount).toBe(3);
	const filters = snapshot(state.filters);
	await state.updateOwnMods();
	expect(
		replaceInstanceMod.mock.calls.map(
			([, request]) => request.target.project_id,
		),
	).toEqual(["first", "second"]);
	expect(disk.find((mod) => mod.project_id === "second").filename).toBe(
		"second-v2.jar.disabled",
	);
	expect(disk.find((mod) => mod.project_id === "pack").version_id).toBe("v1");
	expect(snapshot(state.modUpdateReport)).toMatchObject({
		total: 3,
		completed: 3,
		updated: 2,
		current: 1,
		skipped: 2,
		failures: [],
	});
	expect(state.modUpdateBusy).toBe(false);
	expect(state.localOperationBusy).toBe(false);
	expect(snapshot(state.filters)).toEqual(filters);
	expect(state.selectedId).toBeNull();
});

test("an individual local update only changes the requested mod and preserves disabled state", async () => {
	await installedUpdateFixture([
		ownMod("first"),
		ownMod("second", {
			enabled: false,
			filename: "second-v1.jar.disabled",
		}),
	]);
	await state.updateOwnMods(["second-v1.jar.disabled"]);
	expect(replaceInstanceMod).toHaveBeenCalledTimes(1);
	expect(replaceInstanceMod.mock.calls[0][1].filename).toBe(
		"second-v1.jar.disabled",
	);
	expect(replaceInstanceMod.mock.calls[0][1].expected_sha1).toBe(
		"a".repeat(40),
	);
	expect(disk[0].version_id).toBe("v1");
	expect(disk[1].enabled).toBe(false);
	expect(snapshot(state.modUpdateReport)).toMatchObject({
		total: 1,
		completed: 1,
		updated: 1,
		failures: [],
	});
	expect(state.selectedId).toBeNull();
});

test("bulk updates continue after a failed mod and preserve its original file", async () => {
	await installedUpdateFixture([ownMod("first"), ownMod("second")]);
	replaceInstanceMod.mockRejectedValueOnce(Error("Checksum mismatch"));
	await state.updateOwnMods();
	expect(disk[0].filename).toBe("first-v1.jar");
	expect(disk[1].filename).toBe("second-v2.jar");
	expect(snapshot(state.modUpdateReport)).toMatchObject({
		total: 2,
		completed: 2,
		updated: 1,
		failures: [{ filename: "first-v1.jar", error: "Checksum mismatch" }],
	});
});

test("a mod updated as a dependency is rediscovered by project and is not updated twice", async () => {
	await installedUpdateFixture([ownMod("first"), ownMod("library")]);
	api.resolveModDependencies.mockResolvedValueOnce({
		tree: [dependency("first", { children: [dependency("library")] })],
		conflicts: [],
	});
	replaceInstanceMod.mockImplementationOnce(async (_id, request) => {
		expect(request.downloads.map((ref) => ref.project_id)).toEqual([
			"first",
			"library",
		]);
		disk = disk.map((mod) => ({
			...mod,
			filename: `${mod.project_id}-v2.jar`,
			sha1: "b".repeat(40),
			version_id: "v2",
		}));
		return "first-v2.jar";
	});
	await state.updateOwnMods();
	expect(replaceInstanceMod).toHaveBeenCalledTimes(1);
	expect(snapshot(state.modUpdateReport)).toMatchObject({
		updated: 1,
		current: 1,
		completed: 2,
		failures: [],
	});
});

test("cancel remaining lets an admitted replacement finish without starting another mod", async () => {
	await installedUpdateFixture([ownMod("first"), ownMod("second")]);
	const pending = deferred();
	replaceInstanceMod.mockImplementationOnce(() => pending.promise);
	const updating = state.updateOwnMods();
	await settle();
	expect(state.modUpdateReport.phase).toBe("updating");
	state.cancelModUpdates();
	pending.resolve("first-v2.jar");
	await updating;
	expect(replaceInstanceMod).toHaveBeenCalledTimes(1);
	expect(snapshot(state.modUpdateReport)).toMatchObject({
		updated: 1,
		completed: 1,
		skipped: 1,
		cancelled: true,
		failures: [],
	});
	expect(state.localOperationBusy).toBe(false);
});

test("a duplicate bulk click does not start concurrent replacements", async () => {
	await installedUpdateFixture([ownMod("first"), ownMod("second")]);
	const pending = deferred();
	getModrinthProjectVersions.mockImplementationOnce(() => pending.promise);
	const updating = state.updateOwnMods();
	await settle();
	await state.updateOwnMods();
	expect(getModrinthProjectVersions).toHaveBeenCalledTimes(1);
	pending.resolve([
		modVersion("v2", "b".repeat(40)),
		modVersion("v1", "a".repeat(40)),
	]);
	await updating;
	expect(replaceInstanceMod).toHaveBeenCalledTimes(2);
});

test("bulk updates retain a prerelease newer than the stable channel and skip incompatible releases", async () => {
	await installedUpdateFixture([ownMod("beta"), ownMod("incompatible")]);
	getModrinthProjectVersions.mockImplementation(async (id) =>
		id === "beta"
			? [
					modVersion("old-stable", "x", {
						date_published: "2024-01-01",
						version_type: "release",
					}),
					modVersion("current-beta", "a".repeat(40), {
						date_published: "2026-01-01",
						version_type: "beta",
					}),
					modVersion("new-alpha", "z", {
						date_published: "2026-02-01",
						version_type: "alpha",
					}),
				]
			: [modVersion("v2", "b", { game_versions: ["1.20.1"] })],
	);
	await state.updateOwnMods();
	expect(replaceInstanceMod).not.toHaveBeenCalled();
	expect(snapshot(state.modUpdateReport)).toMatchObject({
		current: 1,
		skipped: 1,
		completed: 2,
		failures: [],
	});
});

test("CurseForge mods participate in bulk updates with their exact provider and file IDs", async () => {
	await installedUpdateFixture([
		ownMod("42", { source: "curseforge", version_id: "1" }),
	]);
	getCurseForgeProjectFiles.mockResolvedValue([
		{
			id: 1,
			fileName: "old.jar",
			fileDate: "2025-01-01",
			gameVersions: ["1.21.1", "Fabric"],
			isAvailable: true,
			releaseType: 1,
			dependencies: [],
			hashes: [{ algo: 1, value: "a".repeat(40) }],
		},
		{
			id: 2,
			fileName: "new.jar",
			fileDate: "2026-01-01",
			gameVersions: ["1.21.1", "Fabric"],
			isAvailable: true,
			releaseType: 1,
			dependencies: [],
			hashes: [{ algo: 1, value: "b".repeat(40) }],
		},
	]);
	await state.updateOwnMods();
	expect(replaceInstanceMod.mock.calls[0][1].target).toEqual({
		source: "curseforge",
		project_id: "42",
		version_id: "2",
	});
	expect(state.modUpdateReport.updated).toBe(1);
});

test("destroy cancels metadata checks and prevents pending bulk work from reaching another instance", async () => {
	await installedUpdateFixture([ownMod("first"), ownMod("second")]);
	const pending = deferred();
	getModrinthProjectVersions.mockImplementationOnce(() => pending.promise);
	const updating = state.updateOwnMods();
	await settle();
	const signal = getModrinthProjectVersions.mock.calls[0][3];
	state.destroy();
	expect(signal.aborted).toBe(true);
	pending.resolve([modVersion("v2", "b")]);
	await updating;
	expect(replaceInstanceMod).not.toHaveBeenCalled();
	expect(state.modUpdateReport).toBeNull();
});

test("bulk updates stop admitting replacements if the instance starts during a lookup", async () => {
	await installedUpdateFixture([ownMod("first")]);
	const pending = deferred();
	getModrinthProjectVersions.mockImplementationOnce(() => pending.promise);
	const updating = state.updateOwnMods();
	await settle();
	instance.status = InstState.Started;
	pending.resolve([modVersion("v2", "b".repeat(40))]);
	await updating;
	expect(replaceInstanceMod).not.toHaveBeenCalled();
	expect(state.modUpdateReport.failures[0].error).toBe("errors.INST_BUSY");
	expect(state.localOperationBusy).toBe(false);
});

test("a burst of enrichment events shares one scan and one final refresh", async () => {
	await start();
	await source("local");
	const pending = deferred();
	scan.mockImplementationOnce(() => pending.promise);
	const scans = scan.mock.calls.length;
	for (let i = 0; i < 100; i++) refreshLocal();
	expect(scan.mock.calls.length).toBe(scans + 1);
	pending.resolve([file("stale.jar")]);
	await settle();
	expect(scan.mock.calls.length).toBe(scans + 2);
	expect(localIds()).toEqual(["local-a.jar", "local-b.jar"]);
});

test("closing during a scan discards its data and its queued refresh", async () => {
	await start();
	const pending = deferred();
	scan.mockImplementationOnce(() => pending.promise);
	refreshLocal();
	refreshLocal();
	const calls = scan.mock.calls.length;
	state.destroy();
	pending.resolve(disk);
	await settle();
	expect(scan.mock.calls.length).toBe(calls);
	expect(state.items).toHaveLength(0);
	expect(state.loading).toBe(false);
});

test("query changes and closing abort the signals sent to native searches and details", async () => {
	await start();
	const pending = deferred();
	searchModrinth.mockImplementationOnce(() => pending.promise);
	void state.refresh();
	const signal = searchModrinth.mock.lastCall[8];
	expect(signal.aborted).toBe(false);
	state.setQuery("new query");
	expect(signal.aborted).toBe(true);
	pending.resolve(result("modrinth"));
	await source("local");
	state.setQuery("");
	const detail = deferred();
	getModrinthProject.mockImplementationOnce(() => detail.promise);
	state.selectProject("local-a.jar");
	const detailSignal = getModrinthProject.mock.lastCall[1];
	expect(detailSignal.aborted).toBe(false);
	state.destroy();
	expect(detailSignal.aborted).toBe(true);
	detail.resolve({ id: "stale", body: "Late response" });
	await settle();
	expect(state.detail.fullProject).toBeUndefined();
});

test("long browsing sessions retain bounded pages, and repeated sessions release them", async () => {
	const closedHeapBytes = [];
	const sessions = Math.max(
		1,
		Math.min(50, Number(process.env.MARKET_MEMORY_SESSIONS) || 5),
	);
	let firstTypes;
	for (let session = 0; session < sessions; session++) {
		await new Promise((done, fail) =>
			setTimeout(() => {
				const runSession = async () => {
					searchModrinth.mockImplementation(async (...args) =>
						page(
							"modrinth",
							Array.from({ length: 20 }, (_, i) =>
								String(args[6] + i),
							),
							10000,
						),
					);
					await start();
					for (let i = 0; i < 99; i++) {
						state.loadMore();
						await settle();
						// Mock histories would otherwise retain every resolved API payload.
						searchModrinth.mockClear();
					}
					expect(state.itemCount).toBe(2000);
					expect(state.items.length).toBe(300);
					expect(state.cachedPageCount).toBe(15);
					state.destroy();
					expect(state.items.length).toBe(0);
					expect(state.cachedPageCount).toBe(0);
					expect(state.itemCount).toBe(0);
					dispose();
					dispose = undefined;
					state = undefined;
					refreshLocal = undefined;
					// Bun mock histories retain call frames as well as resolved payloads.
					for (const fn of new Set(Object.values(api)))
						fn.mockClear();
					unregisterRefresh.mockClear();
				};
				void runSession().then(done, fail);
			}, 0),
		);
		await settle();
		Bun.gc(true);
		await Bun.sleep(0);
		Bun.gc(true);
		const stats = heapStats();
		closedHeapBytes.push(stats.heapSize);
		if (!firstTypes) firstTypes = stats.objectTypeCounts;
		if (process.env.MARKET_MEMORY_PROFILE && session === sessions - 1) {
			console.info(
				"[market retention] object count deltas:",
				Object.entries(stats.objectTypeCounts)
					.map(([name, count]) => [
						name,
						count - (firstTypes[name] ?? 0),
					])
					.filter(([, count]) => count !== 0),
			);
		}
	}
	console.info(
		`[market retention] ${sessions} sessions × 2000 results; max 300 cached, 0 after destroy; JSC closed heap bytes:`,
		closedHeapBytes,
	);
}, 60000);
