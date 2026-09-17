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
	unregisterRefresh.mockClear();
	disk = [file("a.jar"), file("b.jar")];
	for (const fn of new Set(Object.values(api))) fn.mockClear();
	scan.mockReset().mockImplementation(async () => structuredClone(disk));
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
		const project = state.items[0];
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
