import { afterEach, beforeEach, expect, mock, spyOn, test } from "bun:test";
import { plugin, Transpiler } from "bun";
import { readFile } from "node:fs/promises";
import { compileModule } from "svelte/compiler";
import { flushSync } from "svelte";
import {
	derived,
	effect_root,
	get,
	render_effect,
} from "svelte/internal/client";

// Compile the real shared state, rather than poisoning the diagnostics tests
// with a process-wide module mock.
const transpiler = new Transpiler({ loader: "ts", target: "browser" });
plugin({
	name: "theme-manager-diagnostics-runes",
	setup(build) {
		build.onLoad(
			{ filter: /themeDiagnostics\.svelte\.ts$/ },
			async ({ path }) => ({
				contents: compileModule(
					transpiler.transformSync(await readFile(path, "utf8")),
					{ filename: path, generate: "client" },
				).js.code,
				loader: "js",
			}),
		);
	},
});

const { applyTheme, themeIcons, getThemeIcon, invalidateThemeCache } =
	await import("../src/lib/api/themeManager.ts");
const { themeDiagnostics } =
	await import("../src/lib/state/themeDiagnostics.svelte.ts");

let originals, spies, reads, deferreds, images, faces, registeredFonts;
let nodes,
	properties,
	blobs,
	invoke,
	fetchTheme,
	convert,
	createBlob,
	revokeBlob;
let errors, warnings, savedDiagnostics;
let resetId = 0;

function deferred() {
	let resolve, reject;
	const promise = new Promise((yes, no) => {
		resolve = yes;
		reject = no;
	});
	const result = { promise, resolve, reject };
	deferreds.push(result);
	return result;
}

function theme(name = "Empty", overrides = {}) {
	return {
		name,
		author: "Author",
		version: "1",
		type: "user",
		variables: { "--accent": name },
		fonts: [],
		icons: {},
		...overrides,
	};
}

function richTheme(name = "A", overrides = {}) {
	return theme(name, {
		fonts: [{ family: name, src: `/${name}.woff2` }],
		icons: { play: `/${name}.svg` },
		bg_image: `/${name}.png`,
		inject_css: `.card { color: ${name}; }`,
		...overrides,
	});
}

function respond(read, value) {
	read.resolve(
		read.kind === "fetch" ? { ok: true, json: async () => value } : value,
	);
}

async function commit(id, value, options) {
	const count = reads.length;
	const pending = applyTheme(id, options);
	expect(reads).toHaveLength(count + 1);
	respond(reads.at(-1), value);
	await pending;
}

function style(id = "cubic-theme-vars") {
	return nodes.find((node) => node.id === id);
}

function diagnostics() {
	return JSON.parse(JSON.stringify(themeDiagnostics));
}

function visuals() {
	return {
		styles: nodes.map(({ id, textContent, href, rel }) => ({
			id,
			textContent,
			href,
			rel,
		})),
		properties: [...properties],
		icons: [...themeIcons],
		fonts: [...registeredFonts],
		blobs: [...blobs.keys()],
		diagnostics: diagnostics(),
	};
}

async function settle() {
	await Bun.sleep(0);
}

