import { invoke } from "@tauri-apps/api/core";

export type DiagnosticSeverity = "error" | "warning";

export type DiagnosticAction =
	| { kind: "reinstallVersion"; version: string }
	| { kind: "installJava"; version: number }
	| { kind: "openMods" }
	| { kind: "openWorlds" };

export interface DiagnosticCheck {
	id: string;
	severity: DiagnosticSeverity;
	params: Record<string, string>;
	details: string[];
	action?: DiagnosticAction;
}

export interface DiagnosticReport {
	generatedAt: number;
	checks: DiagnosticCheck[];
	errors: number;
	warnings: number;
	launchReady: boolean;
}

export type CrashSeverity = "info" | "warning" | "error";

export interface CrashEvidence {
	line: number;
	text: string;
}

export interface CrashFinding {
	code: string;
	severity: CrashSeverity;
	params: Record<string, string>;
	evidence: CrashEvidence[];
	action?: DiagnosticAction;
}

export type CrashSource = "crash_report" | "ring" | "latest_log";

export interface CrashContext {
	description: string | null;
	minecraftVersion: string | null;
	javaVersion: string | null;
	exitCode: number | null;
	loader: string | null;
}

export interface CrashAnalysis {
	source: CrashSource;
	reportName: string | null;
	exitCode: number | null;
	context: CrashContext;
	findings: CrashFinding[];
	truncated: boolean;
}

export interface CrashReportEntry {
	name: string;
	modified: number;
	size: number;
}

const SEVERITY_RANK: Record<DiagnosticSeverity, number> = {
	error: 0,
	warning: 1,
};

export function summarize(report: DiagnosticReport | null): {
	errors: number;
	warnings: number;
	label: "allGood" | "issues" | "empty";
} {
	if (!report) return { errors: 0, warnings: 0, label: "empty" };
	return {
		errors: report.errors,
		warnings: report.warnings,
		label: report.errors + report.warnings === 0 ? "allGood" : "issues",
	};
}

export function sortChecks(checks: DiagnosticCheck[]): DiagnosticCheck[] {
	return [...checks].sort(
		(a, b) =>
			SEVERITY_RANK[a.severity] - SEVERITY_RANK[b.severity] ||
			a.id.localeCompare(b.id),
	);
}

const CRASH_SEVERITY_RANK: Record<CrashSeverity, number> = {
	error: 0,
	warning: 1,
	info: 2,
};

export function sortFindings(findings: CrashFinding[]): CrashFinding[] {
	return [...findings].sort(
		(a, b) =>
			Number(a.code === "unknown") - Number(b.code === "unknown") ||
			CRASH_SEVERITY_RANK[a.severity] - CRASH_SEVERITY_RANK[b.severity],
	);
}

export function formatAnalysis(analysis: CrashAnalysis): string {
	const lines: string[] = [];
	const exit = analysis.exitCode ?? analysis.context.exitCode ?? null;
	if (analysis.reportName) lines.push(`# ${analysis.reportName}`);
	if (analysis.context.description)
		lines.push(`Description: ${analysis.context.description}`);
	if (analysis.context.minecraftVersion)
		lines.push(`Minecraft: ${analysis.context.minecraftVersion}`);
	if (analysis.context.javaVersion)
		lines.push(`Java: ${analysis.context.javaVersion}`);
	if (exit !== null) lines.push(`Exit code: ${exit}`);
	for (const finding of sortFindings(analysis.findings)) {
		lines.push(`- ${finding.code} (${finding.severity})`);
		for (const evidence of finding.evidence) {
			lines.push(`  L${evidence.line}: ${evidence.text}`);
		}
	}
	return lines.join("\n");
}

export function diagnoseInstance(id: string): Promise<DiagnosticReport> {
	return invoke("diagnose_instance", { instanceId: id });
}

export function analyzeInstanceCrash(
	id: string,
	report?: string,
): Promise<CrashAnalysis> {
	return invoke("analyze_instance_crash", {
		instanceId: id,
		report: report ?? null,
	});
}

export function listInstanceCrashReports(
	id: string,
): Promise<CrashReportEntry[]> {
	return invoke("list_instance_crash_reports", { instanceId: id });
}

export function getInstanceLogTail(id: string): Promise<string[]> {
	return invoke("get_instance_log_tail", { instanceId: id });
}
