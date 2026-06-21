<script lang="ts">
	import { onMount } from "svelte";
	import { slide } from "svelte/transition";
	import { SvelteMap } from "svelte/reactivity";
	import { listen } from "@tauri-apps/api/event";
	import { getDownloadQueue } from "$lib/api/cubicApi";
	import type { AppEvent } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";
	import DownloadIcon from "$lib/icons/DownloadIcon.svelte";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";

	type SegKey =
		| "Library"
		| "Asset"
		| "Native"
		| "Client"
		| "Verifying"
		| "Generic"
		| "Processing"
		| "Jre";
	const SEGS: SegKey[] = [
		"Library",
		"Asset",
		"Native",
		"Client",
		"Verifying",
		"Generic",
		"Processing",
		"Jre",
	];

	interface SegProg {
		current: number;
		total: number;
	}
	interface DlItem {
		version: string;
		activeType: SegKey | null;
		segs: Record<SegKey, SegProg>;
		done: boolean;
		error: string | null;
		retryMsg: string | null;
	}

	function emptySegs(): Record<SegKey, SegProg> {
		return {
			Library: { current: 0, total: 0 },
			Asset: { current: 0, total: 0 },
			Native: { current: 0, total: 0 },
			Client: { current: 0, total: 0 },
			Verifying: { current: 0, total: 0 },
			Generic: { current: 0, total: 0 },
			Processing: { current: 0, total: 0 },
			Jre: { current: 0, total: 0 },
		};
	}

	let downloads = new SvelteMap<string, DlItem>();
	let open = $state(false);
	let activeCount = $derived(
		[...downloads.values()].filter((d) => !d.done).length,
	);
	let doneCount = $derived(
		[...downloads.values()].filter((d) => d.done).length,
	);

	function pct(segs: Record<SegKey, SegProg>): number {
		const totalAll = SEGS.reduce((a, k) => a + segs[k].total, 0);
		const curAll = SEGS.reduce((a, k) => a + segs[k].current, 0);
		return totalAll > 0 ? Math.round((curAll / totalAll) * 100) : 0;
	}

	function statusLabel(activeType: SegKey | null): string {
		switch (activeType) {
			case "Library":
				return t("downloadProgress.statusLibs");
			case "Asset":
				return t("downloadProgress.statusAssets");
			case "Native":
				return t("downloadProgress.statusNatives");
			case "Client":
				return t("downloadProgress.statusClient");
			case "Verifying":
				return t("downloadProgress.statusVerifying");
			case "Generic":
				return t("downloadProgress.statusGeneric");
			case "Processing":
				return t("downloadProgress.statusProcessing");
			case "Jre":
				return t("downloadProgress.statusJre");
			default:
				return t("downloadProgress.statusGeneric");
		}
	}

	onMount(() => {
		getDownloadQueue().then((queue) => {
			for (const item of queue) {
				if (!downloads.has(item.version)) {
					downloads.set(item.version, {
						version: item.version,
						activeType: null,
						segs: emptySegs(),
						done: item.status === "done",
						error: null,
						retryMsg: null,
					});
				}
			}
			if (queue.length > 0) {
				open = true;
			}
		});

		const unlisten = listen<AppEvent>("app-event", (event) => {
			const p = event.payload;
			switch (p.type) {
				case "DEnqueue": {
					const { version } = p.data;
					if (!downloads.has(version)) {
						downloads.set(version, {
							version,
							activeType: null,
							segs: emptySegs(),
							done: false,
							error: null,
							retryMsg: null,
						});
						open = true;
					}
					break;
				}
				case "DProgress": {
					const { version, current, total, d_type } = p.data;
					const existing = downloads.get(version) ?? {
						version,
						activeType: null,
						segs: emptySegs(),
						done: false,
						error: null,
						retryMsg: null,
					};
					const key = SEGS.includes(d_type as SegKey)
						? (d_type as SegKey)
						: (existing.activeType ?? "Library");
					const newSegs = {
						...existing.segs,
						[key]: { current, total },
					};
					downloads.set(version, {
						...existing,
						segs: newSegs,
						activeType: key,
						done: false,
						retryMsg: null,
					});
					break;
				}
				case "DRetry": {
					const { version, attempt, max } = p.data;
					const existing = downloads.get(version) ?? {
						version,
						activeType: null,
						segs: emptySegs(),
						done: false,
						error: null,
						retryMsg: null,
					};
					downloads.set(version, {
						...existing,
						retryMsg: t("downloadProgress.retrying", {
							attempt,
							max,
						}),
					});
					break;
				}
				case "DFinish": {
					const { version } = p.data;
					const item = downloads.get(version);
					if (item) {
						downloads.set(version, {
							...item,
							done: true,
							activeType: null,
							retryMsg: null,
						});
					}
					setTimeout(() => {
						downloads.delete(version);
					}, 4000);
					break;
				}
				case "DError": {
					const { version, message } = p.data;
					const item = downloads.get(version);
					if (item) {
						downloads.set(version, {
							...item,
							done: true,
							activeType: null,
							error: message,
							retryMsg: null,
						});
					} else {
						downloads.set(version, {
							version,
							activeType: null,
							segs: emptySegs(),
							done: true,
							error: message,
							retryMsg: null,
						});
					}
					setTimeout(() => {
						downloads.delete(version);
					}, 8000);
					break;
				}
			}
		});

		return () => {
			unlisten.then((u) => u());
		};
	});