beforeEach(async () => {
	originals = new Map();
	spies = [];
	reads = [];
	deferreds = [];
	images = [];
	faces = [];
	nodes = [];
	properties = new Map();
	blobs = new Map();
	registeredFonts = new Set();
	savedDiagnostics = diagnostics();
	const read = (kind, args) => {
		const result = { ...deferred(), kind, args };
		reads.push(result);
		return result.promise;
	};
	invoke = mock((...args) => read("invoke", args));
	fetchTheme = mock((...args) => read("fetch", args));
	convert = mock((path, protocol) => `${protocol}://localhost${path}`);
	const globals = {
		window: { __TAURI_INTERNALS__: { invoke, convertFileSrc: convert } },
		fetch: fetchTheme,
		document: {
			head: {
				appendChild: mock((node) => {
					nodes.push(node);
					return node;
				}),
			},
			documentElement: {
				style: {
					setProperty: mock((key, value) =>
						properties.set(key, value),
					),
					removeProperty: mock((key) => {
						const value = properties.get(key) ?? "";
						properties.delete(key);
						return value;
					}),
					getPropertyValue: (key) => properties.get(key) ?? "",
				},
			},
			getElementById: (id) => style(id) ?? null,
			createElement: (tagName) => {
				const node = {
					tagName,
					id: "",
					textContent: "",
					remove: mock(() => {
						const index = nodes.indexOf(node);
						if (index !== -1) nodes.splice(index, 1);
					}),
				};
				return node;
			},
			fonts: {
				add: mock((face) => registeredFonts.add(face)),
				delete: mock((face) => registeredFonts.delete(face)),
			},
		},
		Image: class {
			onload = null;
			onerror = null;
			src = "";
			naturalWidth = 8192;
			naturalHeight = 8192;
			removeAttribute = mock((name) => {
				if (name === "src") this.src = "";
			});
			constructor() {
				images.push(this);
			}
		},
		FontFace: class {
			constructor(family, src, descriptors) {
				if (family === "Invalid") throw new SyntaxError("Invalid font");
				Object.assign(this, { family, src, descriptors });
				this.pending = deferred();
				this.load = mock(() => this.pending.promise);
				faces.push(this);
			}
		},
	};
	for (const [name, value] of Object.entries(globals)) {
		originals.set(name, Object.getOwnPropertyDescriptor(globalThis, name));
		Object.defineProperty(globalThis, name, {
			value,
			configurable: true,
			writable: true,
		});
	}
	let nextBlob = 0;
	createBlob = spyOn(URL, "createObjectURL").mockImplementation((blob) => {
		const url = `blob:theme-test-${++nextBlob}`;
		blobs.set(url, blob);
		return url;
	});
	revokeBlob = spyOn(URL, "revokeObjectURL").mockImplementation((url) => {
		blobs.delete(url);
	});
	errors = spyOn(console, "error").mockImplementation(() => {});
	warnings = spyOn(console, "warn").mockImplementation(() => {});
	spies.push(createBlob, revokeBlob, errors, warnings);
	await commit(`user:__reset-${++resetId}`, theme());
	reads.length = 0;
	invoke.mockClear();
	convert.mockClear();
});

	afterEach(async () => {
		try {
			// Release manager-owned resources while the original fixture still exists.
			// Invalidate callbacks before draining even deferreds left by a failed test.
			invoke.mockReset().mockResolvedValue(theme());
			invalidateThemeCache();
			await applyTheme(`user:__reset-${++resetId}`, { force: true });
			for (const pending of deferreds) pending.resolve(undefined);
			await settle();
		} finally {
		themeDiagnostics.themeId = savedDiagnostics.themeId;
		themeDiagnostics.warnings = savedDiagnostics.warnings;
		for (const spy of spies) spy.mockRestore();
		for (const [name, descriptor] of originals) {
			if (descriptor) Object.defineProperty(globalThis, name, descriptor);
			else delete globalThis[name];
		}
	}
});

for (const id of ["user:A", "dark"]) {
	test(`${id}: concurrent same-ID requests share one read and promise`, async () => {
		const first = applyTheme(id);
		expect(applyTheme(id)).toBe(first);
		expect(reads).toHaveLength(1);
		respond(reads[0], richTheme());
		await first;
		expect(themeDiagnostics.themeId).toBe(id);
		expect(images).toHaveLength(1);
		expect(faces).toHaveLength(1);
		expect(createBlob).toHaveBeenCalledTimes(1);
		const before = visuals();
		await applyTheme(id);
		expect(reads).toHaveLength(1);
		expect(visuals()).toEqual(before);
	});
}

test("committed A -> pending B -> A supersedes B without reloading A", async () => {
	await commit("user:A", richTheme());
	const image = images[0];
	const onload = image.onload;
	const before = visuals();
	const pending = applyTheme("dark");
	const read = reads.at(-1);
	expect(visuals()).toEqual(before);
	await applyTheme("user:A");
	expect(read.args[1].signal.aborted).toBe(true);
	expect(reads).toHaveLength(2);
	respond(read, richTheme("B")); // Simulate a transport that ignores abort.
	await pending;
	expect(visuals()).toEqual(before);
	expect(image.onload).toBe(onload);
	expect(images).toHaveLength(1);
	expect(faces).toHaveLength(1);
	expect(revokeBlob).not.toHaveBeenCalled();
	expect(document.fonts.delete).not.toHaveBeenCalled();
	onload();
	faces[0].pending.resolve(faces[0]);
	await settle();
	expect(properties.get("--bg-image-loaded")).toBe("1");
	expect(properties.get("--font-loaded")).toBe("1");
});

