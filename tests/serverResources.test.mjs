import { expect, mock, test } from "bun:test";
import {
	ResourceCache,
	ServerResources,
} from "../src/lib/state/serverResources.ts";

const row = (index, overrides = {}) => ({
	index,
	name: `Server ${index}`,
	address: `server-${index}.example`,
	resourcePolicy: "prompt",
	hasIcon: true,
	...overrides,
});
const online = (overrides = {}) => ({
	online: true,
	players: 3,
	maxPlayers: 20,
	ping: 12,
	motd: "Hello",
	version: "1.21",
	icon: null,
	...overrides,
});
const settle = () => Bun.sleep(0);

function fixture(count = 1000) {
	let now = 0;
	const calls = [];
	const iconReads = [];
	const views = [];
	const errors = mock();
	const resources = new ServerResources(
		"instance-A",
		(view) => views.push(view),
		errors,
		{
			ping: (id, targets, result, fail) => {
				const cancel = mock();
				calls.push({ id, targets, result, fail, cancel });
				return cancel;
			},
			icons: (id, revision, indices) =>
				new Promise((resolve, reject) =>
					iconReads.push({ id, revision, indices, resolve, reject }),
				),
		},
		() => now,
	);
	resources.setList({
		revision: "one",
		servers: Array.from({ length: count }, (_, i) => row(i)),
	});
	return {
		resources,
		calls,
		iconReads,
		views,
		errors,
		clock: (value) => {
			now = value;
		},
	};
}

function complete(
	f,
	callIndex = f.calls.length - 1,
	iconIndex = f.iconReads.length - 1,
) {
	const call = f.calls[callIndex];
	for (const target of call.targets)
		call.result({ indices: [target.index], status: online(), done: false });
	call.result({ indices: [], status: null, done: true });
	const icons = f.iconReads[iconIndex];
	icons?.resolve(
		icons.indices.map((index) => ({
			index,
			icon: `data:image/png;base64,icon-${index}`,
		})),
	);
}

test("opening a 1000-entry list requests just the visible page and selected row", async () => {
	const f = fixture();
	f.resources.show([...f.resources.rows.slice(0, 50), f.resources.rows[999]]);
	expect(f.calls[0].targets).toHaveLength(51);
	expect(f.calls[0].targets.at(-1)).toEqual({
		index: 999,
		address: "server-999.example",
	});
	expect(f.iconReads[0].indices).toEqual([
		...Array.from({ length: 50 }, (_, i) => i),
		999,
	]);
	complete(f);
	await settle();
	expect(Object.keys(f.views.at(-1).statuses)).toHaveLength(51);
	expect(f.views.at(-1).querying).toBe(false);
	f.resources.show([...f.resources.rows.slice(0, 50), f.resources.rows[999]]);
	expect(f.calls).toHaveLength(1);
	expect(f.iconReads).toHaveLength(1);
	f.resources.dispose();
});

test("cached pages avoid new queries; old pages are evicted instead of accumulating", async () => {
	const f = fixture();
	for (let page = 0; page < 3; page++) {
		f.resources.show(f.resources.rows.slice(page * 50, page * 50 + 50));
		complete(f);
		await settle();
		expect(Object.keys(f.views.at(-1).statuses)).toHaveLength(50);
	}
	f.resources.show(f.resources.rows.slice(50, 100));
	expect(f.calls).toHaveLength(3);
	expect(f.iconReads).toHaveLength(3);
	f.resources.show(f.resources.rows.slice(0, 50));
	expect(f.calls).toHaveLength(4);
	expect(f.calls[3].targets).toHaveLength(50);
	f.resources.dispose();
});

test("rename, reorder and resource-policy edits retain status, icons and row keys", async () => {
	const f = fixture(2);
	f.resources.show(f.resources.rows);
	complete(f);
	await settle();
	const [a, b] = f.resources.rows;
	let servers = [
		{ ...b, index: 0 },
		{ ...a, index: 1 },
	];
	f.resources.setList(
		{ revision: "two", servers },
		{ type: "move", index: 0, direction: "down" },
	);
	f.resources.show(f.resources.rows);
	expect(f.resources.rows.map((r) => r.key)).toEqual([b.key, a.key]);
	expect(f.views.at(-1).icons[0]).toContain("icon-1");
	servers = [
		{ ...servers[0], name: "Renamed", resourcePolicy: "disabled" },
		servers[1],
	];
	f.resources.setList(
		{ revision: "three", servers },
		{ type: "edit", index: 0, server: servers[0] },
	);
	f.resources.show(f.resources.rows);
	expect(f.resources.rows[0].key).toBe(b.key);
	expect(f.calls).toHaveLength(1);
	expect(f.iconReads).toHaveLength(1);
	f.resources.dispose();
});

