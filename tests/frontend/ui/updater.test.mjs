import { expect, test } from "bun:test";
import {
	createUpdateController,
	createUpdaterState,
} from "../../../src/lib/api/updateController.ts";

function deferred() {
	let resolve, reject;
	const promise = new Promise((yes, no) => {
		resolve = yes;
		reject = no;
	});
	return { promise, resolve, reject };
}

function fixture() {
	const state = createUpdaterState();
	const calls = { check: 0, download: 0, install: 0, restart: 0, close: 0 };
	const handle = {
		version: "36.0.0-beta.1",
		body: "## Changes\n- Shared update modal",
		download: async () => {
			calls.download++;
		},
		install: async () => {
			calls.install++;
		},
		close: async () => {
			calls.close++;
		},
	};
	const dependencies = {
		check: async () => {
			calls.check++;
			return handle;
		},
		relaunch: async () => {
			calls.restart++;
		},
	};
	return {
		state,
		calls,
		handle,
		dependencies,
		updater: createUpdateController(state, dependencies),
	};
}

test("startup discovers and opens an update once, without downloading or restarting", async () => {
	const { state, updater, calls } = fixture();
	await updater.checkForUpdates(true);
	expect(state.status).toBe("available");
	expect(state.open).toBe(true);
	expect(state.update.version).toBe("36.0.0-beta.1");
	expect(calls).toEqual({
		check: 1,
		download: 0,
		install: 0,
		restart: 0,
		close: 0,
	});
	updater.close();
	await updater.checkForUpdates(true);
	expect(state.open).toBe(false);
	updater.open();
	expect(state.open).toBe(true);
	expect(calls.check).toBe(1);
});

test("opening settings during an automatic check shares the request and respects dismissal", async () => {
	const { state, updater, calls, handle, dependencies } = fixture();
	const result = deferred();
	dependencies.check = async () => {
		calls.check++;
		return result.promise;
	};
	const pending = updater.checkForUpdates(true);
	updater.open();
	updater.open();
	expect(state.status).toBe("checking");
	expect(calls.check).toBe(1);
	updater.close();
	result.resolve(handle);
	await pending;
	expect(state.open).toBe(false);
	expect(state.status).toBe("available");
	await updater.checkForUpdates(true);
	expect(state.open).toBe(false);
});

test("an unavailable update stays silent automatically and is distinct from a failed check", async () => {
	const { state, updater, dependencies } = fixture();
	dependencies.check = async () => null;
	await updater.checkForUpdates(true);
	expect(state.status).toBe("updated");
	expect(state.lastChecked).not.toBeNull();
	expect(state.open).toBe(false);
	dependencies.check = async () => {
		throw new Error("Offline");
	};
	await updater.checkForUpdates();
	expect(state.status).toBe("error");
	expect(state.failedOperation).toBe("check");
	updater.open();
	expect(state.error).toBe("Offline");
	dependencies.check = async () => null;
	await updater.retry();
	expect(state.status).toBe("updated");
	expect(state.error).toBeNull();
});

test("closing and reopening retains progress, suppresses duplicate downloads, and waits for verification", async () => {
	const { state, updater, calls, handle } = fixture();
	const verification = deferred();
	let progress;
	handle.download = async (callback) => {
		calls.download++;
		progress = callback;
		await verification.promise;
	};
	await updater.checkForUpdates(true);
	const downloading = updater.download();
	progress({ event: "Started", data: { contentLength: 1000 } });
	progress({ event: "Progress", data: { chunkLength: 450 } });
	updater.close();
	expect(state.progress).toBe(45);
	updater.open();
	await updater.download();
	await updater.checkForUpdates(true);
	expect(calls.check).toBe(1);
	expect(calls.download).toBe(1);
	expect(state.downloadedBytes).toBe(450);
	progress({ event: "Progress", data: { chunkLength: 550 } });
	progress({ event: "Finished" });
	expect(state.status).toBe("downloading");
	await updater.install();
	expect(calls.install).toBe(0);
	verification.resolve();
	await downloading;
	expect(state.status).toBe("ready");
	updater.close();
	updater.open();
	expect(state.status).toBe("ready");
	expect(calls.restart).toBe(0);
	await updater.install();
	expect(calls.install).toBe(1);
	expect(calls.restart).toBe(1);
	expect(calls.close).toBe(1);
});

test("unknown size is indeterminate and a verification failure never becomes installable", async () => {
	const { state, updater, calls, handle } = fixture();
	handle.download = async (progress) => {
		progress({ event: "Started", data: {} });
		progress({ event: "Progress", data: { chunkLength: 1024 } });
		expect(state.progress).toBeNull();
		expect(state.downloadedBytes).toBe(1024);
		progress({ event: "Finished" });
		throw new Error("Invalid signature");
	};
	await updater.checkForUpdates(true);
	await updater.download();
	expect(state.status).toBe("error");
	expect(state.failedOperation).toBe("download");
	await updater.install();
	expect(calls.install).toBe(0);
	handle.download = async () => {};
	await updater.retry();
	expect(state.status).toBe("ready");
	expect(state.error).toBeNull();
});

test("installation and restart are single-flight and retrying restart does not reinstall", async () => {
	const { state, updater, calls, dependencies, handle } = fixture();
	const installed = deferred();
	handle.install = async () => {
		calls.install++;
		await installed.promise;
	};
	dependencies.relaunch = async () => {
		calls.restart++;
		throw new Error("Restart failed");
	};
	await updater.checkForUpdates(true);
	await updater.download();
	const installing = updater.install();
	await updater.install();
	await updater.download();
	updater.close();
	updater.open();
	expect(state.status).toBe("installing");
	expect(calls.install).toBe(1);
	installed.resolve();
	await installing;
	expect(state.status).toBe("error");
	expect(state.failedOperation).toBe("restart");
	dependencies.relaunch = async () => {
		calls.restart++;
	};
	await updater.retry();
	expect(calls.restart).toBe(2);
	expect(calls.install).toBe(1);
	expect(state.status).toBe("restarting");
});

test("failed installation retries with a new download before allowing another install", async () => {
	const { state, updater, calls, handle } = fixture();
	handle.install = async () => {
		calls.install++;
		throw new Error("Installer failed");
	};
	await updater.checkForUpdates(true);
	await updater.download();
	await updater.install();
	expect(state.failedOperation).toBe("install");
	await updater.retry();
	expect(calls.download).toBe(2);
	expect(calls.install).toBe(1);
	expect(state.status).toBe("ready");
});
