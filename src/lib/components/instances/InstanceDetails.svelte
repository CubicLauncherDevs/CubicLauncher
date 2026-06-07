<script lang="ts">
	import type { InstanceDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";

	interface ConsoleLine {
		text: string;
		stream: "stdout" | "stderr";
		timestamp: number;
	}

	let { instance } = $props<{ instance: InstanceDto }>();

	let lines = $state<ConsoleLine[]>([]);
	let autoScroll = $state(true);
	let logContainer: HTMLDivElement | undefined = $state();

	const MAX_LINES = 2000;
	const isRunning = $derived(
		instance.status === "started" || instance.status === "starting",
	);

	const loaderColors: Record<string, string> = {
		Vanilla: "var(--clr-loader-vanilla, #78909c)",
		Fabric: "var(--clr-loader-fabric, #66bb6a)",
		Forge: "var(--clr-loader-forge, #ffa726)",
		Quilt: "var(--clr-loader-quilt, #ab47bc)",
	};
	const loaderColor = $derived(loaderColors[instance.loader] || "#78909c");

	const statusLabel = $derived(
		instance.status === "started"
			? t("instanceView.status.started")
			: instance.status === "starting"
				? t("instanceView.status.starting")
				: instance.status === "error"
					? "Error"
					: t("instanceView.status.idle"),
	);
	const statusClass = $derived(
		instance.status === "started"
			? "status-started"
			: instance.status === "starting"
				? "status-starting"
				: instance.status === "error"
					? "status-error"
					: "status-idle",
	);

	function openDir(subDir?: string) {
		invoke("open_instance_dir", {
			id: instance.uuid,
			subDir: subDir ?? null,
		});
	}

	function clearConsole() {
		lines = [];
	}

	async function copyConsole() {
		const text = lines.map((l) => l.text).join("\n");
		await navigator.clipboard.writeText(text);
	}

	function handleScroll() {
		if (!logContainer) return;
		const el = logContainer;
		autoScroll = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
	}

	$effect(() => {
		const id = instance.uuid;
		if (!id) return;

		lines = [];

		const unlistenPromise = listen<{ id: string; line: string; stream: string }>(
			"instance-console-output",
			(event) => {
				if (event.payload.id === id) {
					const line: ConsoleLine = {
						text: event.payload.line,
						stream: event.payload.stream as "stdout" | "stderr",
						timestamp: Date.now(),
					};
					lines = [...lines.slice(-(MAX_LINES - 1)), line];
				}
			},
		).catch(() => {});

		return () => {
			unlistenPromise.then((unsub) => unsub?.());
		};
	});

	$effect(() => {
		if (autoScroll && logContainer) {
			void lines;
			requestAnimationFrame(() => {
				if (logContainer) {
					logContainer.scrollTop = logContainer.scrollHeight;
				}
			});
		}
	});

	function fmtTime(ts: number): string {
		const d = new Date(ts);
		return d.toLocaleTimeString("en-US", { hour12: false });
	}

	function lineLevel(text: string): string {
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
		if (/\b(INFO|CONFIG|DEBUG|TRACE)\b/.test(u)) return "info";
		return "default";
	}
</script>

<div class="details-panel">
	<div class="details-header">
		<div class="info-badges">
			<span class="badge badge-version">{instance.version}</span>
			<span
				class="badge badge-loader"
				style="background: {loaderColor}20; color: {loaderColor}; border-color: {loaderColor}40;"
			>
				{instance.loader}
			</span>
			<span class="badge badge-status {statusClass}">{statusLabel}</span>
		</div>
		<div class="path-row">
			<span class="path-text" title={instance.path}>{instance.path}</span>
			<button
				type="button"
				class="icon-btn"
				onclick={() => openDir()}
				title={t("instanceView.details.location")}
			>
				<svg
					width="14"
					height="14"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path
						d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
					/>
				</svg>
			</button>
		</div>
	</div>

	<div class="action-bar">
		<button type="button" class="action-chip" onclick={() => openDir()}>
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path
					d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
				/>
			</svg>
			{t("instanceView.options.folder")}
		</button>
		<button
			type="button"
			class="action-chip"
			onclick={() => openDir("mods")}
		>
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<rect x="3" y="3" width="7" height="7" /><rect
					x="14"
					y="3"
					width="7"
					height="7"
				/><rect x="14" y="14" width="7" height="7" /><rect
					x="3"
					y="14"
					width="7"
					height="7"
				/>
			</svg>
			{t("instanceView.tabs.mods")}
		</button>
		<button
			type="button"
			class="action-chip"
			onclick={() => openDir("screenshots")}
		>
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path
					d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"
				/><circle cx="12" cy="13" r="4" />
			</svg>
			{t("instanceView.tabs.screenshots")}
		</button>
		<button
			type="button"
			class="action-chip"
			onclick={() => openDir("resourcepacks")}
		>
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<ellipse cx="12" cy="5" rx="9" ry="3" /><path
					d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"
				/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
			</svg>
			{t("instanceView.tabs.resources")}
		</button>
	</div>

	<div class="console-section">
		<div class="console-header">
			<div class="console-title">
				<span class="console-dot" class:alive={isRunning}></span>
				<span
					>Console {#if lines.length > 0}({lines.length}){/if}</span
				>
			</div>
			<div class="console-toolbar">
				<button
					type="button"
					class="toolbar-btn"
					onclick={clearConsole}
					disabled={lines.length === 0}
					title="Clear"
				>
					<svg
						width="13"
						height="13"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<polyline points="3 6 5 6 21 6" /><path
							d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
						/>
					</svg>
				</button>
				<button
					type="button"
					class="toolbar-btn"
					onclick={copyConsole}
					disabled={lines.length === 0}
					title="Copy"
				>
					<svg
						width="13"
						height="13"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<rect
							x="9"
							y="9"
							width="13"
							height="13"
							rx="2"
							ry="2"
						/><path
							d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
						/>
					</svg>
				</button>
				<button
					type="button"
					class="toolbar-btn"
					class:active={autoScroll}
					onclick={() => (autoScroll = !autoScroll)}
					title="Auto-scroll"
				>
					<svg
						width="13"
						height="13"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<line x1="12" y1="5" x2="12" y2="19" /><polyline
							points="19 12 12 19 5 12"
						/>
					</svg>
				</button>
				{#if isRunning}
					<span class="live-badge">LIVE</span>
				{/if}
			</div>
		</div>

		<div
			class="console-viewport"
			bind:this={logContainer}
			onscroll={handleScroll}
		>
			{#if lines.length === 0 && !isRunning}
				<div class="console-placeholder">
					<svg
						width="32"
						height="32"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1"
						stroke-linecap="round"
						stroke-linejoin="round"
						opacity="0.3"
					>
						<polyline points="16 18 22 12 16 6" /><polyline
							points="8 6 2 12 8 18"
						/>
					</svg>
					<span>{t("instanceView.console.placeholder")}</span>
				</div>
			{:else if lines.length === 0 && isRunning}
				<div class="console-placeholder">
					<div class="waiting-dots">
						<span></span><span></span><span></span>
					</div>
					<span>{t("instanceView.console.waiting")}</span>
				</div>
			{:else}
				{#each lines as line, i (i)}
					<div
						class="console-line {lineLevel(line.text)}"
						class:stderr={line.stream === "stderr"}
					>
						<span class="line-ts">{fmtTime(line.timestamp)}</span>
						<span class="line-text">{line.text}</span>
					</div>
				{/each}
			{/if}
		</div>
	</div>
</div>

<style>
	.details-panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		gap: 16px;
		padding: 0;
	}

	.details-header {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		justify-content: space-between;
		gap: 12px;
		padding: 16px;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: 10px;
	}

	.info-badges {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		align-items: center;
	}

	.badge {
		font-size: 0.7rem;
		font-weight: 700;
		padding: 3px 10px;
		border-radius: 20px;
		letter-spacing: 0.5px;
		text-transform: uppercase;
		white-space: nowrap;
	}

	.badge-version {
		background: rgba(255, 255, 255, 0.06);
		color: var(--text-primary);
		border: 1px solid var(--border);
	}

	.badge-loader {
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid var(--border);
	}

	.badge-status {
		font-size: 0.62rem;
	}

	.badge-status.status-idle {
		background: rgba(120, 144, 156, 0.15);
		color: #90a4ae;
		border: 1px solid rgba(120, 144, 156, 0.3);
	}

	.badge-status.status-starting {
		background: rgba(33, 150, 243, 0.15);
		color: #64b5f6;
		border: 1px solid rgba(33, 150, 243, 0.3);
	}

	.badge-status.status-started {
		background: rgba(76, 175, 80, 0.15);
		color: #81c784;
		border: 1px solid rgba(76, 175, 80, 0.3);
	}

	.badge-status.status-error {
		background: rgba(244, 67, 54, 0.15);
		color: #e57373;
		border: 1px solid rgba(244, 67, 54, 0.3);
	}

	.path-row {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		max-width: 100%;
	}

	.path-text {
		font-size: 0.68rem;
		color: var(--text-secondary);
		font-family: "Fira Code", "Cascadia Code", monospace;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 280px;
		opacity: 0.7;
	}

	.icon-btn {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		width: 26px;
		height: 26px;
		border-radius: 5px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		transition: all 0.15s ease;
	}

	.icon-btn:hover {
		background: rgba(255, 255, 255, 0.06);
		color: var(--text-primary);
		border-color: rgba(255, 255, 255, 0.2);
	}

	.action-bar {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.action-chip {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 12px;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: 6px;
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 0.72rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s ease;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.action-chip:hover {
		background: rgba(255, 255, 255, 0.07);
		color: var(--text-primary);
		border-color: rgba(255, 255, 255, 0.15);
	}

	.console-section {
		flex: 1;
		display: flex;
		flex-direction: column;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid var(--border);
		border-radius: 10px;
		overflow: hidden;
		min-height: 0;
	}

	.console-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 14px;
		background: rgba(255, 255, 255, 0.02);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.console-title {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 0.65rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 1px;
		color: var(--text-secondary);
	}

	.console-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #546e7a;
		transition: all 0.3s ease;
	}

	.console-dot.alive {
		background: #66bb6a;
		box-shadow: 0 0 8px rgba(102, 187, 106, 0.5);
		animation: pulse-dot 1.5s ease-in-out infinite;
	}

	.console-toolbar {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.toolbar-btn {
		background: transparent;
		border: none;
		color: var(--text-secondary);
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
		color: var(--text-primary);
	}

	.toolbar-btn.active {
		color: #81c784;
		background: rgba(76, 175, 80, 0.12);
	}

	.toolbar-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.live-badge {
		font-size: 0.55rem;
		font-weight: 800;
		letter-spacing: 1px;
		padding: 2px 6px;
		border-radius: 3px;
		background: rgba(76, 175, 80, 0.15);
		color: #66bb6a;
		border: 1px solid rgba(76, 175, 80, 0.3);
		animation: pulse-badge 2s ease-in-out infinite;
	}

	.console-viewport {
		flex: 1;
		overflow-y: auto;
		padding: 8px 0;
		font-family: "Fira Code", "Cascadia Code", "JetBrains Mono", monospace;
		font-size: 0.78rem;
		line-height: 1.5;
		background: #080808;
		scrollbar-gutter: stable;
	}

	.console-viewport::-webkit-scrollbar {
		width: 4px;
	}

	.console-viewport::-webkit-scrollbar-thumb {
		background: var(--border);
		border-radius: 4px;
	}

	.console-placeholder {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 12px;
		height: 100%;
		min-height: 120px;
		color: var(--text-secondary);
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.8px;
		font-weight: 600;
		opacity: 0.6;
	}

	.console-line {
		display: flex;
		gap: 12px;
		padding: 1px 14px;
		min-height: 20px;
		transition: background 0.1s ease;
	}

	.console-line:hover {
		background: rgba(255, 255, 255, 0.02);
	}

	.console-line.stderr {
		background: rgba(244, 67, 54, 0.03);
	}

	.line-ts {
		color: #444;
		font-size: 0.68rem;
		flex-shrink: 0;
		width: 68px;
		text-align: right;
		user-select: none;
		opacity: 0.6;
		padding-top: 1px;
	}

	.line-text {
		color: #c8c8c8;
		white-space: pre-wrap;
		word-break: break-all;
		min-width: 0;
	}

	.console-line.info .line-text {
		color: #81c784;
	}

	.console-line.warn .line-text {
		color: #ffd54f;
	}

	.console-line.error .line-text {
		color: #e57373;
	}

	.console-line.fatal .line-text {
		color: #ef5350;
		font-weight: 700;
	}

	.console-line.stderr .line-text {
		color: #ff8a65;
	}

	.waiting-dots {
		display: flex;
		gap: 4px;
	}

	.waiting-dots span {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-secondary);
		animation: dot-bounce 1.2s ease-in-out infinite;
	}

	.waiting-dots span:nth-child(2) {
		animation-delay: 0.2s;
	}

	.waiting-dots span:nth-child(3) {
		animation-delay: 0.4s;
	}

	@keyframes pulse-dot {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.5;
		}
	}

	@keyframes pulse-badge {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.6;
		}
	}

	@keyframes dot-bounce {
		0%,
		80%,
		100% {
			transform: scale(0.6);
			opacity: 0.4;
		}
		40% {
			transform: scale(1);
			opacity: 1;
		}
	}

	@media (max-width: 550px) {
		.details-header {
			flex-direction: column;
		}

		.path-text {
			max-width: 180px;
		}
	}
</style>
