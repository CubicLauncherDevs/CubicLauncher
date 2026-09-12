import { afterEach, beforeEach, expect, mock, test } from "bun:test";
import { getInstanceServers, serverAction, pingInstanceServers } from "../src/lib/api/servers.ts";
import { resolve } from "node:path";

let originalWindow, invoke, starts;
const settle = () => Bun.sleep(0);

beforeEach(() => {
	originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
	starts = [];
	invoke = mock((command, args) => {
		if (command !== "start_instance_server_ping") return Promise.resolve({ revision: "updated", servers: [] });
		return new Promise((resolve, reject) => starts.push({ args, resolve, reject }));
	});
	Object.defineProperty(globalThis, "window", {
		configurable: true,
		value: { __TAURI_INTERNALS__: { invoke, transformCallback: () => 1, unregisterCallback: () => {} } },
	});
});

afterEach(() => {
	if (originalWindow) Object.defineProperty(globalThis, "window", originalWindow);
	else delete globalThis.window;
});

test("cancels native work even when the component closes before start returns", async () => {
	const status = mock();
	const error = mock();
	const cancel = pingInstanceServers("instance-A", "revision-A", status, error);
	cancel();
	cancel();
	starts[0].args.onStatus.onmessage({ index: 0, status: { online: true }, done: false });
	starts[0].resolve("native-A");
	await settle();
	expect(status).not.toHaveBeenCalled();
	expect(error).not.toHaveBeenCalled();
	expect(invoke.mock.calls.filter(([command]) => command === "cancel_instance_server_ping")).toEqual([
		["cancel_instance_server_ping", { requestId: "native-A" }, undefined],
	]);
});

test("switching instances rejects stale results and cancels only their native request", async () => {
	const seen = [];
	const cancelA = pingInstanceServers("A", "rev-A", (event) => seen.push(["A", event]), mock());
	cancelA();
	const cancelB = pingInstanceServers("B", "rev-B", (event) => seen.push(["B", event]), mock());
	starts[1].resolve("native-B");
	await settle();
	starts[0].resolve("native-A");
	const result = { index: 0, status: { online: true, players: 8 }, done: false };
	starts[0].args.onStatus.onmessage(result);
	starts[1].args.onStatus.onmessage(result);
	await settle();
	expect(seen).toEqual([["B", result]]);
	expect(invoke.mock.calls.filter(([command]) => command === "cancel_instance_server_ping").map(([, args]) => args.requestId)).toEqual(["native-A"]);
	cancelB();
	await settle();
});

test("reports completion and read conflicts, but ignores failures after disposal", async () => {
	const status = mock();
	const error = mock();
	const cancel = pingInstanceServers("A", "revision", status, error);
	const done = { index: null, status: null, done: true };
	starts[0].args.onStatus.onmessage(done);
	expect(status).toHaveBeenCalledWith(done);
	starts[0].reject("SERVERS_CONFLICT");
	await settle();
	expect(error).toHaveBeenCalledWith("SERVERS_CONFLICT");
	cancel();
	const cancelOther = pingInstanceServers("B", "revision", mock(), error);
	cancelOther();
	starts[1].reject("cancelled read");
	await settle();
	expect(error).toHaveBeenCalledTimes(1);
});

test("edits and queries carry the instance and revision, including duplicate addresses", async () => {
	await getInstanceServers("A");
	const action = { type: "edit", index: 3, server: { name: "Second entry", address: "localhost", resourcePolicy: "disabled" } };
	await serverAction("A", "original-revision", action);
	expect(invoke.mock.calls[0]).toEqual(["get_instance_servers", { instanceId: "A" }, undefined]);
	expect(invoke.mock.calls[1]).toEqual(["instance_server_action", { instanceId: "A", revision: "original-revision", action }, undefined]);
});

test("server target survives Java installation and a normal launch clears it", () => {
	const result = Bun.spawnSync([process.execPath, "--conditions=browser", resolve(import.meta.dir, "fixtures/serverLaunch.mjs")]);
	expect(result.stderr.toString()).toBe("");
	expect(result.exitCode).toBe(0);
});
