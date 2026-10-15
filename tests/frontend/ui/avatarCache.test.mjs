import { expect, test } from "bun:test";
import { readFile, mkdtemp, writeFile, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

async function fixture() {
	const root = resolve(import.meta.dir, "../../..");
	const source = join(root, "src/lib/state/avatarCache.svelte.ts");
	const stub = `
export const hooks = {now:0, native:async()=>'<svg/>', fallback:async()=>new Response('<svg/>')};
export const calls = {native:0, fallback:0};
export const Date = {now:()=>hooks.now};
export const getAvatarSvg = (...args) => {calls.native++; return hooks.native(...args)};
export const fetch = (...args) => {calls.fallback++; return hooks.fallback(...args)};
`;
	const result = await Bun.build({
		entrypoints: ["avatar-entry"],
		target: "bun",
		conditions: ["browser"],
		plugins: [
			{
				name: "avatar-cache-fixture",
				setup(build) {
					build.onResolve(
						{ filter: /^avatar-(entry|stub)$/ },
						({ path }) => ({ path, namespace: "fixture" }),
					);
					build.onResolve(
						{ filter: /^\$lib\/api\/cubicApi$/ },
						() => ({ path: "avatar-stub", namespace: "fixture" }),
					);
					build.onResolve(
						{ filter: /offline-avatar\.svg\?raw$/ },
						() => ({ path: "offline", namespace: "fixture" }),
					);
					build.onLoad(
						{ filter: /.*/, namespace: "fixture" },
						({ path }) => ({
							loader: "js",
							resolveDir: root,
							contents:
								path === "avatar-entry"
									? `export * from ${JSON.stringify(source)}; export {hooks,calls} from 'avatar-stub';`
									: path === "avatar-stub"
										? stub
										: "export default '<svg>offline</svg>';",
						}),
					);
					build.onLoad(
						{ filter: /avatarCache\.svelte\.ts$/ },
						async ({ path }) => ({
							loader: "ts",
							contents:
								"import {fetch, Date} from 'avatar-stub';\n" +
								(await readFile(path, "utf8")),
						}),
					);
				},
			},
		],
	});
	expect(result.success, result.logs.join("\n")).toBe(true);
	const dir = await mkdtemp(join(tmpdir(), "avatar-cache-"));
	const path = join(dir, "fixture.mjs");
	await writeFile(path, await result.outputs[0].text());
	return {
		...(await import(path)),
		cleanup: () => rm(dir, { recursive: true, force: true }),
	};
}

test("simultaneous avatar consumers share one native request and cache the result", async () => {
	const f = await fixture();
	try {
		let release;
		f.hooks.native = () => new Promise((resolve) => (release = resolve));
		const requests = Array.from({ length: 20 }, () =>
			f.fetchAvatarSvg("user", "Microsoft"),
		);
		expect(f.calls.native).toBe(1);
		release("<svg>shared</svg>");
		expect(await Promise.all(requests)).toEqual(
			Array(20).fill("<svg>shared</svg>"),
		);
		expect(await f.fetchAvatarSvg("user", "Microsoft")).toBe(
			"<svg>shared</svg>",
		);
		expect(f.calls.native).toBe(1);
	} finally {
		await f.cleanup();
	}
});

test("avatar expiry only checks the requested entry and refreshing a full cache preserves other entries", async () => {
	const f = await fixture();
	try {
		for (let i = 0; i < 50; i++) f.setAvatar("url-" + i, "svg-" + i);
		f.setAvatar("url-25", "updated");
		expect(f.getAvatar("url-0")).toBe("svg-0");
		f.setAvatar("url-50", "new");
		expect(f.getAvatar("url-0")).toBeUndefined();
		expect(f.getAvatar("url-25")).toBe("updated");
		await f.fetchAvatarSvg("user", "Microsoft");
		f.hooks.now = 5 * 60 * 1000;
		expect(f.getAvatar("url-25")).toBeUndefined();
		await f.fetchAvatarSvg("user", "Microsoft");
		expect(f.calls.native).toBe(2);
	} finally {
		await f.cleanup();
	}
});

test("skin invalidation rejects old native results without deleting a replacement request", async () => {
	const f = await fixture();
	try {
		const releases = [];
		f.hooks.native = () => new Promise((resolve) => releases.push(resolve));
		const old = f.fetchAvatarSvg("user", "Microsoft");
		f.invalidateAvatarFor("user");
		const current = f.fetchAvatarSvg("user", "Microsoft");
		releases[0]("<svg>old</svg>");
		expect(await old).toBe("");
		const shared = f.fetchAvatarSvg("user", "Microsoft");
		expect(f.calls.native).toBe(2);
		releases[1]("<svg>current</svg>");
		expect(await current).toBe("<svg>current</svg>");
		expect(await shared).toBe("<svg>current</svg>");
		f.bumpAvatarVersion("user");
		const next = f.fetchAvatarSvg("user", "Microsoft");
		expect(f.calls.native).toBe(3);
		releases[2]("<svg>new skin</svg>");
		expect(await next).toBe("<svg>new skin</svg>");
	} finally {
		await f.cleanup();
	}
});

test("fallback fetches are shared and aborted when the skin changes", async () => {
	const f = await fixture();
	try {
		let rejectNative;
		let signal;
		let releaseFallback;
		f.hooks.native = () =>
			new Promise((_, reject) => (rejectNative = reject));
		f.hooks.fallback = (_, options) => {
			signal = options.signal;
			return new Promise((resolve) => (releaseFallback = resolve));
		};
		const first = f.fetchAvatarSvg("user", "Microsoft");
		const second = f.fetchAvatarSvg("user", "Microsoft");
		rejectNative(new Error("native avatar unavailable (expected in test)"));
		await Promise.resolve();
		expect(f.calls.fallback).toBe(1);
		f.bumpAvatarVersion("user");
		expect(signal.aborted).toBe(true);
		releaseFallback(new Response("<svg>stale fallback</svg>"));
		expect(await first).toBe("");
		expect(await second).toBe("");
		expect(f.getAvatar("user@0")).toBeUndefined();
	} finally {
		await f.cleanup();
	}
});

test("failed requests can retry and offline users need no native or network requests", async () => {
	const f = await fixture();
	try {
		f.hooks.native = async () => {
			throw new Error("expected native failure");
		};
		f.hooks.fallback = async () => new Response(null, { status: 503 });
		expect(await f.fetchAvatarSvg("user", "Microsoft")).toBe("");
		f.hooks.native = async () => "<svg>recovered</svg>";
		expect(await f.fetchAvatarSvg("user", "Microsoft")).toBe(
			"<svg>recovered</svg>",
		);
		expect(await f.fetchAvatarSvg("offline", "Cracked")).toBe(
			"<svg>offline</svg>",
		);
		expect(await f.fetchAvatarSvg("", "Microsoft")).toBe("");
		expect(f.calls).toEqual({ native: 2, fallback: 1 });
	} finally {
		await f.cleanup();
	}
});
