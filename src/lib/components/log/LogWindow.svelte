<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import {
		showSuccess,
		showError,
		launcherStore,
	} from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import { openUrl } from "$lib/api/cubicApi";
import { LogState } from "./logState.svelte";
import { LogRenderer } from "./LogRenderer";
import { CONSOLE_HISTORY_MAX } from "./logHelpers";
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

	const activeUser = $derived(
		launcherStore.settings.user[launcherStore.settings.active_user_idx],
	);
	const activeUsername = $derived(activeUser?.username ?? null);
	const consoleHistoryLimit = $derived(
		Math.max(
			100,
			Math.min(
				launcherStore.settings.console_history_limit ??
					CONSOLE_HISTORY_MAX,
				CONSOLE_HISTORY_MAX,
			),
		),
	);
	const showLevelTags = $derived(
		launcherStore.settings.console_show_level_tags ?? true,
	);

	$effect(() => {
		log.setMaxLines(consoleHistoryLimit);
	});

	function buildPrivacyTerms(username: string | null): string[] {
		// El backend ya sanitiza tokens, session ids, correos e IPs.
		// Aquí solo ocultamos líneas que contengan el nombre de usuario activo.
		return username ? [username.toLowerCase()] : [];
	}

	$effect(() => {
		log.setPrivacyTerms(buildPrivacyTerms(activeUsername));
	});

	let isAtBottom = $state(true);
	let unseenCount = $state(0);
	let destroyed = false;
	let unlistenFn: (() => void) | undefined;

	function onScrollState(state: {
		isAtBottom: boolean;
		unseenCount: number;
	}) {
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
		log.setPrivacyTerms(buildPrivacyTerms(activeUsername));

		(async () => {
			const raw = await invoke<
				{
					id: number;
					text: string;
					stream: string;
					level: string;
					timestamp: number;
				}[]
			>("get_log_history_cmd", {
				instanceId,
				limit: consoleHistoryLimit,
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
	<div class="log-card">
		<LogHeader
			{instanceName}
			totalLines={log.totalLines}
			{isAtBottom}
			uploading={log.uploading}
			{onClear}
			onCopy={copyLog}
			onUpload={uploadToMclogs}
			onScrollBottom={() => renderer.scrollToBottom()}
		/>

	<LogControls
		activeLevels={log.activeLevels}
		query={log.inputQuery}
		matchCount={log.matchCount}
		currentMatchIndex={log.currentMatchIndex}
		{showLevelTags}
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
	</div>

	{#if !isAtBottom && unseenCount > 0}
		<button
			type="button"
			class="jump-bottom"
			onclick={() => renderer.scrollToBottom()}
		>
			<img
				class="jump-icon"
				src="/images/icons/log/arrow-down.svg"
				alt=""
			/>
			{unseenCount} líneas nuevas
		</button>
	{/if}
</div>

<style>
	.log-window {
		position: relative;
		display: flex;
		flex-direction: column;
		height: 100vh;
		padding: 16px;
		box-sizing: border-box;
		background: var(--bg-main);
		color: var(--text-primary);
		font-size: 0.8rem;
	}

	.log-card {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--bg-card-gradient), var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		box-shadow: var(--shadow-sm);
		overflow: hidden;
		contain: layout paint;
	}

	.jump-bottom {
		position: absolute;
		bottom: 28px;
		left: 50%;
		transform: translateX(-50%);
		background: var(--bg-card-gradient), var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text-primary);
		padding: 8px 18px;
		border-radius: 20px;
		font-size: 0.72rem;
		font-family: inherit;
		font-weight: 600;
		cursor: pointer;
		backdrop-filter: blur(var(--backdrop-blur-float, 4px));
		box-shadow: var(--shadow-md);
		transition: all 0.2s ease;
		z-index: 10;
	}

	.jump-bottom:hover {
		background: var(--bg-card);
		border-color: var(--accent);
		color: var(--accent);
	}

	.jump-icon {
		width: 14px;
		height: 14px;
		flex-shrink: 0;
		filter: var(--icon-filter);
	}

	:global(.log-line) {
		display: flex;
		align-items: flex-start;
		gap: 12px;
		padding: 1px 18px;
		min-height: 18px;
		font-family:
			ui-monospace, SFMono-Regular, "SF Mono", Consolas,
			"Liberation Mono", Menlo, monospace;
		content-visibility: auto;
		contain-intrinsic-size: auto 18px;
	}

	:global(.log-line.hidden) {
		display: none;
	}

	:global(.log-line:hover) {
		background: var(--surface-hover);
	}

	:global(.log-line.new) {
		animation: logSlideIn 0.2s ease-out;
	}

	:global(.log-line.search-active) {
		background: color-mix(in srgb, var(--accent) 10%, transparent);
		box-shadow: inset 2px 0 0 0 var(--accent);
	}

	:global(.log-line.stderr) {
		background: color-mix(in srgb, var(--color-error) 5%, transparent);
	}

	:global(.line-ts) {
		color: var(--text-muted);
		font-size: 0.65rem;
		flex-shrink: 0;
		width: 72px;
		text-align: right;
		user-select: none;
		opacity: 0.75;
		padding-top: 1px;
		line-height: 1.45;
	}

	:global(.line-text) {
		color: var(--text-primary);
		white-space: pre-wrap;
		word-break: break-all;
		min-width: 0;
		font-size: 0.72rem;
		line-height: 1.45;
	}

	:global(.line-text mark) {
		background: color-mix(in srgb, var(--accent) 28%, transparent);
		color: inherit;
		padding: 0 2px;
		border-radius: 2px;
	}

	:global(.log-line.info .line-text) {
		color: var(--color-success);
	}

	:global(.log-line.warn .line-text) {
		color: var(--color-warning);
	}

	:global(.log-line.error .line-text) {
		color: var(--color-error);
	}

	:global(.log-line.fatal .line-text) {
		color: var(--color-error);
		font-weight: 700;
	}

	:global(.log-line.stderr .line-text) {
		color: var(--color-error);
	}

	:global(.log-line.launcher .line-text) {
		color: var(--color-info);
		font-style: italic;
	}

	:global(.log-line.trace .line-text),
	:global(.log-line.debug .line-text) {
		color: var(--text-muted);
	}

	:global(.log-line.trace .line-text) {
		font-size: 0.65rem;
	}

	:global(.log-line.message .line-text) {
		color: var(--text-primary);
	}

	:global(.log-line.unknown .line-text) {
		color: var(--text-tertiary);
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

	@media (min-width: 1100px) {
		.log-window {
			padding: 24px;
		}

		.jump-bottom {
			bottom: 36px;
		}
	}
</style>
