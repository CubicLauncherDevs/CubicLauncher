<script lang="ts">
	import { onMount, tick } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";

	let { instanceId, instanceName }: { instanceId: string; instanceName: string } = $props();

	interface LogLine {
		text: string;
		stream: string;
		timestamp: number;
	}

	const MAX_LINES = 4000;
	const SCROLL_THRESHOLD = 60;

	let lines: LogLine[] = $state([]);
	let isAtBottom = $state(true);
	let unseenCount = $state(0);
	let logContainer: HTMLDivElement | undefined = $state();
	let unlistenFn: (() => void) | undefined;

	function computeLevel(text: string): "info" | "warn" | "error" | "fatal" | "default" {
		const m = text.match(/\[.*?\/(\w+)\]/);
		if (m) {
			const lv = m[1].toUpperCase();
			if (["FATAL", "SEVERE"].includes(lv)) return "fatal";
			if (lv === "ERROR") return "error";
			if (["WARN", "WARNING"].includes(lv)) return "warn";
			if (["INFO", "CONFIG", "FINE", "FINER", "FINEST", "DEBUG", "TRACE"].includes(lv))
				return "info";
		}
		const u = text.toUpperCase();
		if (/\b(FATAL|SEVERE)\b/.test(u)) return "fatal";
		if (/\bERROR\b/.test(u)) return "error";
		if (/\bWARN(ING)?\b/.test(u)) return "warn";
		return "default";
	}

	const timeFmt = new Intl.DateTimeFormat("en-US", {
		hour: "2-digit",
		minute: "2-digit",
		second: "2-digit",
		hour12: false,
	});

	function appendLines(newLines: LogLine[]) {
		lines.push(...newLines);
		if (lines.length > MAX_LINES) {
			lines = lines.slice(-MAX_LINES);
		}
		renderLines();
	}

	function renderLines() {
		const viewport = logContainer;
		if (!viewport) return;
		const container = viewport.querySelector(".log-lines");
		if (!container) return;

		const frag = document.createDocumentFragment();
		const start = container.children.length;
		for (let i = start; i < lines.length; i++) {
			const line = lines[i];
			const level = line.stream === "stderr" ? "error" : computeLevel(line.text);
			const div = document.createElement("div");
			div.className = `log-line ${level}${line.stream === "stderr" ? " stderr" : ""} new`;
			const ts = document.createElement("span");
			ts.className = "line-ts";
			ts.textContent = timeFmt.format(new Date(line.timestamp));
			const txt = document.createElement("span");
			txt.className = "line-text";
			txt.textContent = line.text;
			div.appendChild(ts);
			div.appendChild(txt);
			frag.appendChild(div);
		}
		container.appendChild(frag);

		requestAnimationFrame(() => {
			container.querySelectorAll(".log-line.new").forEach((el) => {
				el.classList.remove("new");
			});
		});

		if (isAtBottom && viewport) {
			viewport.scrollTop = viewport.scrollHeight;
		}
	}

	onMount(() => {
		let destroyed = false;

		(async () => {
			const history: LogLine[] = await invoke("get_log_history_cmd", { instanceId });
			lines = history;
			await tick();

			const viewport = logContainer;
			if (viewport) {
				const container = viewport.querySelector(".log-lines");
				if (container) {
					const frag = document.createDocumentFragment();
					for (const line of lines) {
						const level = line.stream === "stderr" ? "error" : computeLevel(line.text);
						const div = document.createElement("div");
						div.className = `log-line ${level}${line.stream === "stderr" ? " stderr" : ""}`;
						const ts = document.createElement("span");
						ts.className = "line-ts";
						ts.textContent = timeFmt.format(new Date(line.timestamp));
						const txt = document.createElement("span");
						txt.className = "line-text";
						txt.textContent = line.text;
						div.appendChild(ts);
						div.appendChild(txt);
						frag.appendChild(div);
					}
					container.appendChild(frag);
				}
				viewport.scrollTop = viewport.scrollHeight;
			}

			unlistenFn = await listen<{
				id: string;
				lines: { line: string; stream: string; timestamp: number }[];
			}>("instance-log-batch", (event) => {
				if (destroyed || event.payload.id !== instanceId) return;
				const batch: LogLine[] = event.payload.lines.map((e) => ({
					text: e.line,
					stream: e.stream,
					timestamp: e.timestamp,
				}));
				appendLines(batch);
			});
		})();

		return () => {
			destroyed = true;
			unlistenFn?.();
		};
	});

	function handleScroll() {
		if (!logContainer) return;
		const el = logContainer;
		const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < SCROLL_THRESHOLD;
		if (atBottom && !isAtBottom) {
			unseenCount = 0;
		}
		isAtBottom = atBottom;
		if (!isAtBottom) {
			const container = el.querySelector(".log-lines");
			if (container) {
				unseenCount = Math.max(0, container.children.length - Math.floor(el.scrollTop / 20) - Math.floor(el.clientHeight / 20));
			}
		}
	}

	function scrollToBottom() {
		if (logContainer) {
			logContainer.scrollTo({
				top: logContainer.scrollHeight,
				behavior: "smooth",
			});
			isAtBottom = true;
			unseenCount = 0;
		}
	}

	function clearLog() {
		lines = [];
		const container = logContainer?.querySelector(".log-lines");
		if (container) container.innerHTML = "";
		unseenCount = 0;
	}

	async function copyLog() {
		const text = lines.map((l) => l.text).join("\n");
		await navigator.clipboard.writeText(text);
	}
