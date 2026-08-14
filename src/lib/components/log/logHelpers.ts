export interface LogLine {
	id: number;
	text: string;
	textLower: string;
	displayClass: string;
	stream: string;
	timestamp: number;
	level: string;
}

export type RawLogEvent = {
	id: number;
	line: string;
	stream: string;
	level: string;
	timestamp: number;
};

export const MAX_LINES = 3000;
export const SCROLL_THRESHOLD = 60;
export const SEARCH_DEBOUNCE_MS = 180;

export const LEVEL_ORDER = [
	"trace",
	"debug",
	"info",
	"message",
	"warn",
	"error",
	"fatal",
	"launcher",
	"stderr",
	"unknown",
];

const LEVELS = new Set(LEVEL_ORDER);

export function normalizeLevel(level: string | undefined): string {
	if (!level) return "message";
	const l = level.toLowerCase();
	return LEVELS.has(l) ? l : "message";
}

export function computeDisplayClass(level: string, stream: string): string {
	let c = normalizeLevel(level);
	if (stream === "stderr" && c === "message") c = "stderr";
	return c;
}

export function createLogLine(raw: {
	id: number;
	text?: string;
	line?: string;
	stream: string;
	level?: string;
	timestamp: number;
}): LogLine {
	const text = raw.text ?? raw.line ?? "";
	const level = normalizeLevel(raw.level);
	return {
		id: raw.id,
		text,
		textLower: text.toLowerCase(),
		displayClass: computeDisplayClass(level, raw.stream ?? ""),
		stream: raw.stream ?? "",
		timestamp: raw.timestamp,
		level,
	};
}

export const timeFmt = new Intl.DateTimeFormat("en-US", {
	hour: "2-digit",
	minute: "2-digit",
	second: "2-digit",
	hour12: false,
});

export function escapeHtml(text: string): string {
	return text
		.replace(/&/g, "&amp;")
		.replace(/</g, "&lt;")
		.replace(/>/g, "&gt;");
}

export function escapeRegExp(text: string): string {
	return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function buildQueryRegex(q: string): RegExp | null {
	if (!q) return null;
	return new RegExp(`(${escapeRegExp(q)})`, "gi");
}

export function highlightText(
	text: string,
	q: string,
	regex?: RegExp | null,
): string {
	if (!q) return escapeHtml(text);
	const needle = q.toLowerCase();
	const re = regex ?? new RegExp(`(${escapeRegExp(q)})`, "gi");
	const parts = text.split(re);
	return parts
		.map((part) =>
			part.toLowerCase() === needle
				? `<mark>${escapeHtml(part)}</mark>`
				: escapeHtml(part),
		)
		.join("");
}

const LEVEL_COLORS: Record<string, string> = {
	trace: "#888",
	debug: "#888",
	info: "#81c784",
	message: "#c8c8c8",
	warn: "#ffd54f",
	error: "#e57373",
	fatal: "#ef5350",
	launcher: "#82b1ff",
	stderr: "#ff8a65",
	unknown: "#888",
};

export function levelColor(level: string): string {
	return LEVEL_COLORS[level] ?? "#888";
}