test("out-of-order native responses only commit the newest request", async () => {
	const old = applyTheme("user:A");
	const latest = applyTheme("user:B");
	respond(reads[1], richTheme("B"));
	await latest;
	const before = visuals();
	respond(reads[0], richTheme("A"));
	await old;
	expect(visuals()).toEqual(before);
	expect(style().textContent).toContain("--accent: B;");
	expect(images).toHaveLength(1);
	expect(createBlob).toHaveBeenCalledTimes(1);
	expect(errors).not.toHaveBeenCalled();
});

for (const id of ["user:A", "dark"]) {
	test(`${id}: forced same-ID read supersedes pending and commits newest data`, async () => {
		await commit(id, theme("Initial"));
		const old = applyTheme(id, { force: true });
		const latest = applyTheme(id, { force: true });
		expect(latest).not.toBe(old);
		expect(reads).toHaveLength(3);
		if (id === "dark") expect(reads[1].args[1].signal.aborted).toBe(true);
		expect(applyTheme(id)).toBe(latest);
		respond(reads[2], richTheme("Newest"));
		await latest;
		const before = visuals();
		respond(reads[1], richTheme("Obsolete"));
		await old;
		expect(visuals()).toEqual(before);
		expect(style().textContent).toContain("--accent: Newest;");
		expect(createBlob).toHaveBeenCalledTimes(1);
	});
}

for (const outcome of ["resolve", "reject"]) {
	test(`old ${outcome}/finally cannot clear a newer pending request`, async () => {
		const old = applyTheme("user:A");
		const latest = applyTheme("user:B");
		if (outcome === "resolve") respond(reads[0], richTheme("Old"));
		else reads[0].reject(new Error("Obsolete failure"));
		await old;
		expect(applyTheme("user:B")).toBe(latest);
		expect(reads).toHaveLength(2);
		expect(errors).not.toHaveBeenCalled();
		respond(reads[1], theme("B"));
		await latest;
		expect(themeDiagnostics.themeId).toBe("user:B");
	});
}

for (const failure of ["invoke", "fetch", "http", "json"]) {
	test(`${failure} failure retains styles, icons, fonts and background and allows retry`, async () => {
		await commit("user:A", richTheme());
		images[0].onload();
		faces[0].pending.resolve(faces[0]);
		await settle();
		const before = visuals();
		const vars = style();
		const custom = style("cubic-theme-css");
		const id = failure === "invoke" ? "user:B" : "dark";
		const pending = applyTheme(id);
		const read = reads.at(-1);
		expect(visuals()).toEqual(before);
		const error = new Error(`${failure} failed`);
		if (failure === "http") read.resolve({ ok: false, status: 503 });
		else if (failure === "json") {
			const json = deferred();
			read.resolve({ ok: true, json: () => json.promise });
			await settle();
			expect(visuals()).toEqual(before);
			json.reject(error);
		} else read.reject(error);
		await pending;
		expect(errors).toHaveBeenCalledTimes(1);
		expect(visuals()).toEqual(before);
		expect(style()).toBe(vars);
		expect(style("cubic-theme-css")).toBe(custom);
		expect(revokeBlob).not.toHaveBeenCalled();
		expect(document.fonts.delete).not.toHaveBeenCalled();
		await commit(id, richTheme("Retry"));
		expect(style().textContent).toContain("--accent: Retry;");
		expect(themeDiagnostics.themeId).toBe(id);
		expect(reads).toHaveLength(3);
	});
}

test("a failed forced reload of the applied ID can be retried with force", async () => {
	await commit("user:A", richTheme());
	const before = visuals();
	const reload = applyTheme("user:A", { force: true });
	reads.at(-1).reject(new Error("Read failed"));
	await reload;
	expect(visuals()).toEqual(before);
	await commit("user:A", theme("Updated"), { force: true });
	expect(style().textContent).toContain("--accent: Updated;");
});

