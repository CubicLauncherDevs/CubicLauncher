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

export const MAX_LINES = 2000;
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

export function computeLevel(text: string): string {
	const m = text.match(/\[.*?\/(\w+)\]/);
	if (m) {
		const lv = m[1].toUpperCase();
		if (["FATAL", "SEVERE"].includes(lv)) return "fatal";
		if (lv === "ERROR") return "error";
		if (["WARN", "WARNING"].includes(lv)) return "warn";
		if (
			[
				"INFO",
				"CONFIG",
				"FINE",
				"FINER",
				"FINEST",
				"DEBUG",
				"TRACE",
			].includes(lv)
		)
			return "info";
	}
	const u = text.toUpperCase();
	if (/\b(FATAL|SEVERE)\b/.test(u)) return "fatal";
	if (/\bERROR\b/.test(u)) return "error";
	if (/\bWARN(ING)?\b/.test(u)) return "warn";
	return "message";
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

export function highlightText(text: string, q: string): string {
	if (!q) return escapeHtml(text);
	const needle = q.toLowerCase();
	const parts = text.split(new RegExp(`(${escapeRegExp(q)})`, "gi"));
	return parts
		.map((part) =>
			part.toLowerCase() === needle
				? `<mark>${escapeHtml(part)}</mark>`
				: escapeHtml(part),
		)
		.join("");
}

export function levelColor(level: string): string {
	switch (level) {
		case "trace":
		case "debug":
			return "#888";
		case "info":
			return "#81c784";
		case "message":
			return "#c8c8c8";
		case "warn":
			return "#ffd54f";
		case "error":
			return "#e57373";
		case "fatal":
			return "#ef5350";
		case "launcher":
			return "#82b1ff";
		case "stderr":
			return "#ff8a65";
		default:
			return "#888";
	}
}
