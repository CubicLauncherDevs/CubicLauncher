<script lang="ts">
	import { onMount } from "svelte";
	import { slide } from "svelte/transition";
	import { SvelteMap } from "svelte/reactivity";
	import { listen } from "@tauri-apps/api/event";
	import { getDownloadQueue } from "$lib/api/cubicApi";
	import type { AppEvent } from "$lib/types/types";
	import { t } from "$lib/i18n";

	type SegKey = "Library" | "Asset" | "Native" | "Client";
	const SEGS: SegKey[] = ["Library", "Asset", "Native", "Client"];

	interface SegProg {
		current: number;
		total: number;
	}
	interface DlItem {
		version: string;
		activeType: SegKey | null;
		segs: Record<SegKey, SegProg>;
		done: boolean;
	}

	function emptySegs(): Record<SegKey, SegProg> {
		return {
			Library: { current: 0, total: 0 },
			Asset: { current: 0, total: 0 },
			Native: { current: 0, total: 0 },
			Client: { current: 0, total: 0 },
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

	onMount(() => {
		getDownloadQueue().then((queue) => {
			for (const item of queue) {
				if (!downloads.has(item.version)) {
					downloads.set(item.version, {
						version: item.version,
						activeType: null,
						segs: emptySegs(),
						done: item.status === "done",
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
						});
					}
					setTimeout(() => {
						downloads.delete(version);
					}, 4000);
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
				<span class="sd-label">{activeCount} {t("sidebar.downloading")}</span>
			{:else if doneCount > 0}
				<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--color-success)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12" /></svg>
				<span class="sd-label">{doneCount} {t("sidebar.completed")}</span>
			{:else}
				<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<path d="M12 15V3m0 12l-4-4m4 4l4-4M2 17l.621 2.485A2 2 0 0 0 4.561 21h14.878a2 2 0 0 0 1.94-1.515L22 17" />
				</svg>
				<span class="sd-label">{t("sidebar.noDownloads")}</span>
			{/if}
		</span>
		<svg class="sd-chevron" class:open={open} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6" /></svg>
	</button>
	{#if open}
		<div class="sd-body" transition:slide={{ duration: 150 }}>
			{#if downloads.size === 0}
				<div class="sd-empty">{t("sidebar.noDownloadDesc")}</div>
			{:else}
				{#each [...downloads.values()] as item (item.version)}
					{@const overall = pct(item.segs)}
					<div class="sd-item" class:done={item.done}>
						<div class="sd-item-header">
							<span class="sd-item-left">
								{#if item.done}
									<svg width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="var(--color-success)" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12" /></svg>
								{:else}
									<span class="sd-spinner-sm"></span>
								{/if}
								<span class="sd-version">{item.version}</span>
							</span>
							<span class="sd-pct" class:done={item.done}>{overall}%</span>
						</div>
						<div class="sd-progress-track">
							<div class="sd-progress-fill" class:done={item.done} style:width="{overall}%"></div>
						</div>
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
		font-family: "Cantarell", system-ui, sans-serif;
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
		flex-shrink: 0;
	}

	.sd-spinner-sm {
		width: 8px;
		height: 8px;
		border: 1.5px solid var(--border);
		border-top-color: var(--text-muted);
		border-radius: 50%;
		animation: sd-spin 0.7s linear infinite;
		flex-shrink: 0;
		display: block;
	}

	@keyframes sd-spin {
		to { transform: rotate(360deg); }
	}

	.sd-chevron {
		color: var(--accent);
		flex-shrink: 0;
		transition: transform 0.2s;
	}

	.sd-chevron.open {
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

	.sd-version {
		font-size: 0.72rem;
		font-weight: 700;
		color: var(--text-primary);
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

		.sd-header .sd-chevron {
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
