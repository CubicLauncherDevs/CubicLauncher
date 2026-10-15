import { expect, test } from "bun:test";
import { compileModule } from "svelte/compiler";
import { readFile, mkdtemp, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

async function fixture() {
	const root = resolve(import.meta.dir, "../../..");
	const entry = `
import {flushSync} from 'svelte';
import {lazyComponent} from './src/lib/utils/lazyComponent.svelte.ts';
export {flushSync};
export function create(load) {
	let open = $state(false);
	let panel;
	const errors = [];
	const destroy = $effect.root(() => {
		panel = lazyComponent(load, () => open, error => { errors.push(error); open = false });
	});
	return {panel, errors, destroy, setOpen(value) {open = value; flushSync()}};
}
`;
	const bundle = await Bun.build({
		entrypoints: ["lazy-entry"],
		target: "bun",
		conditions: ["browser"],
		plugins: [
			{
				name: "lazy-panel-fixture",
				setup(build) {
					build.onResolve({ filter: /^lazy-entry$/ }, ({ path }) => ({
						path,
						namespace: "fixture",
					}));
					build.onLoad(
						{ filter: /.*/, namespace: "fixture" },
						() => ({
							loader: "js",
							resolveDir: root,
							contents: compileModule(entry, {
								filename: "entry.svelte.js",
								generate: "client",
							}).js.code,
						}),
					);
					build.onLoad(
						{ filter: /lazyComponent\.svelte\.ts$/ },
						async ({ path }) => ({
							loader: "ts",
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
	const dir = await mkdtemp(join(tmpdir(), "lazy-panel-"));
	const path = join(dir, "fixture.mjs");
	await writeFile(path, await bundle.outputs[0].text());
	return {
		...(await import(path)),
		cleanup: () => rm(dir, { recursive: true, force: true }),
	};
}

test("panels defer loading, retain loaded components and ignore closed or destroyed work", async () => {
	const f = await fixture();
	let app;
	try {
		let calls = 0;
		let resolveLoad;
		const component = () => {};
		app = f.create(() => {
			calls++;
			return new Promise((resolve) => (resolveLoad = resolve));
		});
		f.flushSync();
		expect(calls).toBe(0);
		app.setOpen(true);
		expect(calls).toBe(1);
		app.setOpen(false);
		resolveLoad({ default: component });
		await Promise.resolve();
		expect(app.panel.component).toBeNull();
		app.setOpen(true);
		resolveLoad({ default: component });
		await Promise.resolve();
		f.flushSync();
		expect(app.panel.component).toBe(component);
		app.setOpen(false);
		app.setOpen(true);
		expect(calls).toBe(2);
		app.destroy();

		app = f.create(() => new Promise((resolve) => (resolveLoad = resolve)));
		app.setOpen(true);
		app.destroy();
		resolveLoad({ default: component });
		await Promise.resolve();
		expect(app.panel.component).toBeNull();
	} finally {
		app?.destroy();
		await f.cleanup();
	}
});

test("failed panel imports report once and can retry on the next opening", async () => {
	const f = await fixture();
	const error = new Error("import failed");
	let attempts = 0;
	const component = () => {};
	const app = f.create(async () => {
		if (++attempts === 1) throw error;
		return { default: component };
	});
	try {
		app.setOpen(true);
		await Promise.resolve();
		f.flushSync();
		expect(app.errors).toEqual([error]);
		expect(app.panel.component).toBeNull();
		app.setOpen(true);
		await Promise.resolve();
		f.flushSync();
		expect(app.panel.component).toBe(component);
		expect(attempts).toBe(2);
	} finally {
		app.destroy();
		await f.cleanup();
	}
});