test("same-ID same-path reload updates the derived icon consumer and all asset revisions only on commit", async () => {
	spies.push(spyOn(Date, "now").mockReturnValue(123456789));
	const value = richTheme();
	await commit("user:A", value);
	const assets = () => ({
		icon: themeIcons.get("play"),
		background: images.at(-1).src,
		font: faces.at(-1).src,
	});
	const initial = assets();
	const seen = [];
	// Mirror Icon.svelte's $derived lookup and rendered mask URL. A direct
	// map read alone misses same-turn clear/set with an unchanged derived value.
	const dispose = effect_root(() => {
		const customIcon = derived(() => getThemeIcon("play"));
		const mask = derived(() => `url("${get(customIcon)}")`);
		render_effect(() => seen.push(get(mask)));
	});
	try {
		expect(seen).toEqual([`url("${initial.icon}")`]);
		await applyTheme("user:A");
		flushSync();
		expect(reads).toHaveLength(1);
		expect(assets()).toEqual(initial);
		expect(seen).toHaveLength(1);

		const failed = applyTheme("user:A", { force: true });
		expect(applyTheme("user:A")).toBe(failed);
		expect(reads).toHaveLength(2);
		flushSync();
		expect(assets()).toEqual(initial);
		expect(seen).toHaveLength(1);
		reads.at(-1).reject(new Error("Replacement read failed"));
		await failed;
		flushSync();
		expect(errors).toHaveBeenCalledTimes(1);
		expect(assets()).toEqual(initial);
		expect(seen).toHaveLength(1);

		const reload = applyTheme("user:A", { force: true });
		expect(applyTheme("user:A")).toBe(reload);
		expect(reads).toHaveLength(3);
		flushSync();
		expect(assets()).toEqual(initial);
		expect(seen).toHaveLength(1);
		respond(reads.at(-1), value); // Identical ID, paths and theme data.
		await reload;
		flushSync();
		const latest = assets();
		expect(seen).toEqual([
			`url("${initial.icon}")`,
			`url("${latest.icon}")`,
		]);
		for (const urls of [initial, latest]) {
			const revision = new URL(urls.icon).search;
			expect(revision).toMatch(/^\?theme-revision=123456789-\d+$/);
			expect(urls.icon).toBe(`asset://localhost/A.svg${revision}`);
			expect(urls.background).toBe(`asset://localhost/A.png${revision}`);
			expect(urls.font).toBe(`url(asset://localhost/A.woff2${revision})`);
		}
		for (const key of Object.keys(initial)) {
			expect(latest[key]).not.toBe(initial[key]);
		}
		images.at(-1).onload();
		faces.at(-1).pending.resolve(faces.at(-1));
		await settle();
		expect(properties.get("--bg-image")).toBe(
			`url("${latest.background}")`,
		);
		await applyTheme("user:A");
		flushSync();
		expect(reads).toHaveLength(3);
		expect(assets()).toEqual(latest);
		expect(seen).toHaveLength(2);
		expect(images).toHaveLength(2);
		expect(faces).toHaveLength(2);
	} finally {
		dispose();
	}
});

test("lazy icon consumers can render, reload and switch themes without reactive mutations", async () => {
	const icons = Object.fromEntries(
		Array.from({ length: 100 }, (_, index) => [
			`icon-${index}`,
			`/icon-${index}.svg`,
		]),
	);
	icons["ui:play"] = "/play.png";
	const value = theme("Lazy", { icons });
	await commit("dark", theme());
	const seen = [];
	const dispose = effect_root(() => {
		const customIcon = derived(() => getThemeIcon("ui:play"));
		render_effect(() => seen.push(get(customIcon)));
	});
	try {
		expect(seen).toEqual([null]);
		await commit("user:lazy", value);
		flushSync();
		const initial = seen.at(-1);
		expect(initial).toMatch(
			/^asset:\/\/localhost\/play\.png\?theme-revision=\d+-\d+$/,
		);
		const conversions = convert.mock.calls.length;
		expect(getThemeIcon("ui:play")).toBe(initial);
		expect(convert.mock.calls).toHaveLength(conversions);

		await commit("user:lazy", value, { force: true });
		flushSync();
		const reloaded = seen.at(-1);
		expect(reloaded).toMatch(
			/^asset:\/\/localhost\/play\.png\?theme-revision=\d+-\d+$/,
		);
		expect(reloaded).not.toBe(initial);

		await commit("dark", value, { force: true });
		flushSync();
		expect(seen.at(-1)).toBe("/play.png");

		await applyTheme("user:lazy");
		flushSync();
		const restored = seen.at(-1);
		expect(restored).toMatch(
			/^asset:\/\/localhost\/play\.png\?theme-revision=\d+-\d+$/,
		);
		expect(restored).not.toBe(reloaded);

		await commit("user:empty", theme());
		flushSync();
		expect(seen).toEqual([
			null, initial, reloaded, "/play.png", restored, null,
		]);
	} finally {
		dispose();
	}
});

