import { expect, test } from "bun:test";
import { resolve } from "node:path";

// Bundle the real service and queue in isolation; only its backend and unrelated
// startup dependencies are substituted. This avoids process-wide module mocks.
async function serviceFixture() {
	const root = resolve(import.meta.dir, "../../..");
	const service = resolve(root, "src/lib/api/launcherService.ts");
	const stub = `
export const launcherStore={settings:{discord_presence:false,interface_preferences:{scale:100,density:'theme'},notification_preferences:{title_size:13}}};
export const hooks={read:async()=>null,write:async()=>{}};
export const getSettings=()=>hooks.read();
export const updateSettings=settings=>hooks.write(structuredClone(settings));
export const showErrorParsed=()=>{};
export const clearPendingJreLaunch=()=>{};
export const listen=()=>{};
export const killInstance=()=>{};
export const initDiscordPresence=()=>{};
export const shutdownDiscordPresence=()=>{};
export const launchInstance=()=>{};
export const applyTheme=()=>{};
export const initDownloadState=()=>{};
export const destroyDownloadState=()=>{};
export const initDownloadQueueState=()=>{};
export const destroyDownloadQueueState=()=>{};
export const invoke=()=>{};
`;
	const bundle = await Bun.build({
		entrypoints: ["persistence-entry"],
		target: "bun",
		plugins: [
			{
				name: "settings-persistence",
				setup(build) {
					build.onResolve({ filter: /^persistence-entry$/ }, () => ({
						path: "entry",
						namespace: "fixture",
					}));
					build.onResolve({ filter: /^persistence-stub$/ }, () => ({
						path: "stub",
						namespace: "fixture",
					}));
					build.onResolve({ filter: /.*/ }, ({ path, importer }) => {
						if (path === "$lib/utils/saveQueue")
							return {
								path: resolve(
									root,
									"src/lib/utils/saveQueue.ts",
								),
							};
						if (importer === service)
							return { path: "stub", namespace: "fixture" };
					});
					build.onLoad(
						{ filter: /.*/, namespace: "fixture" },
						({ path }) => ({
							loader: "js",
							resolveDir: root,
							contents:
								path === "stub"
									? stub
									: `export {saveSettings,scheduleSettingsSave,flushSettingsSaves,syncSettings} from ${JSON.stringify(service)}; export {launcherStore,hooks} from 'persistence-stub';`,
						}),
					);
				},
			},
		],
	});
	expect(bundle.success, bundle.logs.join("\n")).toBe(true);
	return import(
		`data:text/javascript;base64,${Buffer.from(await bundle.outputs[0].text()).toString("base64")}`
	);
}

test("a stale settings read cannot overwrite a pending personalization edit", async () => {
	const {
		launcherStore,
		hooks,
		syncSettings,
		scheduleSettingsSave,
		flushSettingsSaves,
		saveSettings,
	} = await serviceFixture();
	let resolveRead;
	let reads = 0;
	hooks.read = () => {
		reads++;
		return new Promise((resolve) => {
			resolveRead = resolve;
		});
	};
	const writes = [];
	hooks.write = async (settings) => {
		writes.push(settings);
	};
	const staleRead = syncSettings();
	const oldSettings = structuredClone(launcherStore.settings);
	const requests = [];
	for (let i = 0; i < 100; i++) {
		launcherStore.settings.notification_preferences.title_size =
			12 + (i % 13);
		requests.push(scheduleSettingsSave());
	}
	await syncSettings();
	expect(reads).toBe(1);
	resolveRead(oldSettings);
	await staleRead;
	expect(launcherStore.settings.notification_preferences.title_size).toBe(20);
	await flushSettingsSaves();
	await Promise.all(requests);
	expect(writes).toHaveLength(1);
	expect(writes[0].notification_preferences.title_size).toBe(20);
	await syncSettings(); // Acknowledge the local STChanged without reloading.
	expect(reads).toBe(1);

	let release;
	hooks.write = async (settings) => {
		writes.push(settings);
		if (writes.length === 2)
			await new Promise((resolve) => {
				release = resolve;
			});
	};
	const first = saveSettings();
	await Promise.resolve();
	launcherStore.settings.interface_preferences.density = "compact";
	const trailing = scheduleSettingsSave();
	await syncSettings();
	expect(reads).toBe(1);
	release();
	await Promise.all([first, trailing]);
	expect(writes).toHaveLength(3);
	expect(writes[2].interface_preferences.density).toBe("compact");
});
