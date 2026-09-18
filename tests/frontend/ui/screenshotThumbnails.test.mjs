import { expect, test } from "bun:test";
import { createScreenshotThumbnails } from "../../../src/lib/state/screenshotThumbnails.ts";

async function waitFor(condition) {
	for (let i = 0; i < 100 && !condition(); i++) await Bun.sleep(10);
	expect(condition()).toBe(true);
}

test("thumbnails only load the latest viewport and discard offscreen results", async () => {
	const calls = [];
	const releases = [];
	const previews = createScreenshotThumbnails((path) => {
		calls.push(path);
		return new Promise((resolve) => releases.push(resolve));
	});
	try {
		previews.request(["old-1", "old-2", "old-3"]);
		await waitFor(() => calls.length === 1);
		previews.request(["new-1", "new-2"]);
		await Bun.sleep(80);
		expect(calls).toEqual(["old-1"]);
		releases.shift()("obsolete");
		await waitFor(() => calls.length === 2);
		expect(calls).toEqual(["old-1", "new-1"]);
		expect(previews.get("old-1")).toBeUndefined();
		releases.shift()("preview-1");
		await waitFor(() => calls.length === 3);
		expect(previews.get("new-1")).toBe("preview-1");
		previews.request(["new-2"]);
		expect(previews.count).toBe(0);
		releases.shift()("preview-2");
		await waitFor(() => previews.count === 1);
		expect(previews.get("new-2")).toBe("preview-2");
	} finally {
		previews.destroy();
	}
	expect(previews.count).toBe(0);
});

test("reset and destroy ignore pending thumbnails and stop subsequent work", async () => {
	let release;
	let calls = 0;
	const previews = createScreenshotThumbnails(() => {
		calls++;
		return new Promise((resolve) => {
			release = resolve;
		});
	});
	previews.request(["same.png", "unused.png"]);
	await waitFor(() => calls === 1);
	previews.reset();
	previews.request(["same.png"]);
	release("old-instance");
	await waitFor(() => calls === 2);
	expect(previews.get("same.png")).toBeUndefined();
	previews.destroy();
	release("late");
	previews.request(["after-destroy"]);
	await Bun.sleep(100);
	expect(previews.count).toBe(0);
	expect(calls).toBe(2);
});

test("failed thumbnails keep placeholders without a retry loop", async () => {
	let calls = 0;
	const previews = createScreenshotThumbnails(async () => {
		calls++;
		throw Error("missing");
	});
	try {
		previews.request(["missing.png"]);
		await waitFor(() => previews.count === 1);
		previews.request(["missing.png"]);
		await Bun.sleep(100);
		expect(calls).toBe(1);
		expect(previews.get("missing.png")).toBeNull();
	} finally {
		previews.destroy();
	}
});
