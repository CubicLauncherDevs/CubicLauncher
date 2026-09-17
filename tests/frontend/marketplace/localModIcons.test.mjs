import { expect, test } from "bun:test";
import { createLocalModIcons } from "../../../src/lib/state/localModIcons.ts";

function projects(count) {
	return Array.from({ length: count }, (_, i) => ({
		icon: null,
		installed: { filename: `${i}.jar`, icon_revision: String(i) },
	}));
}

test("icon cache bounds entries while scrolling through a large pack", async () => {
	const calls = [];
	const cache = createLocalModIcons(async (files) => {
		calls.push(files);
		return files.map((file) => ({
			...file,
			icon: `image-${file.filename}`,
		}));
	});
	try {
		const mods = projects(200);
		cache.sync(mods);
		for (let first = 0; first < 200; first += 24) {
			cache.request(mods.slice(first, first + 24));
			await Bun.sleep(90);
		}
		expect(calls.every((batch) => batch.length <= 24)).toBe(true);
		expect(cache.count).toBeLessThanOrEqual(160);
		expect(cache.get(mods[199])).toBe("image-199.jar");
	} finally {
		cache.destroy();
	}
	expect(cache.count).toBe(0);
	expect(cache.retainedBytes).toBe(0);
});

test("large icons cannot exceed the byte budget or create an eviction/refetch loop", async () => {
	let requests = 0;
	const cache = createLocalModIcons(async (files) => {
		requests++;
		return files.map((file) => ({ ...file, icon: "x".repeat(256 * 1024) }));
	});
	try {
		const mods = projects(72);
		cache.sync(mods);
		cache.request(mods);
		await Bun.sleep(320);
		expect(requests).toBe(3);
		expect(cache.retainedBytes).toBeLessThanOrEqual(8 * 1024 * 1024);
		await Bun.sleep(150);
		expect(requests).toBe(3);
	} finally {
		cache.destroy();
	}
});

test("a missing icon is cached and not retried on every viewport notification", async () => {
	let requests = 0;
	const cache = createLocalModIcons(async (files) => {
		requests++;
		return files.map((file) => ({ ...file, icon: null }));
	});
	try {
		const mods = projects(2);
		cache.sync(mods);
		cache.request(mods.slice(0, 1));
		await Bun.sleep(90);
		cache.request(mods);
		await Bun.sleep(90);
		cache.request(mods.slice(0, 1));
		await Bun.sleep(90);
		expect(requests).toBe(2);
		expect(cache.count).toBe(2);
	} finally {
		cache.destroy();
	}
});
