import { expect, test } from "bun:test";
import { resolve } from "node:path";

// Bundle the real API wrapper with only `@tauri-apps/api/core` substituted, so
// the tests assert the exact command names and camelCase argument mapping the
// Rust commands expect, without process-wide module mocks.
async function apiFixture() {
	const root = resolve(import.meta.dir, "../../..");
	const api = resolve(root, "src/lib/api/optionsProfiles.ts");
	const stub = `
export const calls = [];
export const invoke = async (cmd, args) => { calls.push([cmd, args]); return { cmd, args }; };
`;
	const bundle = await Bun.build({
		entrypoints: ["options-entry"],
		target: "bun",
		plugins: [
			{
				name: "options-profiles",
				setup(build) {
					build.onResolve({ filter: /^options-entry$/ }, () => ({
						path: "entry",
						namespace: "fixture",
					}));
					build.onResolve(
						{ filter: /^@tauri-apps\/api\/core$/ },
						() => ({ path: "stub", namespace: "fixture" }),
					);
					build.onResolve({ filter: /^options-stub$/ }, () => ({
						path: "stub",
						namespace: "fixture",
					}));
					build.onLoad(
						{ filter: /.*/, namespace: "fixture" },
						({ path }) => ({
							loader: "js",
							resolveDir: root,
							contents:
								path === "stub"
									? stub
									: `export * from ${JSON.stringify(api)}; export { calls } from 'options-stub';`,
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

test("profile commands map to the expected names and arguments", async () => {
	const api = await apiFixture();
	api.calls.length = 0;
	await api.listOptionsProfiles();
	await api.createOptionsProfile("PvP", "fov:0.5");
	await api.updateOptionsProfile("id1", "PvP", "fov:1");
	await api.deleteOptionsProfile("id1");
	await api.createOptionsProfileFromInstance("inst1", "From instance");
	await api.importOptionsProfileFromFile("/tmp/options.txt", "");

	expect(api.calls).toEqual([
		["list_options_profiles", undefined],
		["create_options_profile", { name: "PvP", content: "fov:0.5" }],
		["update_options_profile", { id: "id1", name: "PvP", content: "fov:1" }],
		["delete_options_profile", { id: "id1" }],
		["create_options_profile_from_instance", { instanceId: "inst1", name: "From instance" }],
		["import_options_profile_from_file", { path: "/tmp/options.txt", name: "" }],
	]);
});

test("apply and assignment map profile id and flags", async () => {
	const api = await apiFixture();
	api.calls.length = 0;
	await api.applyOptionsProfile("inst1", "id1", false);
	await api.applyOptionsProfile("inst1", "id1", true);
	await api.setInstanceOptionsProfile("inst1", null, true);
	await api.setInstanceOptionsProfile("inst1", "id1", false);

	expect(api.calls).toEqual([
		["apply_options_profile", { id: "inst1", profileId: "id1", overwrite: false }],
		["apply_options_profile", { id: "inst1", profileId: "id1", overwrite: true }],
		["set_instance_options_profile", { id: "inst1", profileId: null, autoApply: true }],
		["set_instance_options_profile", { id: "inst1", profileId: "id1", autoApply: false }],
	]);
});

test("optionsErrorCode extracts the Rust error code", async () => {
	const { optionsErrorCode } = await apiFixture();
	expect(optionsErrorCode('{"code":"INST_OPTIONS_EXISTS"}')).toBe(
		"INST_OPTIONS_EXISTS",
	);
	expect(optionsErrorCode("not json")).toBeNull();
	expect(optionsErrorCode(new Error("boom"))).toBeNull();
	expect(optionsErrorCode(undefined)).toBeNull();
});
