import { expect, test } from "bun:test";
import { createAutoUpdateScheduler } from "../../../src/lib/api/autoUpdateScheduler.ts";
import {
	createUpdateController,
	createUpdaterState,
} from "../../../src/lib/api/updateController.ts";

function fixture() {
	let time = 0;
	let nextId = 0;
	const timers = new Map();
	const state = createUpdaterState();
	const calls = { check: 0, notify: 0 };
	const environment = {
		visible: true,
		offline: false,
		enabled: true,
		wait: null,
	};
	const updater = createUpdateController(state, {
		now: () => time,
		check: async () => {
			calls.check++;
			if (environment.wait) await environment.wait;
			if (environment.offline) throw Error("Offline");
			return null;
		},
		relaunch: async () => {},
	});
	const scheduler = createAutoUpdateScheduler({
		check: () => updater.checkForUpdates(true),
		state: () => state,
		canCheck: () => environment.visible && environment.enabled,
		notifyAvailable: () => calls.notify++,
		now: () => time,
		setTimer: (callback, delay) => {
			const id = ++nextId;
			timers.set(id, { at: time + delay, callback });
			return id;
		},
		clearTimer: (id) => timers.delete(id),
	});
	async function settle() {
		for (let i = 0; i < 12; i++) await Promise.resolve();
	}
	async function advance(ms) {
		const target = time + ms;
		while (true) {
			const next = [...timers.entries()]
				.filter(([, item]) => item.at <= target)
				.sort((a, b) => a[1].at - b[1].at)[0];
			if (!next) break;
			time = next[1].at;
			timers.delete(next[0]);
			next[1].callback();
			await settle();
		}
		time = target;
		await settle();
	}
	return { state, calls, environment, scheduler, timers, advance, settle };
}

test("startup and periodic checks use one timer and stop cleanly", async () => {
	const f = fixture();
	await f.advance(1999);
	expect(f.calls.check).toBe(0);
	await f.advance(1);
	expect(f.calls.check).toBe(1);
	await f.advance(6 * 60 * 60_000);
	expect(f.calls.check).toBe(2);
	expect(f.timers.size).toBe(1);
	f.scheduler.stop();
	await f.advance(24 * 60 * 60_000);
	expect(f.calls.check).toBe(2);
	expect(f.timers.size).toBe(0);
});

test("an offline startup retries with backoff and resumes regular checks after recovery", async () => {
	const f = fixture();
	f.environment.offline = true;
	await f.advance(2000);
	expect(f.calls.check).toBe(1);
	await f.advance(60_000);
	expect(f.calls.check).toBe(2);
	await f.advance(5 * 60_000);
	expect(f.calls.check).toBe(3);
	f.environment.offline = false;
	await f.advance(15 * 60_000);
	expect(f.calls.check).toBe(4);
	expect(f.state.status).toBe("updated");
	await f.advance(6 * 60 * 60_000);
	expect(f.calls.check).toBe(5);
	f.scheduler.stop();
});

test("reconnecting retries a failed check without an online-event request storm", async () => {
	const f = fixture();
	f.environment.offline = true;
	await f.advance(2000);
	f.environment.offline = false;
	for (let i = 0; i < 20; i++) f.scheduler.wake(true);
	await f.advance(0);
	expect(f.calls.check).toBe(1);
	await f.advance(30_000);
	for (let i = 0; i < 20; i++) f.scheduler.wake(true);
	await f.advance(0);
	expect(f.calls.check).toBe(2);
	expect(f.timers.size).toBe(1);
	f.scheduler.stop();
});

test("hidden windows defer networking and focus checks again after a reasonable cooldown", async () => {
	const f = fixture();
	f.environment.visible = false;
	await f.advance(2000);
	expect(f.calls.check).toBe(0);
	f.environment.visible = true;
	f.scheduler.wake();
	await f.advance(0);
	expect(f.calls.check).toBe(1);
	f.scheduler.wake();
	await f.advance(0);
	expect(f.calls.check).toBe(1);
	await f.advance(5 * 60_000);
	f.scheduler.wake();
	await f.advance(0);
	expect(f.calls.check).toBe(2);
	f.scheduler.stop();
});

test("downloads and install errors are not replaced by automatic checks", async () => {
	const f = fixture();
	f.state.status = "downloading";
	await f.advance(2000);
	expect(f.calls.check).toBe(0);
	f.state.status = "error";
	f.state.failedOperation = "install";
	await f.advance(60_000);
	expect(f.calls.check).toBe(0);
	f.scheduler.stop();
});

test("stopping during an in-flight check never rearms a timer", async () => {
	const f = fixture();
	let finish;
	f.environment.wait = new Promise((resolve) => {
		finish = resolve;
	});
	await f.advance(2000);
	expect(f.calls.check).toBe(1);
	f.scheduler.stop();
	finish();
	await f.settle();
	expect(f.timers.size).toBe(0);
	f.scheduler.wake(true);
	await f.advance(60_000);
	expect(f.calls.check).toBe(1);
});
