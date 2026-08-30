import {
	SCROLL_THRESHOLD,
	highlightText,
	timeFmt,
	type LogLine,
} from "./logHelpers";
import type { LogState } from "./logState.svelte";
import { launcherStore } from "$lib/state/state.svelte";

export class LogRenderer {
	private viewport?: HTMLDivElement;
	private pool: HTMLDivElement[] = [];
	private readonly maxPoolSize = 500;
	private scrollTicking = false;

	constructor(private state: LogState) {}

	attach(viewport: HTMLDivElement) {
		this.viewport = viewport;
	}

	detach() {
		this.viewport = undefined;
		this.pool.length = 0;
	}

	private getLinesContainer(): Element | null {
		return this.viewport?.querySelector(".log-lines") ?? null;
	}

	private acquireNode(): HTMLDivElement {
		return this.pool.pop() ?? document.createElement("div");
	}

	private poolNode(node: HTMLDivElement) {
		if (this.pool.length < this.maxPoolSize) {
			node.classList.remove("new", "search-active");
			this.pool.push(node);
		}
	}

	private clearContainer(container: Element) {
		let child = container.firstChild;
		while (child) {
			const next = child.nextSibling;
			this.poolNode(container.removeChild(child) as HTMLDivElement);
			child = next;
		}
	}

	private shouldReduceLogAnimations(): boolean {
		return launcherStore.settings.reduce_log_animations;
	}

	private initLineNode(
		div: HTMLDivElement,
		line: LogLine,
		index: number,
		animate = false,
	) {
		div.className = `log-line ${line.displayClass}${animate ? " new" : ""}`;
		div.dataset.idx = String(index);
		div.dataset.level = line.displayClass;
		div.replaceChildren();

		const ts = document.createElement("span");
		ts.className = "line-ts";
		ts.textContent = timeFmt.format(new Date(line.timestamp));

		const txt = document.createElement("span");
		txt.className = "line-text";
		if (this.state.normalizedQuery && this.state.searchRegex) {
			txt.innerHTML = highlightText(
				line.text,
				this.state.normalizedQuery,
				this.state.searchRegex,
			);
		} else {
			txt.textContent = line.text;
		}

		div.appendChild(ts);
		div.appendChild(txt);
	}

	private removeActiveMatch() {
		this.viewport
			?.querySelector(".log-line.search-active")
			?.classList.remove("search-active");
	}

	rebuild() {
		const viewport = this.viewport;
		if (!viewport) return;
		const container = this.getLinesContainer();
		if (!container) return;

		this.clearContainer(container);
		this.state.currentMatchIndex = 0;

		const frag = document.createDocumentFragment();
		for (let i = 0; i < this.state.lines.length; i++) {
			if (this.state.lineVisible(this.state.lines[i])) {
				const div = this.acquireNode();
				this.initLineNode(div, this.state.lines[i], i);
				frag.appendChild(div);
			}
		}
		container.appendChild(frag);

		requestAnimationFrame(() => {
			if (!this.viewport) return;
			this.updateMatches(false);
			if (this.isAtBottom()) {
				viewport.scrollTop = viewport.scrollHeight;
			}
		});
	}

	appendLines(newLines: LogLine[]) {
		const viewport = this.viewport;
		const container = this.getLinesContainer();
		if (!container || !viewport) return;

		const startIndex = this.state.lines.length - newLines.length;
		const frag = document.createDocumentFragment();
		const inserted: HTMLDivElement[] = [];
		for (let i = 0; i < newLines.length; i++) {
			const line = newLines[i];
			if (this.state.lineVisible(line)) {
				const div = this.acquireNode();
				this.initLineNode(
					div,
					line,
					startIndex + i,
					!this.shouldReduceLogAnimations(),
				);
				frag.appendChild(div);
				inserted.push(div);
			}
		}
		container.appendChild(frag);

		requestAnimationFrame(() => {
			if (!this.viewport) return;
			for (const div of inserted) {
				div.classList.remove("new");
			}
			this.updateMatches(false);
			if (this.isAtBottom()) {
				viewport.scrollTop = viewport.scrollHeight;
			}
		});
	}

