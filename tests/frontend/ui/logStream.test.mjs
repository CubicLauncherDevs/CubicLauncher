import { expect, test } from "bun:test";
import { resolve } from "node:path";

async function fixture() {
	const root = resolve(import.meta.dir, "../../..");
	const service = resolve(root, "src/lib/api/logStream.ts");
	const bundle = await Bun.build({
		entrypoints: ["log-entry"],
		target: "bun",
		plugins: [
			{
				name: "log-stream",
				setup(build) {
					build.onResolve(
						{ filter: /^log-(entry|stub)$/ },
						({ path }) => ({ path, namespace: "fixture" }),
					);
					build.onResolve({ filter: /^@tauri-apps\// }, () => ({
						path: "log-stub",
						namespace: "fixture",
					}));
					build.onResolve({ filter: /^\$lib\// }, ({ path }) => ({
						path: Bun.resolveSync(
							path.replace("$lib", resolve(root, "src/lib")),
							root,
						),
					}));
					build.onLoad(
						{ filter: /.*/, namespace: "fixture" },
						({ path }) => ({
							loader: "js",
							resolveDir: root,
							contents:
								path === "log-entry"
									? `export * from ${JSON.stringify(service)}; export * from 'log-stub';`
									: `
export const hooks={listen:async()=>()=>{},invoke:async()=>[]};
export const listen=(...args)=>hooks.listen(...args);
export const invoke=(...args)=>hooks.invoke(...args);
// ${crypto.randomUUID()}
`,
						}),
					);
				},
			},
		],
	});
	expect(bundle.success, bundle.logs.join("\n")).toBe(true);
	return import(
		`data:text/javascript;base64,${Buffer.from(await bundle.outputs[0].text()).toString("base64")}`
	);
}

function deferred() {
	let resolve, reject;
	const promise = new Promise((yes, no) => {
		resolve = yes;
		reject = no;
	});
	return { promise, resolve, reject };
}
const settle = async () => {
	for (let i = 0; i < 8; i++) await Promise.resolve();
};
const line = (id) => ({
	id,
	line: `line ${id}`,
	text: `line ${id}`,
	stream: "stdout",
	level: "info",
	timestamp: id,
});

test("console targets its own window and merges history/live overlap exactly once", async () => {
	const f = await fixture();
	const history = deferred();
	let handler,
		unlistens = 0;
	f.hooks.listen = async (event, callback, options) => {
		expect(event).toBe("instance-log-batch");
		expect(options.target).toEqual({
			kind: "WebviewWindow",
			label: "log-a",
		});
		handler = callback;
		return () => unlistens++;
	};
	f.hooks.invoke = () => history.promise;
	const initial = [],
		batches = [],
		errors = [];
	const stop = f.openLogStream(
		"a",
		100,
		(lines) => initial.push(lines),
		(lines) => batches.push(lines),
		(err) => errors.push(err),
	);
	await settle();
	handler({ payload: { id: "other", lines: [line(999)] } });
	handler({ payload: { id: "a", lines: [line(2), line(4)] } });
	history.resolve([line(1), line(2), line(3)]);
	await settle();
	expect(initial[0].map((l) => l.id)).toEqual([1, 2, 3, 4]);
	handler({ payload: { id: "a", lines: [line(3), line(4), line(5)] } });
	expect(batches.flat().map((l) => l.id)).toEqual([5]);
	stop();
	stop();
	handler({ payload: { id: "a", lines: [line(6)] } });
	expect(batches.flat().map((l) => l.id)).toEqual([5]);
	expect(unlistens).toBe(1);
	expect(errors).toEqual([]);
});

test("closing before listener registration releases the late listener without requesting history", async () => {
	const f = await fixture();
	const registration = deferred();
	let unlistens = 0,
		reads = 0;
	f.hooks.listen = () => registration.promise;
	f.hooks.invoke = () => {
		reads++;
		return [];
	};
	const stop = f.openLogStream(
		"a",
		100,
		() => {},
		() => {},
		() => {},
	);
	stop();
	registration.resolve(() => unlistens++);
	await settle();
	expect(unlistens).toBe(1);
	expect(reads).toBe(0);
});

test("pending history cannot revive a closed console and live buffering stays bounded", async () => {
	const f = await fixture();
	let handler,
		off = 0;
	f.hooks.listen = async (_, cb) => {
		handler = cb;
		return () => off++;
	};
	const history = deferred();
	f.hooks.invoke = () => history.promise;
	const initial = [];
	const stop = f.openLogStream(
		"a",
		100,
		(lines) => initial.push(lines),
		() => {},
		() => {},
	);
	await settle();
	handler({
		payload: {
			id: "a",
			lines: Array.from({ length: 6000 }, (_, i) => line(i)),
		},
	});
	history.resolve([]);
	await settle();
	expect(initial[0]).toHaveLength(100);
	expect(initial[0][0].id).toBe(5900);
	stop();
	const late = deferred();
	f.hooks.invoke = () => late.promise;
	const stopLate = f.openLogStream(
		"a",
		100,
		(lines) => initial.push(lines),
		() => {},
		() => {},
	);
	await settle();
	stopLate();
	late.resolve([line(7000)]);
	await settle();
	expect(initial).toHaveLength(1);
	expect(off).toBe(2);
});

test("a history failure leaves the live stream usable", async () => {
	const f = await fixture();
	const history = deferred();
	let handler;
	f.hooks.listen = async (_, cb) => {
		handler = cb;
		return () => {};
	};
	f.hooks.invoke = () => history.promise;
	const batches = [],
		errors = [];
	const stop = f.openLogStream(
		"a",
		100,
		() => {},
		(lines) => batches.push(lines),
		(error) => errors.push(error),
	);
	await settle();
	handler({ payload: { id: "a", lines: [line(1)] } });
	history.reject(Error("history unavailable"));
	await settle();
	handler({ payload: { id: "a", lines: [line(2)] } });
	expect(batches.flat().map((l) => l.id)).toEqual([1, 2]);
	expect(errors).toHaveLength(1);
	stop();
});

test("preview cleanup uses mount-specific tokens across delayed acknowledgements", async () => {
	const f = await fixture();
	const pending = [],
		listeners = [],
		subscriptions = new Map();
	let off = 0;
	f.hooks.listen = async (_, cb, options) => {
		expect(options.target).toEqual({
			kind: "WebviewWindow",
			label: "main",
		});
		listeners.push(cb);
		return () => off++;
	};
	f.hooks.invoke = (_, args) => {
		if (args.instanceId === null) {
			subscriptions.delete(args.subscriptionId);
			return Promise.resolve();
		}
		subscriptions.set(args.subscriptionId, args.instanceId);
		const ack = deferred();
		pending.push(ack);
		return ack.promise;
	};
	const received = [];
	const stopA = f.subscribeLogPreview("a", (l) => received.push(l));
	await settle();
	stopA();
	const stopB = f.subscribeLogPreview("b", (l) => received.push(l));
	await settle();
	pending[1].resolve();
	pending[0].resolve();
	await settle();
	expect([...subscriptions.values()]).toEqual(["b"]);
	listeners[0]({ payload: { id: "a", line: "old" } });
	listeners[1]({ payload: { id: "a", line: "wrong instance" } });
	listeners[1]({ payload: { id: "b", line: "latest" } });
	expect(received).toEqual(["latest"]);
	stopB();
	await settle();
	expect(subscriptions.size).toBe(0);
	expect(off).toBe(2);
});
