<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { showSuccess, showError } from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import { openUrl } from "$lib/api/cubicApi";
	import { LogState } from "./logState.svelte";
	import { LogRenderer } from "./LogRenderer";
	import LogHeader from "./LogHeader.svelte";
	import LogControls from "./LogControls.svelte";
	import LogViewport from "./LogViewport.svelte";

	let {
		instanceId,
		instanceName,
	}: { instanceId: string; instanceName: string } = $props();

	const log = new LogState();
	const renderer = new LogRenderer(log);
	log.setRenderer(renderer);

	let isAtBottom = $state(true);
	let unseenCount = $state(0);
	let destroyed = false;
	let unlistenFn: (() => void) | undefined;

	function onScrollState(state: { isAtBottom: boolean; unseenCount: number }) {
		isAtBottom = state.isAtBottom;
		unseenCount = state.unseenCount;
	}

	function handleSearchKeydown(e: KeyboardEvent) {
		if (e.key === "Enter") {
			e.preventDefault();
			log.flushSearch();
			if (e.shiftKey) renderer.prevMatch();
			else renderer.nextMatch();
		}
		if (e.key === "Escape") {
			log.resetSearch();
		}
	}

	function handleGlobalKeydown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
			e.preventDefault();
			const input = document.getElementById(
				"log-search-input",
			) as HTMLInputElement | null;
			input?.focus();
			input?.select();
		}
	}

	async function copyLog() {
		const text = log.lines.map((l) => l.text).join("\n");
		await navigator.clipboard.writeText(text);
	}

	async function uploadToMclogs() {
		if (log.uploading || log.totalLines === 0) return;
		log.uploading = true;
		try {
			const text = log.lines.map((l) => l.text).join("\n");
			const url = await invoke<string>("upload_log_to_mclogs", {
				content: text,
			});
			showSuccess("McLogs", url);
			await openUrl(url);
		} catch (err) {
			showError(t("errors.title"), String(err));
		} finally {
			log.uploading = false;
		}
	}

	function onClear() {
		log.clear();
		renderer.scrollToBottom();
	}

	onMount(() => {
		destroyed = false;

		(async () => {
			const raw = await invoke<
				{ id: number; text: string; stream: string; level: string; timestamp: number }[]
			>("get_log_history_cmd", {
				instanceId,
			});
			log.ingestHistory(raw);
			renderer.rebuild();
			renderer.scrollToBottom();

			unlistenFn = await listen<{
				id: string;
				lines: {
					id: number;
					line: string;
					stream: string;
					level: string;
					timestamp: number;
				}[];
			}>("instance-log-batch", (event) => {
				if (destroyed || event.payload.id !== instanceId) return;
				log.ingestBatch(event.payload.lines);
			});
		})();

		document.addEventListener("keydown", handleGlobalKeydown);

		return () => {
			destroyed = true;
			unlistenFn?.();
			document.removeEventListener("keydown", handleGlobalKeydown);
			renderer.detach();
		};
	});
</script>

<div class="log-window">
	<LogHeader
		{instanceName}
		totalLines={log.totalLines}
		{isAtBottom}
		uploading={log.uploading}
		onClear={onClear}
		onCopy={copyLog}
		onUpload={uploadToMclogs}
		onScrollBottom={() => renderer.scrollToBottom()}
	/>

	<LogControls
		activeLevels={log.activeLevels}
		query={log.inputQuery}
		matchCount={log.matchCount}
		currentMatchIndex={log.currentMatchIndex}
		onQueryInput={(v) => log.searchInput(v)}
		onQueryKeydown={handleSearchKeydown}
		onClearQuery={() => log.resetSearch()}
		onPrev={() => {
			log.flushSearch();
			renderer.prevMatch();
		}}
		onNext={() => {
			log.flushSearch();
			renderer.nextMatch();
		}}
		onToggleLevel={(l) => log.toggleLevel(l)}
		onSetAllLevels={(a) => log.setAllLevels(a)}
	/>

	<LogViewport {renderer} {onScrollState} />

	{#if !isAtBottom && unseenCount > 0}
		<button type="button" class="jump-bottom" onclick={() => renderer.scrollToBottom()}>
			↓ {unseenCount} líneas nuevas
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
		background: var(--bg-input, #0a0a0a);
		color: var(--text-primary, #c8c8c8);
		font-size: 0.65rem;
	}

	.jump-bottom {
		position: absolute;
		bottom: 12px;
		left: 50%;
		transform: translateX(-50%);
		background: rgba(30, 30, 30, 0.9);
		border: 1px solid var(--border, #444);
		color: var(--text-primary, #ccc);
		padding: 6px 16px;
		border-radius: 20px;
		font-size: 0.6rem;
		font-family: inherit;
		cursor: pointer;
		backdrop-filter: blur(var(--backdrop-blur-float, 4px));
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
		transition: all 0.2s ease;
		z-index: 10;
	}

	.jump-bottom:hover {
		background: rgba(50, 50, 50, 0.95);
		border-color: var(--text-secondary, #666);
		color: white;
	}

	:global(.log-line) {
		display: flex;
		gap: 10px;
		padding: 0 14px;
		min-height: 16px;
		content-visibility: auto;
		contain-intrinsic-size: auto 16px;
	}

	:global(.log-line.hidden) {
		display: none;
	}

	:global(.log-line:hover) {
		background: rgba(255, 255, 255, 0.02);
	}

	:global(.log-line.new) {
		animation: logSlideIn 0.2s ease-out;
	}

	:global(.log-line.search-active) {
		background: rgba(255, 235, 59, 0.1);
		box-shadow: inset 2px 0 0 0 rgba(255, 235, 59, 0.8);
	}

	:global(.log-line.stderr) {
		background: rgba(244, 67, 54, 0.03);
	}

	:global(.line-ts) {
		color: var(--text-muted, #444);
		font-size: 0.6rem;
		flex-shrink: 0;
		width: 68px;
		text-align: right;
		user-select: none;
		opacity: 0.6;
		padding-top: 1px;
	}

	:global(.line-text) {
		color: var(--text-primary, #c8c8c8);
		white-space: pre-wrap;
		word-break: break-all;
		min-width: 0;
	}

	:global(.line-text mark) {
		background: rgba(255, 235, 59, 0.28);
		color: inherit;
		padding: 0 1px;
		border-radius: 2px;
	}

	:global(.log-line.info .line-text) {
		color: var(--color-success, #81c784);
	}

	:global(.log-line.warn .line-text) {
		color: var(--color-warning, #ffd54f);
	}

	:global(.log-line.error .line-text) {
		color: var(--color-error, #e57373);
	}

	:global(.log-line.fatal .line-text) {
		color: var(--color-error, #ef5350);
		font-weight: 700;
	}

	:global(.log-line.stderr .line-text) {
		color: #ff8a65;
	}

	:global(.log-line.launcher .line-text) {
		color: #82b1ff;
		font-style: italic;
	}

	:global(.log-line.trace .line-text),
	:global(.log-line.debug .line-text) {
		color: var(--text-muted, #888);
	}

	:global(.log-line.trace .line-text) {
		font-size: 0.58rem;
	}

	:global(.log-line.message .line-text) {
		color: var(--text-primary, #c8c8c8);
	}

	:global(.log-line.unknown .line-text) {
		color: var(--text-tertiary, #888);
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
</style>
