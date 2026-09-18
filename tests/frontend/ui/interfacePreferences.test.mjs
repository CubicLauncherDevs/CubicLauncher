import { expect, test } from "bun:test";
import { interfacePreferences } from "../../../src/lib/utils/interfacePreferences.ts";
import { applyInterfaceScale } from "../../../src/lib/api/interfaceAppearance.ts";

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
	expect(interfacePreferences(saved)).toEqual(saved);
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
		await Promise.all([90, 110, 125, 100, 100].map(applyInterfaceScale));
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