test("address changes request only the changed server and cannot reuse its old icon", async () => {
	const f = fixture(2);
	f.resources.show(f.resources.rows);
	complete(f);
	await settle();
	const servers = [
		row(0, { address: "new.example", hasIcon: false }),
		row(1),
	];
	f.resources.setList(
		{ revision: "two", servers },
		{ type: "edit", index: 0, server: servers[0] },
	);
	f.resources.show(f.resources.rows);
	expect(f.calls[1].targets).toEqual([{ index: 0, address: "new.example" }]);
	expect(f.views.at(-1).icons[0]).toBeUndefined();
	expect(f.iconReads).toHaveLength(1);
	f.resources.dispose();
});

test("expiration, manual refresh and external changes invalidate the right resources", async () => {
	const f = fixture(1);
	f.resources.show(f.resources.rows);
	complete(f);
	await settle();
	f.resources.pause();
	f.clock(30_001);
	f.resources.show(f.resources.rows);
	expect(f.calls).toHaveLength(2);
	expect(f.iconReads).toHaveLength(1);
	f.calls[1].result({ indices: [0], status: online(), done: true });
	f.resources.setList({ revision: "external", servers: [row(0)] });
	f.resources.show(f.resources.rows);
	expect(f.iconReads).toHaveLength(2);
	expect(f.calls).toHaveLength(2);
	f.resources.refresh();
	f.resources.show(f.resources.rows);
	expect(f.calls).toHaveLength(3);
	f.iconReads[1].resolve([]);
	await settle();
	expect(f.iconReads).toHaveLength(3);
	f.resources.dispose();
});

test("stale icon and ping responses cannot repopulate another page or disposed state", async () => {
	const f = fixture(100);
	f.resources.show(f.resources.rows.slice(0, 50));
	f.resources.show(f.resources.rows.slice(50));
	expect(f.calls[0].cancel).toHaveBeenCalledTimes(1);
	complete(f, 0, 0);
	await settle();
	expect(f.views.at(-1).statuses[0]).toBeUndefined();
	expect(f.views.at(-1).icons[0]).toBeUndefined();
	f.calls[0].fail("stale failure");
	expect(f.errors).not.toHaveBeenCalled();
	f.resources.dispose();
	const count = f.views.length;
	complete(f, 1, 1);
	await settle();
	expect(f.views).toHaveLength(count);
	expect(f.resources.rows).toHaveLength(0);
	expect(f.views.at(-1)).toEqual({
		statuses: {},
		icons: {},
		querying: false,
	});
});

test("one grouped response populates duplicate addresses without cloning status", async () => {
	const f = fixture(2);
	f.resources.setList({
		revision: "dupes",
		servers: [
			row(0, { address: "same.example" }),
			row(1, { address: "same.example:25565" }),
		],
	});
	f.resources.show(f.resources.rows);
	const status = online({ icon: "data:image/png;base64,new-icon" });
	f.calls[0].result({ indices: [0, 1], status, done: true });
	f.iconReads[0].resolve([
		{ index: 0, icon: "old-icon" },
		{ index: 1, icon: "old-icon" },
	]);
	await settle();
	expect(f.views.at(-1).statuses[0]).toBe(f.views.at(-1).statuses[1]);
	expect(f.views.at(-1).icons).toEqual({});
	f.resources.pause();
	f.resources.show(f.resources.rows);
	expect(f.calls).toHaveLength(1);
	expect(f.iconReads).toHaveLength(1);
	f.resources.dispose();
});

test("cache enforces byte and entry budgets, expiry, LRU order and cleanup", () => {
	let now = 0;
	const cache = new ResourceCache(() => now, 2, 100);
	cache.set("a", "A", 40, 10);
	cache.set("b", "B", 40, 20);
	expect(cache.get("a")).toBe("A");
	cache.set("c", "C", 40, 20);
	expect(cache.get("b")).toBeUndefined();
	cache.set("too-big", "X", 101, 20);
	expect(cache.get("too-big")).toBeUndefined();
	expect(cache.bytes).toBe(80);
	now = 10;
	expect(cache.get("a")).toBeUndefined();
	expect(cache.bytes).toBe(40);
	cache.set("c", "replacement", 80, 20);
	expect(cache.bytes).toBe(80);
	cache.clear();
	expect(cache.bytes).toBe(0);
	expect(cache.size).toBe(0);
});

test("slow icon reads are serialized and intermediate pages never hit disk", async () => {
	const f = fixture(150);
	f.resources.show(f.resources.rows.slice(0, 50));
	f.resources.show(f.resources.rows.slice(50, 100));
	f.resources.show(f.resources.rows.slice(100, 150));
	expect(f.iconReads).toHaveLength(1);
	f.iconReads[0].resolve([]);
	await settle();
	expect(f.iconReads).toHaveLength(2);
	expect(f.iconReads[1].indices).toEqual(
		Array.from({ length: 50 }, (_, i) => i + 100),
	);
	f.resources.pause();
	let ready = false;
	const pending = f.resources.waitForIcons().then(() => {
		ready = true;
	});
	await settle();
	expect(ready).toBe(false);
	f.iconReads[1].resolve([]);
	await pending;
	expect(ready).toBe(true);
	expect(f.views.at(-1).icons).toEqual({});
	f.resources.dispose();
});
