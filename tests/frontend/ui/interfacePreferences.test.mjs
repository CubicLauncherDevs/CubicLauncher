import { expect, test } from "bun:test";
import {
	interfacePreferences,
	DEFAULT_INTERFACE_PREFERENCES,
} from "../../../src/lib/utils/interfacePreferences.ts";
import {
	applyInterfaceScale,
	applyInterfaceDensity,
} from "../../../src/lib/api/interfaceAppearance.ts";

test("invalid and legacy interface preferences recover without altering the source", () => {
	for (const value of [
		undefined,
		null,
		{},
		{ scale: -1, density: "unknown" },
		{ scale: NaN },
		{ scale: Infinity },
	]) {
		expect(interfacePreferences(value)).toEqual({
			scale: 100,
			density: "theme",
		});
	}
	const saved = Object.freeze({ scale: 125, density: "comfortable" });
	expect(interfacePreferences(saved)).toBe(saved);
	expect(interfacePreferences()).toBe(DEFAULT_INTERFACE_PREFERENCES);
	expect(interfacePreferences({ scale: 90 })).toEqual({
		scale: 90,
		density: "theme",
	});
});

test("native zoom serializes changes, deduplicates successful requests and recovers after failure", async () => {
	const windowDescriptor = Object.getOwnPropertyDescriptor(
		globalThis,
		"window",
	);
	const tauriDescriptor = Object.getOwnPropertyDescriptor(
		globalThis,
		"isTauri",
	);
	const calls = [];
	let rejectNext = false;
	let inFlight = 0;
	let maxInFlight = 0;
	Object.defineProperty(globalThis, "isTauri", {
		configurable: true,
		value: true,
	});
	Object.defineProperty(globalThis, "window", {
		configurable: true,
		value: {
			__TAURI_INTERNALS__: {
				metadata: {
					currentWindow: { label: "main" },
					currentWebview: { label: "main" },
				},
				async invoke(command, payload) {
					calls.push({ command, ...payload });
					maxInFlight = Math.max(maxInFlight, ++inFlight);
					await new Promise((resolve) => setTimeout(resolve, 2));
					inFlight--;
					if (rejectNext) {
						rejectNext = false;
						throw Error("Zoom unavailable");
					}
				},
			},
		},
	});
	try {
		const requests = [90, 110, 125, 100, 100].map(applyInterfaceScale);
		expect(requests[3]).toBe(requests[4]);
		await Promise.all(requests);
		expect(calls.map((call) => call.value)).toEqual([0.9, 1.1, 1.25, 1]);
		expect(maxInFlight).toBe(1);
		expect(
			calls.every(
				(call) =>
					call.command === "plugin:webview|set_webview_zoom" &&
					call.label === "main",
			),
		).toBe(true);
		rejectNext = true;
		await expect(applyInterfaceScale(125)).rejects.toThrow(
			"Zoom unavailable",
		);
		await applyInterfaceScale(125);
		await applyInterfaceScale(999);
		expect(calls.slice(-3).map((call) => call.value)).toEqual([
			1.25, 1.25, 1,
		]);
	} finally {
		if (windowDescriptor)
			Object.defineProperty(globalThis, "window", windowDescriptor);
		else delete globalThis.window;
		if (tauriDescriptor)
			Object.defineProperty(globalThis, "isTauri", tauriDescriptor);
		else delete globalThis.isTauri;
	}
});

test("density updates do not rewrite an unchanged root attribute", () => {
	const descriptor = Object.getOwnPropertyDescriptor(globalThis, "document");
	let attribute = null,
		writes = 0;
	Object.defineProperty(globalThis, "document", {
		configurable: true,
		value: {
			documentElement: {
				getAttribute: () => attribute,
				setAttribute: (_, value) => {
					attribute = value;
					writes++;
				},
				removeAttribute: () => {
					attribute = null;
					writes++;
				},
			},
		},
	});
	try {
		for (let i = 0; i < 100; i++) applyInterfaceDensity("theme");
		expect(writes).toBe(0);
		for (let i = 0; i < 100; i++) applyInterfaceDensity("compact");
		expect(writes).toBe(1);
		applyInterfaceDensity("comfortable");
		applyInterfaceDensity("theme");
		applyInterfaceDensity("theme");
		expect(writes).toBe(3);
		expect(attribute).toBeNull();
	} finally {
		if (descriptor)
			Object.defineProperty(globalThis, "document", descriptor);
		else delete globalThis.document;
	}
});