test("current fonts and image finish while the next theme read is pending", async () => {
	await commit("user:A", richTheme());
	const pending = applyTheme("user:B");
	images[0].onload();
	faces[0].pending.resolve(faces[0]);
	await settle();
	expect(properties.get("--bg-image")).toBe(`url("${images[0].src}")`);
	expect(properties.get("--bg-image-loaded")).toBe("1");
	expect(properties.get("--font-loaded")).toBe("1");
	expect([...registeredFonts]).toEqual([faces[0]]);
	expect(diagnostics()).toEqual({
		themeId: "user:A",
		warnings: [
			{
				key: "themes.diagnostics.background",
				params: { width: 8192, height: 8192, mib: 256 },
			},
		],
	});
	respond(reads.at(-1), theme("B"));
	await pending;
	expect(registeredFonts.size).toBe(0);
	expect(properties.size).toBe(0);
	expect(diagnostics()).toEqual({ themeId: "user:B", warnings: [] });
});

for (const id of ["user:B", "user:A"]) {
	test(`${id}: replaced font and image callbacks cannot mutate the new generation`, async () => {
		await commit("user:A", richTheme());
		const oldImage = images[0];
		const onload = oldImage.onload;
		const onerror = oldImage.onerror;
		await commit(id, richTheme("New"), { force: true });
		expect(oldImage.onload).toBeNull();
		expect(oldImage.onerror).toBeNull();
		expect(oldImage.src).toBe("");
		expect(oldImage.removeAttribute).toHaveBeenCalledWith("src");
		const before = visuals();
		// Saved handlers model callbacks already queued before detachment.
		onload();
		onerror();
		faces[0].pending.resolve(faces[0]);
		await settle();
		expect(visuals()).toEqual(before);
		expect(document.fonts.add).not.toHaveBeenCalled();
		expect(properties.get("--font-loaded")).toBe("0");
		expect(properties.get("--bg-image-loaded")).toBe("0");
		images[1].onload();
		faces[1].pending.resolve(faces[1]);
		await settle();
		expect([...registeredFonts]).toEqual([faces[1]]);
		expect(properties.get("--font-loaded")).toBe("1");
		expect(properties.get("--bg-image")).toBe(`url("${images[1].src}")`);
		expect(diagnostics().warnings).toHaveLength(1);
	});
}

test("stale font settlement cannot restore --font-loaded after a fontless replacement", async () => {
	await commit("user:A", richTheme());
	await commit("user:A", theme("No fonts"), { force: true });
	faces[0].pending.resolve(faces[0]);
	await settle();
	expect(document.fonts.add).not.toHaveBeenCalled();
	expect(properties.has("--font-loaded")).toBe(false);
	expect(registeredFonts.size).toBe(0);
});

test("invalid font constructors and rejected loads do not prevent CSS or other fonts", async () => {
	const value = richTheme("A", {
		fonts: [
			{ family: "Invalid", src: "/invalid.woff2" },
			{ family: "Good", src: "/good.woff2" },
			{ family: "Broken", src: "/broken.woff2" },
		],
	});
	await commit("user:A", value);
	expect(style().textContent).toContain("--accent: A;");
	expect(style("cubic-theme-css").href).toBe("blob:theme-test-1");
	expect(await blobs.get("blob:theme-test-1").text()).toBe(value.inject_css);
	// Bun may append a charset to the Blob's MIME type.
	expect(blobs.get("blob:theme-test-1").type.split(";")[0]).toBe("text/css");
	expect(themeIcons.get("play")).toMatch(
		/^asset:\/\/localhost\/A\.svg\?theme-revision=\d+-\d+$/,
	);
	faces[0].pending.resolve(faces[0]);
	faces[1].pending.reject(new Error("Font download failed"));
	await settle();
	expect([...registeredFonts]).toEqual([faces[0]]);
	expect(properties.get("--font-loaded")).toBe("1");
	expect(warnings).toHaveBeenCalledTimes(2);
	expect(errors).not.toHaveBeenCalled();
});

