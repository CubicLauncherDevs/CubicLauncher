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

test("the UI, native package and updater use the same full version including prerelease", () => {
	expect(nativeVersion).toBe(pkg.version);
	expect(tauri.version).toBe(pkg.version);
});

test("release tags describe the version embedded in the updater artifacts", () => {
	if (process.env.GITHUB_REF_TYPE !== "tag") return;
	expect(process.env.GITHUB_REF_NAME).toBe(`v${pkg.version}`);
});
