import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "../../..");
const pkg = JSON.parse(await readFile(resolve(root, "package.json"), "utf8"));
const tauri = JSON.parse(
	await readFile(resolve(root, "src-tauri/tauri.conf.json"), "utf8"),
);
const cargo = await readFile(resolve(root, "src-tauri/Cargo.toml"), "utf8");
const nativeVersion = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1];

const cargoPluginVersions = new Map();
for (const match of cargo.matchAll(
	/^tauri-plugin-([a-z0-9-]+)\s*=\s*"=?(\d+)\.(\d+)/gm,
)) {
	cargoPluginVersions.set(match[1], `${match[2]}.${match[3]}`);
}

test("the UI, native package and updater use the same full version including prerelease", () => {
	expect(nativeVersion).toBe(pkg.version);
	expect(tauri.version).toBe(pkg.version);
});

test("Tauri Rust and JavaScript plugins share the same major/minor version", () => {
	for (const [packageName, packageVersion] of Object.entries(
		pkg.dependencies ?? {},
	)) {
		if (!packageName.startsWith("@tauri-apps/plugin-")) continue;

		const plugin = packageName.slice("@tauri-apps/plugin-".length);
		const rustVersion = cargoPluginVersions.get(plugin);
		if (!rustVersion) continue;

		const jsVersion = String(packageVersion).match(/^(\d+)\.(\d+)/);
		expect(
			jsVersion,
			`${packageName} must use a numeric version`,
		).not.toBeNull();
		expect(`${jsVersion[1]}.${jsVersion[2]}`).toBe(rustVersion);
	}
});

test("release tags describe the version embedded in the updater artifacts", () => {
	if (process.env.GITHUB_REF_TYPE !== "tag") return;
	expect(process.env.GITHUB_REF_NAME).toBe(`v${pkg.version}`);
});