test("repeated switches bound blob, stylesheet and registered-font counts", async () => {
	const defaults = style("cubic-default-fonts");
	expect(defaults.textContent.match(/@font-face/g)).toHaveLength(4);
	for (let index = 0; index < 6; index++) {
		const oldImage = images.at(-1);
		const oldVars = style();
		const oldLink = style("cubic-theme-css");
		await commit(
			`user:switch-${index}`,
			richTheme(`Font${index}`, {
				fonts: [
					{ family: `Regular${index}`, src: "/regular.woff2" },
					{
						family: `Bold${index}`,
						src: "/bold.woff2",
						weight: "700",
					},
				],
			}),
		);
		expect(oldVars.remove).toHaveBeenCalledTimes(1);
		if (oldLink) expect(oldLink.remove).toHaveBeenCalledTimes(1);
		if (oldImage) {
			expect(oldImage.src).toBe("");
			expect(oldImage.onload).toBeNull();
			expect(oldImage.onerror).toBeNull();
		}
		expect(registeredFonts.size).toBe(0);
		for (const face of faces.slice(-2)) face.pending.resolve(face);
		await settle();
		expect(registeredFonts.size).toBe(2);
		expect(document.fonts.add).toHaveBeenCalledTimes((index + 1) * 2);
		expect(document.fonts.delete).toHaveBeenCalledTimes(index * 2);
		expect(blobs.size).toBe(1);
		expect(createBlob).toHaveBeenCalledTimes(index + 1);
		expect(revokeBlob).toHaveBeenCalledTimes(index);
		expect(nodes.map((node) => node.id).sort()).toEqual([
			"cubic-default-fonts",
			"cubic-theme-css",
			"cubic-theme-vars",
		]);
	}
	// Also remove a legacy custom-font stylesheet if one is present.
	const legacy = document.createElement("style");
	legacy.id = "cubic-theme-fonts";
	document.head.appendChild(legacy);
	await commit("dark", theme());
	expect(legacy.remove).toHaveBeenCalledTimes(1);
	expect(registeredFonts.size).toBe(0);
	expect(document.fonts.delete).toHaveBeenCalledTimes(12);
	expect(blobs.size).toBe(0);
	expect(revokeBlob.mock.calls.map(([url]) => url)).toEqual(
		createBlob.mock.results.map(({ value }) => value),
	);
	expect(style("cubic-default-fonts")).toBe(defaults);
	expect(nodes).toHaveLength(2);
	expect(properties.size).toBe(0);
	expect(themeIcons.size).toBe(0);
});

for (const id of ["dark", "user:custom"]) {
	test(`${id}: URL conversion and CSS variable precedence preserve theme visuals`, async () => {
		const value = richTheme("A", {
			variables: {
				"--bg-image-blur": "7px",
				"--bg-image-opacity": "0.4",
				"--accent": "#97C459",
			},
			bg_image_blur: 30,
			bg_image_opacity: 0.9,
			icons: { play: "/A.svg", empty: "" },
			fonts: [
				{
					family: "Custom",
					src: "/A.woff2",
					weight: "700",
					style: "italic",
					format: "woff2",
				},
			],
		});
		const input = structuredClone(value);
		await commit(id, value);
		const prefix = id === "dark" ? "" : "asset://localhost";
		const revision =
			id === "dark" ? "" : new URL(themeIcons.get("play")).search;
		if (id !== "dark")
			expect(revision).toMatch(/^\?theme-revision=\d+-\d+$/);
		expect([...themeIcons]).toEqual([
			["play", `${prefix}/A.svg${revision}`],
		]);
		expect(images[0].src).toBe(`${prefix}/A.png${revision}`);
		expect(faces[0].src).toBe(`url(${prefix}/A.woff2${revision})`);
		expect(faces[0].descriptors).toEqual({
			weight: "700",
			style: "italic",
		});
		expect(faces[0].display).toBe("swap");
		expect(faces[0].load).toHaveBeenCalledTimes(1);
		expect(style().textContent).toBe(
			":root {\n  --bg-image-blur: 7px;\n  --bg-image-opacity: 0.4;\n  --accent: #97C459;\n}\n",
		);
		images[0].onload();
		expect(properties.get("--bg-image")).toBe(
			`url("${prefix}/A.png${revision}")`,
		);
		if (id === "dark") {
			expect(convert).not.toHaveBeenCalled();
			expect(fetchTheme).toHaveBeenCalledWith("/themes/dark/dark.json", {
				signal: expect.any(AbortSignal),
			});
		} else {
			expect(convert.mock.calls).toEqual([
				["/A.svg", "asset"],
				["/A.png", "asset"],
				["/A.woff2", "asset"],
			]);
			expect(invoke).toHaveBeenCalledWith(
				"get_user_theme",
				{ id: "custom" },
				undefined,
			);
		}
		expect(value).toEqual(input);
		await commit(
			id,
			theme("Fallback", {
				variables: {},
				bg_image_blur: 0,
				bg_image_opacity: 0,
			}),
			{ force: true },
		);
		expect(style().textContent).toBe(
			":root {\n  --bg-image-blur: 0px;\n  --bg-image-opacity: 0;\n}\n",
		);
		await commit(
			id,
			theme("Fallback", {
				variables: {},
				bg_image_blur: 30,
				bg_image_opacity: 0.75,
			}),
			{ force: true },
		);
		expect(style().textContent).toBe(
			":root {\n  --bg-image-blur: 30px;\n  --bg-image-opacity: 0.75;\n}\n",
		);
	});
}

