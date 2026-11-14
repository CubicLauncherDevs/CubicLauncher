import { expect, test } from "bun:test";
import {
	formatAnalysis,
	sortChecks,
	sortFindings,
	summarize,
} from "../../../src/lib/api/diagnostics.ts";

/** @typedef {import("../../../src/lib/api/diagnostics.ts").CrashAnalysis} CrashAnalysis */
/** @typedef {import("../../../src/lib/api/diagnostics.ts").DiagnosticCheck} DiagnosticCheck */

/**
 * @param {string} id
 * @param {"error" | "warning"} severity
 * @returns {DiagnosticCheck}
 */
function check(id, severity) {
	return { id, severity, params: {}, details: [] };
}

test("summary counts errors and warnings separately", () => {
	expect(summarize(null).label).toBe("empty");
	expect(
		summarize({
			generatedAt: 0,
			checks: [],
			errors: 0,
			warnings: 0,
			launchReady: true,
		}),
	).toEqual({ errors: 0, warnings: 0, label: "allGood" });
	expect(
		summarize({
			generatedAt: 0,
			checks: [check("a", "error")],
			errors: 1,
			warnings: 2,
			launchReady: false,
		}).label,
	).toBe("issues");
});

test("checks are sorted with errors first and stably by id", () => {
	const sorted = sortChecks([
		check("worlds_metadata_error", "warning"),
		check("version_missing", "error"),
		check("java_missing", "error"),
		check("memory_low", "warning"),
	]);
	expect(sorted.map((entry) => entry.id)).toEqual([
		"java_missing",
		"version_missing",
		"memory_low",
		"worlds_metadata_error",
	]);
});

test("unknown crash findings sort last", () => {
	const sorted = sortFindings([
		{ code: "unknown", severity: "info", params: {}, evidence: [] },
		{
			code: "memory_out_of_memory",
			severity: "error",
			params: {},
			evidence: [],
		},
		{
			code: "network_download",
			severity: "warning",
			params: {},
			evidence: [],
		},
	]);
	expect(sorted.map((finding) => finding.code)).toEqual([
		"memory_out_of_memory",
		"network_download",
		"unknown",
	]);
});

test("copied diagnosis lists context and evidence without secrets", () => {
	/** @type {CrashAnalysis} */
	const analysis = {
		source: "crash_report",
		reportName: "crash-1.txt",
		exitCode: 1,
		context: {
			description: "Unexpected error",
			minecraftVersion: "1.21",
			javaVersion: null,
			exitCode: 1,
			loader: null,
		},
		findings: [
			{
				code: "memory_out_of_memory",
				severity: "error",
				params: {},
				evidence: [
					{ line: 12, text: "OutOfMemoryError: Java heap space" },
				],
			},
		],
		truncated: false,
	};
	const text = formatAnalysis(analysis);
	expect(text).toContain("# crash-1.txt");
	expect(text).toContain("Minecraft: 1.21");
	expect(text).toContain("Exit code: 1");
	expect(text).toContain("L12: OutOfMemoryError: Java heap space");
});
