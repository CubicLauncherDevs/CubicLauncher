import { expect, test } from "bun:test";
import {
	catalogToGrouped,
	createCatalogCache,
} from "../../../src/lib/components/layout/VersionDownloader/versionCatalog.ts";

test("catalog adapter keeps the grouping and order decided by Rust", () => {
	const release = (mc, version, stable) => ({
		game_version: mc,
		version_id: `${mc}-${version}`,
		display_version: version,
		stable,
	});
	const catalog = catalogToGrouped({
		games: [
			{
				game_version: "1.21",
				has_stable: false,
				versions: [release("1.21", "HD_U_J1_pre9", false)],
			},
			{
				game_version: "1.20.1",
				has_stable: true,
				versions: [
					release("1.20.1", "HD_U_I6", true),
					release("1.20.1", "HD_U_I5", true),
					release("1.20.1", "HD_U_I7_pre1", false),
				],
			},
			{
				game_version: "1.9",
				has_stable: true,
				versions: [release("1.9", "HD_U_I6", true)],
			},
		],
	});
	expect(catalog.gameVersions).toEqual(["1.21", "1.20.1", "1.9"]);
	expect(catalog.stableGameVersions).toEqual(["1.20.1", "1.9"]);
	expect(catalog.byGame.get("1.20.1").map((v) => v.display_version)).toEqual([
		"HD_U_I6",
		"HD_U_I5",
		"HD_U_I7_pre1",
	]);
	expect(catalog.byGame.get("1.21")).toHaveLength(1);
});

test("catalog shares concurrent requests and refresh replaces cached data", async () => {
	const cache = createCatalogCache();
	let calls = 0;
	const load = async () => ++calls;
	const first = cache.get("fabric:1.20.1", load);
	expect(cache.get("fabric:1.20.1", load)).toBe(first);
	expect(await first).toBe(1);
	expect(await cache.get("fabric:1.20.1", load)).toBe(1);
	expect(await cache.get("fabric:1.20.1", load, true)).toBe(2);
	expect(await cache.get("fabric:1.20.1", load)).toBe(2);
	cache.clear();
	expect(await cache.get("fabric:1.20.1", load)).toBe(3);
});

test("failed requests can be retried and old failures cannot evict a refresh", async () => {
	const cache = createCatalogCache();
	await expect(
		cache.get("optifine", async () => {
			throw new Error("offline");
		}),
	).rejects.toThrow("offline");
	expect(await cache.get("optifine", async () => "retried")).toBe("retried");
	let reject;
	const old = cache.get(
		"optifine",
		() =>
			new Promise((_, fail) => {
				reject = fail;
			}),
		true,
	);
	const caught = old.catch(() => {});
	await Promise.resolve();
	expect(await cache.get("optifine", async () => "fresh", true)).toBe(
		"fresh",
	);
	reject(new Error("old request failed"));
	await caught;
	expect(await cache.get("optifine", async () => "unexpected")).toBe("fresh");
});