</script>

<div class="log-window">
	<div class="log-header">
		<div class="log-title">
			<span class="log-dot" class:alive={lines.length > 0}></span>
			<span>{instanceName}</span>
			{#if lines.length > 0}
				<span class="log-count">{lines.length}</span>
			{/if}
		</div>
		<div class="log-toolbar">
			<button type="button" class="toolbar-btn" onclick={clearLog} disabled={lines.length === 0} title="Clear log">
				<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<polyline points="3 6 5 6 21 6" /><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
				</svg>
			</button>
			<button type="button" class="toolbar-btn" onclick={copyLog} disabled={lines.length === 0} title="Copy log">
				<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<rect x="9" y="9" width="13" height="13" rx="2" ry="2" /><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
				</svg>
			</button>
			<button
				type="button"
				class="toolbar-btn"
				class:active={isAtBottom}
				onclick={scrollToBottom}
				title="Auto-scroll"
			>
				<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<line x1="12" y1="5" x2="12" y2="19" /><polyline points="19 12 12 19 5 12" />
				</svg>
			</button>
		</div>
	</div>

	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="log-viewport" bind:this={logContainer} onscroll={handleScroll}>
		<div class="log-lines"></div>
	</div>

	{#if !isAtBottom && unseenCount > 0}
		<button type="button" class="jump-bottom" onclick={scrollToBottom}>
			↓ {unseenCount} lineas nuevas
		</button>
	{/if}
</div>

<style>
	.log-window {
		position: relative;
		display: flex;
		flex-direction: column;
		height: 100vh;
		margin: 0;
		padding: 0;
		background: #0a0a0a;
		color: #c8c8c8;
	
		font-size: 0.65rem;
	}

	.log-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 6px 14px;
		background: #111;
		border-bottom: 1px solid #222;
		flex-shrink: 0;
	}

	.log-title {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 0.62rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 1px;
		color: #888;
	}

	.log-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #546e7a;
		transition: all 0.3s ease;
	}

	.log-dot.alive {
		background: #66bb6a;
		box-shadow: 0 0 8px rgba(102, 187, 106, 0.5);
		animation: pulse 1.5s ease-in-out infinite;
	}

	.log-count {
		background: rgba(255, 255, 255, 0.08);
		padding: 1px 6px;
		border-radius: 8px;
		font-size: 0.55rem;
		color: #666;
	}

	.log-toolbar {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.toolbar-btn {
		background: transparent;
		border: none;
		color: #666;
		width: 26px;
		height: 26px;
		border-radius: 4px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s ease;
	}

	.toolbar-btn:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.06);
		color: #ccc;
	}

	.toolbar-btn.active {
		color: #81c784;
		background: rgba(76, 175, 80, 0.12);
	}

	.toolbar-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.log-viewport {
		flex: 1;
		overflow-y: auto;
		contain: layout style;
	}

	.log-viewport::-webkit-scrollbar {
		width: 4px;
	}

	.log-viewport::-webkit-scrollbar-thumb {
		background: #333;
		border-radius: 4px;
	}

	:global(.log-line) {
		display: flex;
		gap: 10px;
		padding: 0 14px;
		min-height: 16px;
		content-visibility: auto;
		contain-intrinsic-size: auto 16px;
	}

	:global(.log-line:hover) {
		background: rgba(255, 255, 255, 0.02);
	}

	:global(.log-line.new) {
		animation: logSlideIn 0.2s ease-out;
	}

	:global(.log-line.stderr) {
		background: rgba(244, 67, 54, 0.03);
	}

	:global(.line-ts) {
		color: #444;
		font-size: 0.6rem;
		flex-shrink: 0;
		width: 68px;
		text-align: right;
		user-select: none;
		opacity: 0.6;
		padding-top: 1px;
	}

	:global(.line-text) {
		color: #c8c8c8;
		white-space: pre-wrap;
		word-break: break-all;
		min-width: 0;
	}

	:global(.log-line.info .line-text) {
		color: #81c784;
	}

	:global(.log-line.warn .line-text) {
		color: #ffd54f;
	}

	:global(.log-line.error .line-text) {
		color: #e57373;
	}

	:global(.log-line.fatal .line-text) {
		color: #ef5350;
		font-weight: 700;
	}

	:global(.log-line.stderr .line-text) {
		color: #ff8a65;
	}

	.jump-bottom {
		position: absolute;
		bottom: 12px;
		left: 50%;
		transform: translateX(-50%);
		background: rgba(30, 30, 30, 0.9);
		border: 1px solid #444;
		color: #ccc;
		padding: 6px 16px;
		border-radius: 20px;
		font-size: 0.6rem;
		font-family: inherit;
		cursor: pointer;
		backdrop-filter: blur(8px);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
		transition: all 0.2s ease;
		z-index: 10;
	}

	.jump-bottom:hover {
		background: rgba(50, 50, 50, 0.95);
		border-color: #666;
		color: white;
	}

	@keyframes logSlideIn {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@keyframes pulse {
		0%, 100% {
			opacity: 1;
		}
		50% {
			opacity: 0.5;
		}
	}
</style>
