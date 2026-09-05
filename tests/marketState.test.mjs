import { afterEach, beforeEach, expect, mock, spyOn, test } from "bun:test";
import { plugin, Transpiler } from "bun";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { compileModule } from "svelte/compiler";
import { flushSync } from "svelte";
import { effect_root } from "svelte/internal/client";

// Bun strips types before Svelte compiles runes; browser conditions select the
// client runtime, so effects can run without a DOM or a mounted component.
const lib = resolve(import.meta.dir, "../src/lib");
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

let disk;
let refreshLocal;
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
	getInstanceMods: scan,
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
		return () => {};
	},
}));
mock.module(`${lib}/state/state.svelte`, () => ({ showWarning: mock() }));
mock.module(`${lib}/i18n`, () => ({ t: (key) => key }));

const { createMarketState } =
	await import("../src/lib/state/marketState.svelte.ts");
const { getMarketProjectId, localModToMarket } =
	await import("../src/lib/types/market");
const { InstState } = await import("../src/lib/types/types");
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
		expect(getModrinthProject).toHaveBeenLastCalledWith("shared");
		expect(getModrinthProjectVersions).toHaveBeenLastCalledWith(
			"shared",
			"",
			"1.21.1",
		);
	}
	await state.toggleEnabled(state.selectedProject);
	expect(toggleInstanceMod).not.toHaveBeenCalled();
	expect(disk.every((entry) => entry.enabled)).toBe(true);
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
	expect(getCurseForgeProject).toHaveBeenLastCalledWith(42);
	expect(getCurseForgeProjectFiles).toHaveBeenLastCalledWith(
		42,
		"fabric",
		"1.21.1",
	);
	expect(getCurseForgeProjectDescription).toHaveBeenLastCalledWith(42);
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
	expect(scan.mock.calls.length).toBe(scans + 2);

	// The operation's scan finishes first, but the newer scan owns the update.
	toggleScan.resolve(structuredClone(disk));
	await action;
	await settle();
	expect(state.selectedId).toBe("local-a.jar");
	expect(state.selectedProject?.installed?.filename).toBe("a.jar");

	winningScan.resolve(structuredClone(disk));
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
