import { expect, test } from "bun:test";
import { createSaveQueue } from "../../../src/lib/utils/saveQueue.ts";
import { patchPreferences } from "../../../src/lib/utils/preferencePatch.ts";

function deferred() {
	let resolve, reject;
	const promise = new Promise((yes, no) => {
		resolve = yes;
		reject = no;
	});
	return { promise, resolve, reject };
}

test("a 100-change burst saves the latest state once and flush survives panel teardown", async () => {
	let current = 0;
	const writes = [];
	const queue = createSaveQueue(async () => {
		writes.push(current);
	});
	const requests = [];
	for (let i = 1; i <= 100; i++) {
		current = i;
		requests.push(queue.request(120));
	}
	expect(queue.busy).toBe(true);
	expect(writes).toEqual([]);
	expect(new Set(requests).size).toBe(1);
	await queue.flush();
	await Promise.all(requests);
	expect(writes).toEqual([100]);
	expect(queue.busy).toBe(false);
	await queue.flush();
	expect(writes).toEqual([100]);
});

test("writes never overlap and a burst during a write keeps only one trailing snapshot", async () => {
	let current = 1;
	let active = 0,
		maxActive = 0;
	const gate = deferred();
	const writes = [];
	const queue = createSaveQueue(async () => {
		maxActive = Math.max(maxActive, ++active);
		writes.push(current);
		if (writes.length === 1) await gate.promise;
		active--;
	});
	const first = queue.request();
	await Promise.resolve();
	const requests = [];
	for (let i = 2; i <= 101; i++) {
		current = i;
		requests.push(queue.request(120));
	}
	const closing = queue.flush();
	expect(writes).toEqual([1]);
	gate.resolve();
	await Promise.all([first, closing, ...requests]);
	expect(writes).toEqual([1, 101]);
	expect(maxActive).toBe(1);
	expect(queue.busy).toBe(false);
});

test("a failed batch rejects its callers without blocking a pending change or a retry", async () => {
	const gate = deferred();
	let calls = 0;
	const queue = createSaveQueue(async () => {
		if (++calls === 1) await gate.promise;
	});
	const first = queue.request();
	const failed = first.catch((error) => error);
	await Promise.resolve();
	const second = queue.request(120);
	gate.reject(Error("disk unavailable"));
	expect((await failed).message).toBe("disk unavailable");
	await second;
	await queue.request();
	expect(calls).toBe(3);
	expect(queue.busy).toBe(false);
});

test("the batching timer persists changes without waiting for panel teardown", async () => {
	let writes = 0;
	const queue = createSaveQueue(async () => {
		writes++;
	});
	await queue.request(5);
	expect(writes).toBe(1);
	expect(queue.busy).toBe(false);
});

test("preference patches leave unchanged fields and object identity intact", () => {
	const assignments = [];
	const original = { scale: 100, density: "theme" };
	const preferences = new Proxy(original, {
		set(target, key, value) {
			assignments.push(key);
			target[key] = value;
			return true;
		},
	});
	expect(
		patchPreferences(preferences, { scale: 100, density: "theme" }),
	).toBe(false);
	expect(assignments).toEqual([]);
	expect(
		patchPreferences(preferences, { scale: 100, density: "compact" }),
	).toBe(true);
	expect(assignments).toEqual(["density"]);
	expect(original).toEqual({ scale: 100, density: "compact" });
});
