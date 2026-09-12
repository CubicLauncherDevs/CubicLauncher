// Isolated process: other suites mock cubicApi globally, but this regression
// exercises the real launch API, JRE state and download-finished event handler.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { mock } from "bun:test";
import { plugin, Transpiler } from "bun";
import { compileModule } from "svelte/compiler";

const lib = resolve(import.meta.dir, "../../src/lib");
const transpiler = new Transpiler({ loader: "ts", target: "browser" });
plugin({
	name: "server-launch-runes",
	setup(build) {
		build.onResolve({ filter: /^\$lib\// }, ({ path }) => ({ path: Bun.resolveSync(path.replace(/^\$lib/, lib), import.meta.dir) }));
		build.onLoad({ filter: /\.svelte\.ts$/ }, async ({ path }) => ({
			contents: compileModule(transpiler.transformSync(await readFile(path, "utf8")), { filename: path, generate: "client" }).js.code,
			loader: "js",
		}));
	},
});

mock.module(resolve(lib, "i18n/index.ts"), () => ({ t: (key) => key }));
mock.module(resolve(lib, "api/themeManager.ts"), () => ({ applyTheme: async () => {} }));
mock.module(resolve(lib, "state/downloadState.svelte.ts"), () => ({ initDownloadState() {}, destroyDownloadState() {} }));
mock.module(resolve(lib, "state/downloadQueueState.svelte.ts"), () => ({ initDownloadQueueState() {}, destroyDownloadQueueState() {} }));
let onEvent;
mock.module("@tauri-apps/api/event", () => ({ listen: async (_name, callback) => { onEvent = callback; return () => {}; } }));
const launches = [];
let javaMissing = true;
globalThis.window = {
	__TAURI_INTERNALS__: {
		invoke: async (command, args) => {
			if (command === "launch") {
				launches.push(args);
				if (javaMissing) throw JSON.stringify({ code: "INST_JRE_MISSING", params: { version: "21" } });
			}
		},
	},
};

const { launcherStore, setPendingJreLaunch, dismissJreInstallPrompt } = await import(resolve(lib, "state/state.svelte.ts"));
const { launchInstance } = await import(resolve(lib, "api/cubicApi.ts"));
const { initEventListeners, destroyEventListeners } = await import(resolve(lib, "api/launcherService.ts"));
const instance = { uuid: "550e8400-e29b-41d4-a716-446655440000", name: "Test", version: "1.21.1" };
await launchInstance(instance, undefined, undefined, "play.example:25566");
assert.equal(launcherStore.jreInstallPrompt.serverAddress, "play.example:25566");
const prompt = launcherStore.jreInstallPrompt;
setPendingJreLaunch(prompt.version, prompt.instance, prompt.serverAddress);
dismissJreInstallPrompt();
initEventListeners();
javaMissing = false;
onEvent({ payload: { type: "DFinish", data: { version: "jre-17" } } });
await Bun.sleep(0);
assert.equal(launches.length, 1);
onEvent({ payload: { type: "DFinish", data: { version: "jre-21" } } });
await Bun.sleep(0);
assert.equal(launches.length, 2);
assert.deepEqual(launches[1], { instanceId: instance.uuid, serverAddress: "play.example:25566" });
assert.equal(launcherStore.pendingJreLaunch, null);
await launchInstance(instance);
assert.equal(launches.at(-1).serverAddress, undefined);
destroyEventListeners();
