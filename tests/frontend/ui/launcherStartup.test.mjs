import { expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { pathToFileURL } from "node:url";
import { compileModule } from "svelte/compiler";

// Exercise the real event service and installed-version state together, without
// process-wide mocks or a native Tauri runtime.
async function fixture() {
	const root = resolve(import.meta.dir, "../../..");
	const service = resolve(root, "src/lib/api/launcherService.ts");
	const stub = `
export const calls={downloads:0,queue:0,settings:0,themes:[],scans:0,unlistens:0};
export const launcherStore={settings:{theme:'test'},loadedInstances:[],pendingJreLaunch:null};
export const hooks={listener:null,scan:async()=>['1.20.1']};
export const listen=async(_,handler)=>{hooks.listener=handler;return ()=>{calls.unlistens++;hooks.listener=null}};
export const emit=payload=>hooks.listener?.({payload});
export const getInstalledVersions=()=>{calls.scans++;return hooks.scan()};
export const getSettings=async()=>{calls.settings++;return {theme:'test',language:'en'}};
export const applyTheme=(...args)=>calls.themes.push(args);
export const initDownloadState=()=>calls.downloads++;
export const initDownloadQueueState=()=>calls.queue++;
export const destroyDownloadState=()=>{};
export const destroyDownloadQueueState=()=>{};
export const showErrorParsed=()=>{};
export const clearPendingJreLaunch=()=>{};
export const updateSettings=async()=>{};
export const killInstance=()=>{};
export const initDiscordPresence=()=>{};
export const shutdownDiscordPresence=()=>{};
export const launchInstance=()=>{};
export const invoke=async()=>[];
`;
	const bundle = await Bun.build({
		entrypoints: ["startup-entry"],
		target: "bun",
		conditions: ["browser"],
		plugins: [
			{
				name: "launcher-startup",
				setup(build) {
					build.onResolve(
						{ filter: /^startup-(entry|stub)$/ },
						({ path }) => ({ path, namespace: "fixture" }),
					);
					build.onResolve({ filter: /.*/ }, ({ path, importer }) => {
						if (
							[
								"$lib/utils/saveQueue",
								"$lib/utils/versionUtils",
								"$lib/state/versionsState.svelte",
							].includes(path)
						) {
							return {
								path: resolve(
									root,
									path.replace("$lib", "src/lib") + ".ts",
								),
							};
						}
						if (
							importer === service ||
							path === "$lib/api/cubicApi"
						)
							return {
								path: "startup-stub",
								namespace: "fixture",
							};
					});
					build.onLoad(
						{ filter: /.*/, namespace: "fixture" },
						({ path }) => ({
							loader: "js",
							resolveDir: root,
							contents:
								path === "startup-stub"
									? stub
									: `
export {initEventListeners,destroyEventListeners} from ${JSON.stringify(service)};
export {loadInstalledVersions,versionsState} from '$lib/state/versionsState.svelte';
export * from 'startup-stub';`,
						}),
					);
					build.onLoad(
						{ filter: /versionsState\.svelte\.ts$/ },
						async ({ path }) => ({
							loader: "js",
							contents: compileModule(
								new Bun.Transpiler({
									loader: "ts",
								}).transformSync(await readFile(path, "utf8")),
								{ filename: path, generate: "client" },
							).js.code,
						}),
					);
				},
			},
		],
	});
	expect(bundle.success, bundle.logs.join("\n")).toBe(true);
	const directory = await mkdtemp(join(tmpdir(), "launcher-startup-"));
	try {
		const entry = join(directory, "fixture.mjs");
		await writeFile(entry, await bundle.outputs[0].text());
		return await import(pathToFileURL(entry).href);
	} finally {
		await rm(directory, { recursive: true, force: true });
	}
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 100));

test("log consoles keep live settings and themes without download or instance work", async () => {
	const f = await fixture();
	try {
		f.initEventListeners("logs");
		f.initEventListeners("logs");
		f.emit({ type: "InstanceCreated", data: { dto: { uuid: "unused" } } });
		f.emit({ type: "DFinish", data: { version: "1.20.1" } });
		f.emit({ type: "STChanged" });
		f.emit({ type: "ThemeChanged", data: { id: "other" } });
		f.emit({ type: "ThemeChanged", data: { id: "test" } });
		await settle();
		expect(f.calls.downloads).toBe(0);
		expect(f.calls.queue).toBe(0);
		expect(f.calls.scans).toBe(0);
		expect(f.launcherStore.loadedInstances).toEqual([]);
		expect(f.calls.settings).toBe(1);
		expect(f.launcherStore.settings.language).toBe("en");
		expect(f.calls.themes).toEqual([["test", { force: true }]]);
		f.emit({ type: "STChanged" });
	} finally {
		f.destroyEventListeners();
	}
	await settle();
	expect(f.calls.settings).toBe(1);
	expect(f.calls.unlistens).toBe(1);
});

test("completed downloads refresh installed versions without mounting a downloader", async () => {
	const f = await fixture();
	try {
		f.initEventListeners();
		f.initEventListeners();
		await f.loadInstalledVersions();
		expect(f.calls.downloads).toBe(1);
		expect(f.calls.queue).toBe(1);
		f.hooks.scan = async () => ["1.20.1", "1.21"];
		f.emit({ type: "DFinish", data: { version: "1.21" } });
		await f.loadInstalledVersions();
		expect([...f.versionsState.mcVersions.vanilla]).toEqual([
			"1.20.1",
			"1.21",
		]);
		expect(f.calls.scans).toBe(2);
	} finally {
		f.destroyEventListeners();
	}
});

test("downloads finishing during a scan coalesce into one trailing scan", async () => {
	const f = await fixture();
	let release;
	f.hooks.scan = () =>
		new Promise((resolve) => {
			release = resolve;
		});
	try {
		f.initEventListeners();
		const initial = f.loadInstalledVersions();
		for (let i = 0; i < 20; i++)
			f.emit({ type: "DFinish", data: { version: `version-${i}` } });
		const shared = f.loadInstalledVersions();
		expect(f.calls.scans).toBe(1);
		f.hooks.scan = async () => ["1.20.1", "1.21"];
		release(["1.20.1"]);
		await Promise.all([initial, shared]);
		expect(f.calls.scans).toBe(2);
		expect(f.versionsState.rawVersions).toEqual(["1.20.1", "1.21"]);
		expect(f.versionsState.loaded).toBe(true);
		expect(f.versionsState.loading).toBe(false);
	} finally {
		f.destroyEventListeners();
	}
});

test("a failed version scan remains retryable", async () => {
	const f = await fixture();
	f.hooks.scan = async () => {
		throw Error("disk unavailable");
	};
	await f.loadInstalledVersions();
	expect(f.versionsState.loaded).toBe(false);
	expect(f.versionsState.loading).toBe(false);
	expect(f.versionsState.error).toBe("disk unavailable");
	f.hooks.scan = async () => ["1.21"];
	await f.loadInstalledVersions();
	expect(f.versionsState.rawVersions).toEqual(["1.21"]);
	expect(f.versionsState.error).toBeNull();
});
