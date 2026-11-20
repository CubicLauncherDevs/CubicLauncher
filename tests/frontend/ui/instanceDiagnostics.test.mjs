import { beforeEach, expect, test } from "bun:test";
import {
	cacheCrashReports,
	cacheDiagnosticReport,
	clearInstanceDiagnostics,
	isDiagnosticReportFresh,
	readInstanceDiagnostics,
	resetInstanceDiagnosticsCache,
} from "../../../src/lib/state/instanceDiagnostics.ts";

/** @param {number} generatedAt */
function report(generatedAt) {
	return {
		generatedAt,
		checks: [],
		errors: 0,
		warnings: 0,
		launchReady: true,
	};
}

beforeEach(() => resetInstanceDiagnosticsCache());

test("caches the report and the crash report list per instance", () => {
	const now = Date.now();
	expect(readInstanceDiagnostics("a")).toBeUndefined();

	cacheDiagnosticReport("a", report(now));
	cacheCrashReports("a", [{ name: "crash-1.txt", modified: now, size: 12 }]);

	const cached = readInstanceDiagnostics("a");
	expect(cached?.report?.generatedAt).toBe(now);
	expect(cached?.reports).toHaveLength(1);
	expect(readInstanceDiagnostics("b")).toBeUndefined();
});

test("keeps the report when only the crash report list is refreshed", () => {
	const now = Date.now();
	cacheDiagnosticReport("a", report(now));
	cacheCrashReports("a", [{ name: "crash-1.txt", modified: now, size: 12 }]);

	expect(readInstanceDiagnostics("a")?.report?.generatedAt).toBe(now);
});

test("detects fresh and stale diagnostic reports", () => {
	const now = Date.now();
	expect(isDiagnosticReportFresh(null, 60_000, now)).toBe(false);
	expect(isDiagnosticReportFresh(report(now), 60_000, now)).toBe(true);
	expect(isDiagnosticReportFresh(report(now - 60_001), 60_000, now)).toBe(
		false,
	);
});

test("clears a single instance and limits retained entries", () => {
	cacheDiagnosticReport("a", report(1));
	clearInstanceDiagnostics("a");
	expect(readInstanceDiagnostics("a")).toBeUndefined();

	for (let index = 0; index < 17; index++) {
		cacheDiagnosticReport(`instance-${index}`, report(index));
	}
	expect(readInstanceDiagnostics("instance-0")).toBeUndefined();
	expect(readInstanceDiagnostics("instance-16")?.report?.generatedAt).toBe(
		16,
	);
});