	applyLevelVisibility() {
		const container = this.getLinesContainer();
		if (!container) return;
		container.querySelectorAll<HTMLElement>(".log-line").forEach((node) => {
			const lvl = node.dataset.level || "message";
			node.classList.toggle("hidden", !this.state.activeLevels.has(lvl));
		});
		this.state.currentMatchIndex = 0;
		this.updateMatches(false);
	}

	clear() {
		const container = this.getLinesContainer();
		if (container) this.clearContainer(container);
		this.pool.length = 0;
		this.state.matchCount = 0;
		this.state.currentMatchIndex = 0;
	}

	private findMatches(): HTMLElement[] {
		const q = this.state.normalizedQuery;
		const container = this.getLinesContainer();
		if (!q || !container) return [];

		const matches: HTMLElement[] = [];
		for (let i = 0; i < container.children.length; i++) {
			const el = container.children[i] as HTMLElement;
			if (el.classList.contains("hidden")) continue;
			const text = el.querySelector(".line-text")?.textContent;
			if (text && text.toLowerCase().includes(q)) {
				matches.push(el);
			}
		}
		return matches;
	}

	private updateMatches(scroll = false) {
		const matches = this.findMatches();
		this.state.matchCount = matches.length;

		if (matches.length === 0) {
			this.state.currentMatchIndex = 0;
			this.removeActiveMatch();
			return;
		}

		let idx = this.state.currentMatchIndex;
		if (idx < 1 || idx > matches.length) idx = 1;
		this.state.currentMatchIndex = idx;
		this.goToMatch(idx, scroll, matches);
	}

	private goToMatch(index: number, scroll = true, matches?: HTMLElement[]) {
		if (!this.viewport || this.state.matchCount === 0) return;
		const list = matches ?? this.findMatches();
		if (list.length === 0) return;
		this.state.currentMatchIndex = Math.max(
			1,
			Math.min(index, list.length),
		);
		this.removeActiveMatch();
		const el = list[this.state.currentMatchIndex - 1];
		el.classList.add("search-active");
		if (scroll) {
			el.scrollIntoView({ block: "center" });
		}
	}

	nextMatch() {
		if (this.state.matchCount === 0) return;
		const next = (this.state.currentMatchIndex % this.state.matchCount) + 1;
		this.goToMatch(next, true);
	}

	prevMatch() {
		if (this.state.matchCount === 0) return;
		const prev = this.state.currentMatchIndex - 1;
		this.goToMatch(prev < 1 ? this.state.matchCount : prev, true);
	}

	private isAtBottom(): boolean {
		if (!this.viewport) return true;
		return (
			this.viewport.scrollHeight -
				this.viewport.scrollTop -
				this.viewport.clientHeight <
			SCROLL_THRESHOLD
		);
	}

	private computeUnseenCount(): number {
		if (!this.viewport) return 0;
		const container = this.getLinesContainer();
		if (!container?.lastElementChild) return 0;

		const last = container.lastElementChild as HTMLElement;
		const lastBottom = last.getBoundingClientRect().bottom;
		const viewportBottom = this.viewport.getBoundingClientRect().bottom;
		const lineHeight = last.offsetHeight || 20;

		return Math.max(
			0,
			Math.ceil((lastBottom - viewportBottom) / lineHeight),
		);
	}

	handleScroll(
		onState?: (state: { isAtBottom: boolean; unseenCount: number }) => void,
	) {
		if (!this.viewport || this.scrollTicking) return;
		this.scrollTicking = true;
		requestAnimationFrame(() => {
			this.scrollTicking = false;
			const atBottom = this.isAtBottom();
			const unseen = atBottom ? 0 : this.computeUnseenCount();
			onState?.({ isAtBottom: atBottom, unseenCount: unseen });
		});
	}

	scrollToBottom() {
		if (!this.viewport) return;
		this.viewport.scrollTo({
			top: this.viewport.scrollHeight,
			behavior: "smooth",
		});
	}
}
