import { expect, test } from "bun:test";
import { compileModule } from "svelte/compiler";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { pathToFileURL } from "node:url";

async function fixture() {
	const root = resolve(import.meta.dir, "../../..");
	const source = join(root, "src/lib/i18n/index.ts");
	const bundle = await Bun.build({
		entrypoints: ["i18n-entry"],
		target: "bun",
		conditions: ["browser"],
		plugins: [
			{
				name: "i18n-fixture",
				setup(build) {
					build.onResolve(
						{ filter: /^i18n-(entry|stub)$/ },
						({ path }) => ({ path, namespace: "fixture" }),
					);
					build.onResolve(
						{
							filter: /^(@tauri-apps\/api\/core|\$lib\/state\/state.svelte)$/,
						},
						() => ({ path: "i18n-stub", namespace: "fixture" }),
					);
					build.onLoad(
						{ filter: /.*/, namespace: "fixture" },
						({ path }) => ({
							loader: "js",
							resolveDir: root,
							contents:
								path === "i18n-entry"
									? `export * from ${JSON.stringify(source)};export * from 'i18n-stub';
import {t} from ${JSON.stringify(source)};
import {effect_root,render_effect,derived,get} from 'svelte/internal/client';
export {flushSync} from 'svelte';
export const observeTranslation=onValue=>effect_root(()=>{const text=derived(()=>t('greeting',{name:'X'}));render_effect(()=>onValue(get(text)))});`
									: compileModule(
											`
export const launcherStore=$state({settings:{language:'en'}});
export const calls=[];
export const hooks={invoke:async()=>null};
export const invoke=(command,args)=>{calls.push({command,args});return hooks.invoke(command,args)};
`,
											{
												filename: "stub.svelte.js",
												generate: "client",
											},
										).js.code,
						}),
					);
					build.onLoad(
						{ filter: /loader\.svelte\.ts$/ },
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
	const dir = await mkdtemp(join(tmpdir(), "i18n-fixture-"));
	try {
		const entry = join(dir, "entry.mjs");
		await writeFile(entry, await bundle.outputs[0].text());
		return await import(pathToFileURL(entry).href);
	} finally {
		await rm(dir, { recursive: true, force: true });
	}
}

const stored = (code, extra = {}) => ({
	code,
	id: `${code}-XX`,
	data: JSON.stringify({
		id: `${code}-XX`,
		version: "1",
		greeting: `Hello ${code} {name}`,
		...extra,
	}),
});
const settle = async () => {
	for (let i = 0; i < 12; i++) await Promise.resolve();
};
function deferred() {
	let resolve, reject;
	const promise = new Promise((yes, no) => {
		resolve = yes;
		reject = no;
	});
	return { promise, resolve, reject };
}

test("bundled translations and idle imports do not scan stored dictionaries or list languages", async () => {
	const f = await fixture();
	expect(f.calls).toEqual([]);
	expect(f.t("common.cancel")).toBe("Cancel");
	f.launcherStore.settings.language = "es";
	await f.downloadLocale("es");
	expect(f.t("common.cancel")).toBe("Cancelar");
	expect(f.calls).toEqual([]);
	expect(f.locales.map((l) => l.code)).toContain("en");
});

test("derived translations update automatically after lazy loading and language changes", async () => {
	const f = await fixture();
	f.hooks.invoke = (command, args) =>
		command === "load_locale" ? stored(args.code) : null;
	const values = [];
	const stop = f.observeTranslation((value) => values.push(value));
	try {
		f.launcherStore.settings.language = "fr";
		f.flushSync();
		await settle();
		f.flushSync();
		expect(values.at(-1)).toBe("Hello fr X");
		f.launcherStore.settings.language = "de";
		f.flushSync();
		await settle();
		f.flushSync();
		expect(values.at(-1)).toBe("Hello de X");
		f.launcherStore.settings.language = "fr";
		f.flushSync();
		expect(values.at(-1)).toBe("Hello fr X");
		expect(f.calls.filter((c) => c.command === "load_locale")).toHaveLength(
			2,
		);
	} finally {
		stop();
	}
});

test("the selected offline dictionary is usable before its remote check completes", async () => {
	const f = await fixture();
	const refresh = deferred();
	f.hooks.invoke = (command, args) =>
		command === "load_locale" ? stored(args.code) : refresh.promise;
	f.launcherStore.settings.language = "fr";
	const request = f.downloadLocale("fr");
	await settle();
	expect(f.t("greeting", { name: "Alex" })).toBe("Hello fr Alex");
	expect(f.t("common.cancel")).toBe("Cancel");
	expect(f.calls.map((c) => c.command)).toEqual([
		"load_locale",
		"refresh_locale",
	]);
	expect(f.calls[1].args.knownVersion).toBe("1");
	refresh.reject(Error("offline"));
	await request;
	expect(f.t("greeting", { name: "Alex" })).toBe("Hello fr Alex");
	expect(f.isFetched("fr")).toBe(true);
});

test("concurrent lookups share a request and late languages cannot replace the current one", async () => {
	const f = await fixture();
	const french = deferred();
	f.hooks.invoke = (command, args) =>
		command === "load_locale"
			? args.code === "fr"
				? french.promise
				: stored(args.code)
			: null;
	f.launcherStore.settings.language = "fr";
	for (let i = 0; i < 100; i++) f.t("greeting");
	const a = f.downloadLocale("fr"),
		b = f.downloadLocale("fr");
	await settle();
	expect(f.calls.filter((c) => c.command === "load_locale")).toHaveLength(1);
	f.launcherStore.settings.language = "de";
	await f.downloadLocale("de");
	french.resolve(stored("fr"));
	await Promise.all([a, b]);
	expect(f.t("greeting", { name: "A" })).toBe("Hello de A");
	f.launcherStore.settings.language = "fr";
	expect(f.t("greeting", { name: "B" })).toBe("Hello fr B");
});

test("inactive dictionaries are evicted and reload from disk without losing installed status", async () => {
	const f = await fixture();
	f.hooks.invoke = (command, args) =>
		command === "load_locale" ? stored(args.code) : null;
	for (const code of ["fr", "de", "ja", "uk", "pt"]) {
		f.launcherStore.settings.language = code;
		await f.downloadLocale(code);
		expect(f.t("greeting", { name: "X" })).toBe(`Hello ${code} X`);
	}
	expect(f.isFetched("fr")).toBe(true);
	f.launcherStore.settings.language = "fr";
	await f.downloadLocale("fr");
	expect(
		f.calls.filter(
			(c) => c.command === "load_locale" && c.args.code === "fr",
		),
	).toHaveLength(2);
	expect(f.t("greeting", { name: "X" })).toBe("Hello fr X");
});

test("a large active dictionary remains usable but is released after switching to English", async () => {
	const f = await fixture();
	f.hooks.invoke = (command, args) =>
		command === "load_locale"
			? stored(args.code, { large: "x".repeat(2 * 1024 * 1024) })
			: null;
	f.launcherStore.settings.language = "fr";
	await f.downloadLocale("fr");
	expect(f.t("large").length).toBe(2 * 1024 * 1024);
	f.launcherStore.settings.language = "en";
	expect(f.t("common.cancel")).toBe("Cancel");
	f.launcherStore.settings.language = "fr";
	await f.downloadLocale("fr");
	expect(f.calls.filter((c) => c.command === "load_locale")).toHaveLength(2);
});

test("opening the selector requests metadata only and deduplicates concurrent opens", async () => {
	const f = await fixture();
	f.hooks.invoke = async () => [
		{
			code: "fr",
			id: "fr-FR",
			label: "Français",
			flag: "🇫🇷",
			installed: true,
		},
	];
	await Promise.all([f.loadAvailableLocales(), f.loadAvailableLocales()]);
	expect(f.calls.map((c) => c.command)).toEqual(["list_locales"]);
	expect(f.locales.map((l) => l.code)).toEqual(["fr", "es", "en"]);
	expect(f.isFetched("fr")).toBe(true);
});

test("missing dictionaries fall back without retry loops and allow an explicit retry", async () => {
	const f = await fixture();
	f.hooks.invoke = async () => null;
	f.launcherStore.settings.language = "fr";
	await f.downloadLocale("fr");
	for (let i = 0; i < 100; i++) expect(f.t("common.cancel")).toBe("Cancel");
	await settle();
	expect(f.calls).toHaveLength(2);
	f.hooks.invoke = (command, args) =>
		command === "load_locale" ? stored(args.code) : null;
	await f.downloadLocale("fr");
	expect(f.t("greeting", { name: "X" })).toBe("Hello fr X");
});
