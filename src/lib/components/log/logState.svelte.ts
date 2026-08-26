import { SvelteSet } from "svelte/reactivity";
import {
	CONSOLE_HISTORY_MAX,
	LEVEL_ORDER,
	SEARCH_DEBOUNCE_MS,
	buildQueryRegex,
	createLogLine,
	type LogLine,
	type RawLogEvent,
} from "./logHelpers";
import type { LogRenderer } from "./LogRenderer";

export type HistoryLogLine = {
	id: number;
	text: string;
	stream: string;
	level: string;
	timestamp: number;
};

export class LogState {
	lines = $state<LogLine[]>([]);
	activeLevels = new SvelteSet<string>(LEVEL_ORDER);
	inputQuery = $state("");
	effectiveQuery = $state("");
	normalizedQuery = $derived(this.effectiveQuery.trim().toLowerCase());
	searchRegex = $derived(buildQueryRegex(this.normalizedQuery));
	currentMatchIndex = $state(0);
	matchCount = $state(0);
	uploading = $state(false);
	privacyTerms = $state<string[]>([]);
	maxLines = $state(CONSOLE_HISTORY_MAX);

	private searchTimer: ReturnType<typeof setTimeout> | null = null;
	renderer?: LogRenderer;

	get totalLines() {
		return this.lines.length;
	}

	setRenderer(renderer: LogRenderer) {
		this.renderer = renderer;
	}

	setMaxLines(limit: number) {
		const clamped = Math.max(100, Math.min(limit, CONSOLE_HISTORY_MAX));
		if (this.maxLines === clamped) return;
		this.maxLines = clamped;
		if (this.lines.length > this.maxLines) {
			this.lines.splice(0, this.lines.length - this.maxLines);
			this.currentMatchIndex = 0;
			this.renderer?.rebuild();
		}
	}

	lineVisible(line: LogLine): boolean {
		if (!this.activeLevels.has(line.displayClass)) return false;
		if (
			this.privacyTerms.length > 0 &&
			this.privacyTerms.some((term) => line.textLower.includes(term))
		) {
			return false;
		}
		if (!this.normalizedQuery) return true;
		return line.textLower.includes(this.normalizedQuery);
	}

	setPrivacyTerms(terms: string[]) {
		const normalized = terms
			.map((t) => t.trim().toLowerCase())
			.filter(Boolean);
		this.privacyTerms = normalized;
	}

	ingestHistory(raw: HistoryLogLine[]) {
		let parsed = raw.map(createLogLine);
		if (parsed.length > this.maxLines) {
			parsed = parsed.slice(parsed.length - this.maxLines);
		}
		this.lines = parsed;
	}

	ingestBatch(raw: RawLogEvent[]) {
		const parsed = raw.map(createLogLine);
		this.lines.push(...parsed);
		if (this.lines.length > this.maxLines) {
			this.lines.splice(0, this.lines.length - this.maxLines);
			this.renderer?.rebuild();
			return;
		}
		this.renderer?.appendLines(parsed);
	}

	searchInput(value: string) {
		this.inputQuery = value;
		this.scheduleSearch();
	}

	private scheduleSearch() {
		if (this.searchTimer) clearTimeout(this.searchTimer);
		this.searchTimer = setTimeout(() => {
			this.searchTimer = null;
			this.effectiveQuery = this.inputQuery;
			this.currentMatchIndex = 0;
			this.renderer?.rebuild();
		}, SEARCH_DEBOUNCE_MS);
	}

	flushSearch() {
		if (this.searchTimer) {
			clearTimeout(this.searchTimer);
			this.searchTimer = null;
		}
		if (this.effectiveQuery !== this.inputQuery) {
			this.effectiveQuery = this.inputQuery;
			this.currentMatchIndex = 0;
			this.renderer?.rebuild();
		}
	}

	resetSearch() {
		if (this.searchTimer) {
			clearTimeout(this.searchTimer);
			this.searchTimer = null;
		}
		this.inputQuery = "";
		this.effectiveQuery = "";
		this.currentMatchIndex = 0;
		this.renderer?.rebuild();
	}

	toggleLevel(level: string) {
		if (this.activeLevels.has(level)) this.activeLevels.delete(level);
		else this.activeLevels.add(level);

		if (this.normalizedQuery) {
			this.currentMatchIndex = 0;
			this.renderer?.rebuild();
		} else {
			this.renderer?.applyLevelVisibility();
		}
	}

	setAllLevels(active: boolean) {
		this.activeLevels.clear();
		if (active) {
			for (const level of LEVEL_ORDER) this.activeLevels.add(level);
		}
		if (this.normalizedQuery) {
			this.currentMatchIndex = 0;
			this.renderer?.rebuild();
		} else {
			this.renderer?.applyLevelVisibility();
		}
	}

	clear() {
		if (this.searchTimer) {
			clearTimeout(this.searchTimer);
			this.searchTimer = null;
		}
		this.lines = [];
		this.inputQuery = "";
		this.effectiveQuery = "";
		this.activeLevels.clear();
		for (const level of LEVEL_ORDER) this.activeLevels.add(level);
		this.currentMatchIndex = 0;
		this.matchCount = 0;
		this.renderer?.clear();
	}
}
