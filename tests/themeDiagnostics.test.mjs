import { beforeEach, expect, test } from "bun:test";
import { plugin, Transpiler } from "bun";
import { readFile } from "node:fs/promises";
import { compileModule } from "svelte/compiler";
import { flushSync } from "svelte";
import { effect_root, render_effect } from "svelte/internal/client";

const transpiler = new Transpiler({ loader: "ts", target: "browser" });
plugin({
	name: "theme-diagnostics-runes",
	setup(build) {
		build.onLoad(
			{ filter: /themeDiagnostics\.svelte\.ts$/ },
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

const { themeDiagnostics, setThemeDiagnostics, setThemeBackgroundDimensions } =
	await import("../src/lib/state/themeDiagnostics.svelte.ts");

function theme(overrides = {}) {
	return {
		name: "Test",
		author: "Author",
		version: "1",
		type: "user",
		variables: {},
		fonts: [],
		icons: {},
		...overrides,
	};
}

function warnings() {
	return JSON.parse(JSON.stringify(themeDiagnostics.warnings));
}

beforeEach(() => setThemeDiagnostics("dark", theme()));

test("default and lightweight customization do not trigger advisories", () => {
	expect(warnings()).toEqual([]);
	setThemeDiagnostics(
		"user:lightweight",
		theme({
			fonts: [{ family: "Custom", src: "/font.woff2" }],
			icons: { play: "/play.svg" },
			inject_css:
				":root { --accent: red; } .card { backdrop-filter: blur(30px); }",
			bg_image: "/background.png",
			bg_image_blur: 30,
		}),
	);
	setThemeBackgroundDimensions("user:lightweight", 3840, 2160);
	expect(warnings()).toEqual([]);
});

test("font advisory uses face count, strictly above twelve", () => {
	const fonts = Array.from({ length: 12 }, () => ({
		family: "Same family",
		src: "/font.woff2",
	}));
	setThemeDiagnostics("fonts", theme({ fonts }));
	expect(warnings()).toEqual([]);
	setThemeDiagnostics("fonts", theme({ fonts: [...fonts, fonts[0]] }));
	expect(warnings()).toEqual([
		{ key: "themes.diagnostics.fonts", params: { count: 13 } },
	]);
});

test("CSS advisory is strictly above 256 KiB of UTF-8, not character count", () => {
	setThemeDiagnostics("css", theme({ inject_css: "a".repeat(256 * 1024) }));
	expect(warnings()).toEqual([]);
	setThemeDiagnostics(
		"css",
		theme({ inject_css: "a".repeat(256 * 1024 + 1) }),
	);
	expect(warnings()).toEqual([
		{ key: "themes.diagnostics.css", params: { kib: 257 } },
	]);
	setThemeDiagnostics(
		"css",
		theme({ inject_css: "\u00e9".repeat(128 * 1024) }),
	);
	expect(warnings()).toEqual([]);
	setThemeDiagnostics(
		"css",
		theme({ inject_css: "\u00e9".repeat(128 * 1024 + 1) }),
	);
	expect(warnings()).toEqual([
		{ key: "themes.diagnostics.css", params: { kib: 257 } },
	]);
});

test("background threshold uses total pixels and estimates one RGBA surface", () => {
	setThemeBackgroundDimensions("dark", 8192, 2048);
	expect(warnings()).toEqual([]);
	setThemeBackgroundDimensions("dark", 4096, 4097);
	expect(warnings()).toEqual([
		{
			key: "themes.diagnostics.background",
			params: { width: 4096, height: 4097, mib: 65 },
		},
	]);
});

test("background updates replace rather than duplicate and preserve other advisories", () => {
	setThemeDiagnostics(
		"heavy",
		theme({ inject_css: "a".repeat(256 * 1024 + 1) }),
	);
	setThemeBackgroundDimensions("heavy", 8192, 8192);
	setThemeBackgroundDimensions("heavy", 8192, 8192);
	expect(warnings()).toHaveLength(2);
	expect(warnings()[1].params.mib).toBe(256);
	setThemeBackgroundDimensions("heavy", 1920, 1080);
	expect(warnings()).toEqual([
		{ key: "themes.diagnostics.css", params: { kib: 257 } },
	]);
});

test("every commit clears prior advisories, including a same-ID reload", () => {
	for (const id of ["dark", "user:new"]) {
		setThemeDiagnostics(
			"dark",
			theme({ inject_css: "a".repeat(256 * 1024 + 1) }),
		);
		setThemeBackgroundDimensions("dark", 8192, 8192);
		setThemeDiagnostics(id, theme());
		expect(themeDiagnostics.themeId).toBe(id);
		expect(warnings()).toEqual([]);
	}
});

test("stale image dimensions cannot change the active theme's warnings", () => {
	setThemeBackgroundDimensions("dark", 8192, 8192);
	const before = warnings();
	setThemeBackgroundDimensions("user:old", 10000, 10000);
	setThemeBackgroundDimensions("user:old", 1, 1);
	expect(themeDiagnostics.themeId).toBe("dark");
	expect(warnings()).toEqual(before);
});

test("invalid dimensions do not produce misleading estimates or erase warnings", () => {
	setThemeBackgroundDimensions("dark", 8192, 8192);
	const before = warnings();
	for (const invalid of [
		0,
		-1,
		NaN,
		Infinity,
		4096.5,
		Number.MAX_SAFE_INTEGER,
	]) {
		setThemeBackgroundDimensions("dark", invalid, 8192);
		setThemeBackgroundDimensions("dark", 8192, invalid);
	}
	expect(warnings()).toEqual(before);
});

test("diagnostics remain advisory and never mutate or load theme resources", () => {
	const input = theme({
		fonts: Object.freeze(
			Array.from({ length: 13 }, () =>
				Object.freeze({ family: "Font", src: "/font.woff2" }),
			),
		),
		icons: Object.freeze({ play: "/icon.svg" }),
		variables: Object.freeze({ "--bg-image-blur": "80px" }),
		inject_css: "a".repeat(256 * 1024 + 1),
		bg_image: "/original.png",
		bg_image_blur: 80,
		bg_image_opacity: 0.75,
	});
	const before = structuredClone(input);
	Object.freeze(input);
	// No DOM, image decoder, font loader, or native API is available in this test.
	expect(setThemeDiagnostics("user:heavy", input)).toBeUndefined();
	expect(
		setThemeBackgroundDimensions("user:heavy", 8192, 8192),
	).toBeUndefined();
	expect(warnings()).toHaveLength(3);
	expect(input).toEqual(before);
});

test("commits and dimension updates notify reactive readers", () => {
	const seen = [];
	const dispose = effect_root(() => {
		render_effect(() =>
			seen.push([
				themeDiagnostics.themeId,
				themeDiagnostics.warnings.length,
			]),
		);
	});
	try {
		setThemeDiagnostics("user:active", theme());
		flushSync();
		setThemeBackgroundDimensions("user:active", 8192, 8192);
		flushSync();
		setThemeDiagnostics("light", theme());
		flushSync();
		expect(seen).toEqual([
			["dark", 0],
			["user:active", 0],
			["user:active", 1],
			["light", 0],
		]);
	} finally {
		dispose();
	}
});

test("bundled translations provide matching diagnostic keys and placeholders", async () => {
	const locales = await Promise.all(
		["en-US", "es-ES"].map(
			async (locale) =>
				JSON.parse(
					await readFile(
						new URL(
							`../src/lib/i18n/${locale}.json`,
							import.meta.url,
						),
						"utf8",
					),
				).themes.diagnostics,
		),
	);
	expect(Object.keys(locales[0]).sort()).toEqual(
		Object.keys(locales[1]).sort(),
	);
	for (const key of Object.keys(locales[0])) {
		expect(locales[0][key]).not.toBe("");
		expect(locales[1][key]).not.toBe("");
		expect(locales[0][key].match(/\{\w+\}/g)).toEqual(
			locales[1][key].match(/\{\w+\}/g),
		);
	}
});
