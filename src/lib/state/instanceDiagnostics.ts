import type {
	CrashReportEntry,
	DiagnosticReport,
} from "#lib/api/diagnostics.js";

export interface InstanceDiagnosticsCache {
	report: DiagnosticReport | null;
	reports: CrashReportEntry[];
	reportsLoadedAt: number;
}

const MAX_ENTRIES = 16;
const cache = new Map<string, InstanceDiagnosticsCache>();

function store(id: string, entry: InstanceDiagnosticsCache) {
	cache.delete(id);
	cache.set(id, entry);
	if (cache.size <= MAX_ENTRIES) return;
	const oldest = cache.keys().next().value;
	if (oldest !== undefined) cache.delete(oldest);
}

function entry(id: string): InstanceDiagnosticsCache {
	const current = cache.get(id);
	if (current) return current;
	const created = { report: null, reports: [], reportsLoadedAt: 0 };
	cache.set(id, created);
	return created;
}

export function readInstanceDiagnostics(
	id: string,
): InstanceDiagnosticsCache | undefined {
	return cache.get(id);
}

export function cacheDiagnosticReport(id: string, report: DiagnosticReport) {
	store(id, { ...entry(id), report });
}

export function cacheCrashReports(id: string, reports: CrashReportEntry[]) {
	store(id, { ...entry(id), reports, reportsLoadedAt: Date.now() });
}

export function isDiagnosticReportFresh(
	report: DiagnosticReport | null,
	maxAgeMs: number,
	now = Date.now(),
): boolean {
	return report !== null && now - report.generatedAt <= maxAgeMs;
}

export function clearInstanceDiagnostics(id: string) {
	cache.delete(id);
}

export function resetInstanceDiagnosticsCache() {
	cache.clear();
}