test("diagnostics update on commit and only the current image can add dimensions", async () => {
	await commit(
		"user:A",
		richTheme("A", {
			inject_css: "a".repeat(256 * 1024 + 1),
		}),
	);
	const oldLoad = images[0].onload;
	expect(diagnostics()).toEqual({
		themeId: "user:A",
		warnings: [{ key: "themes.diagnostics.css", params: { kib: 257 } }],
	});
	const before = diagnostics();
	const reload = applyTheme("user:A", { force: true });
	expect(diagnostics()).toEqual(before);
	respond(reads.at(-1), richTheme("New"));
	await reload;
	expect(diagnostics()).toEqual({ themeId: "user:A", warnings: [] });
	oldLoad();
	expect(diagnostics().warnings).toEqual([]);
	const current = images[1];
	const currentLoad = current.onload;
	currentLoad();
	expect(diagnostics().warnings).toEqual([
		{
			key: "themes.diagnostics.background",
			params: { width: 8192, height: 8192, mib: 256 },
		},
	]);
	expect(current.onload).toBeNull();
	expect(current.onerror).toBeNull();
	current.naturalWidth = current.naturalHeight = 1;
	currentLoad(); // A finished image no longer owns callbacks either.
	expect(diagnostics().warnings).toHaveLength(1);
});

test("current image failure clears handlers without reporting decoded dimensions", async () => {
	await commit("user:A", richTheme());
	const image = images[0];
	const load = image.onload;
	image.onerror();
	expect(image.onload).toBeNull();
	expect(image.onerror).toBeNull();
	expect(properties.get("--bg-image")).toBe("none");
	expect(properties.get("--bg-image-loaded")).toBe("0");
	load();
	expect(properties.get("--bg-image")).toBe("none");
	expect(diagnostics().warnings).toEqual([]);
});

test("cached theme avoids a second read when re-selected later", async () => {
	await commit("user:A", richTheme("A"));
	await commit("dark", theme("Dark", { variables: { "--accent": "dark" } }));
	expect(themeDiagnostics.themeId).toBe("dark");
	reads.length = 0;

	const backToA = applyTheme("user:A");
	expect(reads).toHaveLength(0);
	await backToA;

	expect(style().textContent).toContain("--accent: A;");
	expect(themeDiagnostics.themeId).toBe("user:A");
});

test("force reload invalidates cache and reads fresh theme data", async () => {
	await commit("user:A", richTheme("A"));
	const before = visuals();
	reads.length = 0;

	const reload = applyTheme("user:A", { force: true });
	expect(reads).toHaveLength(1);
	respond(reads[0], richTheme("Updated"));
	await reload;
	expect(style().textContent).toContain("--accent: Updated;");
	expect(visuals()).not.toEqual(before);
});

test("theme cache is cleared after a failed read", async () => {
	await commit("user:A", richTheme("A"));
	const fail = applyTheme("user:A", { force: true });
	reads.at(-1).reject(new Error("Reload failed"));
	await fail;

	await commit("dark", theme());
	reads.length = 0;
	const retry = applyTheme("user:A");
	expect(reads).toHaveLength(1);
	respond(reads[0], richTheme("A"));
	await retry;
	expect(style().textContent).toContain("--accent: A;");
});

test("aborting a pending built-in read does not log an error", async () => {
	applyTheme("dark");
	const a = applyTheme("user:A"); // aborts the pending dark read
	respond(reads.at(-1), richTheme());
	await a;
	expect(errors).not.toHaveBeenCalled();
});