</script>

<div class="sd-root">
	<button
		type="button"
		class="sd-header"
		class:expanded={open}
		onclick={() => (open = !open)}
		aria-expanded={open}
	>
		<span class="sd-header-left">
			{#if activeCount > 0}
				<span class="sd-spinner"></span>
				<span class="sd-label"
					>{activeCount} {t("sidebar.downloading")}</span
				>
			{:else if doneCount > 0}
				<CheckIcon size={12} color="var(--color-success)" />
				<span class="sd-label"
					>{doneCount} {t("sidebar.completed")}</span
				>
			{:else}
				<DownloadIcon size={18} />
				<span class="sd-label">{t("sidebar.noDownloads")}</span>
			{/if}
		</span>
		<ChevronDownIcon
			size={16}
			class={"sd-chevron" + (open ? " open" : "")}
		/>
	</button>
	{#if open}
		<div class="sd-body" transition:slide={{ duration: 150 }}>
			{#if downloads.size === 0}
				<div class="sd-empty">{t("sidebar.noDownloadDesc")}</div>
			{:else}
				{#each [...downloads.values()] as item (item.version)}
					{@const overall = pct(item.segs)}
					<div
						class="sd-item"
						class:done={item.done}
						class:error={item.error}
					>
						<div class="sd-item-header">
							<span class="sd-item-left">
								{#if item.error}
									<span class="sd-error-icon">!</span>
								{:else if item.done}
									<CheckIcon size={8} />
								{:else}
									<span class="sd-spinner-sm"></span>
								{/if}
								<div class="sd-version-wrap">
									<span class="sd-version"
										>{item.version === "mods"
											? t("sidebar.downloadingMods")
											: item.version}</span
									>
									{#if !item.done && !item.error && item.activeType}
										<span class="sd-status-label"
											>{statusLabel(
												item.activeType,
											)}</span
										>
									{/if}
								</div>
							</span>
							{#if item.error}
								<span class="sd-pct error"
									>{t("sidebar.failed")}</span
								>
							{:else}
								<span class="sd-pct" class:done={item.done}
									>{overall}%</span
								>
							{/if}
						</div>
						{#if item.error}
							<div class="sd-error-msg">{item.error}</div>
						{:else if item.retryMsg}
							<div class="sd-retry-msg">{item.retryMsg}</div>
							<div class="sd-progress-track">
								<div
									class="sd-progress-fill"
									class:done={item.done}
									style:width="{overall}%"
								></div>
							</div>
						{:else}
							<div class="sd-progress-track">
								<div
									class="sd-progress-fill"
									class:done={item.done}
									style:width="{overall}%"
								></div>
							</div>
						{/if}
					</div>
				{/each}
			{/if}
		</div>
	{/if}
</div>

<style>
	.sd-root {
		border-bottom: 1px solid var(--border-color);
	}

	.sd-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		background: none;
		border: none;
		color: inherit;
		padding: 8px 10px;
		cursor: pointer;
		border-radius: var(--border-radius-sm);
		transition: background 0.15s ease;
		user-select: none;
	}

	.sd-header:hover {
		background: rgba(255, 255, 255, 0.03);
	}

	.sd-header.expanded {
		border-bottom: 1px solid var(--border);
		border-radius: var(--border-radius-sm) var(--border-radius-sm) 0 0;
	}

	.sd-header-left {
		display: flex;
		align-items: center;
		gap: 7px;
		min-width: 0;
		flex: 1;
	}

	.sd-label {
		font-size: 0.75rem;
		font-weight: 700;
		color: var(--text-primary);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
	}

	.sd-spinner {
		width: 10px;
		height: 10px;
		border: 1.5px solid var(--border);
		border-top-color: var(--text-muted);
		border-radius: 50%;
		animation: sd-spin 0.7s linear infinite;
		will-change: transform;
		flex-shrink: 0;
	}

	.sd-spinner-sm {
		width: 8px;
		height: 8px;
		border: 1.5px solid var(--border);
		border-top-color: var(--text-muted);
		border-radius: 50%;
		animation: sd-spin 0.7s linear infinite;
		will-change: transform;
		flex-shrink: 0;
		display: block;
	}

	@keyframes sd-spin {
		to {
			transform: rotate(360deg);
		}
	}

	:global(.sd-chevron) {
		color: var(--accent);
		flex-shrink: 0;
		transition: transform 0.2s;
	}

	:global(.sd-chevron.open) {
		transform: rotate(180deg);
	}

	.sd-body {
		overflow: hidden;
	}

	.sd-item {
		padding: 8px 10px;
		border-bottom: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.sd-item:last-child {
		border-bottom: none;
	}

	.sd-item-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.sd-item-left {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		flex: 1;
	}

	.sd-version-wrap {
		display: flex;
		flex-direction: column;
		gap: 1px;
		min-width: 0;
	}

	.sd-version {
		font-size: 0.72rem;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.sd-status-label {
		font-size: 0.6rem;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.sd-pct {
		font-size: 0.65rem;
		font-weight: 700;
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.sd-pct.done {
		color: var(--color-success);
	}

	.sd-pct.error {
		color: var(--color-error);
	}

	.sd-error-icon {
		width: 8px;
		height: 8px;
		background: var(--color-error);
		color: white;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 6px;
		font-weight: 900;
		line-height: 1;
		flex-shrink: 0;
	}

	.sd-error-msg {
		font-size: 0.65rem;
		color: var(--color-error);
		word-break: break-word;
		line-height: 1.3;
	}

	.sd-retry-msg {
		font-size: 0.65rem;
		color: var(--color-warning);
		word-break: break-word;
		line-height: 1.3;
	}

	.sd-item.error {
		background: rgba(220, 38, 38, 0.05);
	}

	.sd-progress-track {
		width: 100%;
		height: 3px;
		background: var(--bg-input);
		border-radius: 2px;
		overflow: hidden;
	}

	.sd-progress-fill {
		height: 100%;
		background: var(--accent);
		border-radius: 2px;
		transition: width 0.35s cubic-bezier(0.4, 0, 0.2, 1);
	}

	.sd-progress-fill.done {
		background: var(--color-success);
	}

	.sd-empty {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 130px;
		padding: 16px 10px;
		text-align: center;
		font-size: 0.68rem;
		color: var(--text-muted);
		line-height: 1.4;
	}

	@media (max-width: 650px) {
		.sd-header {
			justify-content: center;
			padding: 8px 4px;
		}

		:global(.sd-header .sd-chevron) {
			display: none;
		}

		.sd-item {
			padding: 6px 4px;
			align-items: center;
		}

		.sd-item-header {
			justify-content: center;
		}

		.sd-item .sd-spinner-sm {
			width: 10px;
			height: 10px;
		}
	}
</style>
