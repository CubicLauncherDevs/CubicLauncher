<script lang="ts">
	import { listen } from "@tauri-apps/api/event";
	import { onMount } from "svelte";
	import type { AppEvent } from "$lib/types/types";
	import { getDownloadQueue } from "$lib/api/cubicApi";
	import { SvelteMap } from "svelte/reactivity";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";
	import DownloadIcon from "$lib/icons/DownloadIcon.svelte";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";

	type SegmentKey = "Library" | "Asset" | "Native" | "Client";
	const SEGMENTS: SegmentKey[] = ["Library", "Asset", "Native", "Client"];

	const SEGMENT_COLORS: Record<SegmentKey, string> = {
		Library: "#4ade80",
		Asset: "#60a5fa",
		Native: "#f59e0b",
		Client: "#a78bfa",
	};

	const SEGMENT_LABELS: Record<SegmentKey, string> = {
		Library: "LIB",
		Asset: "ASSET",
		Native: "NAT",
		Client: "CLIENT",
	};

	interface SegmentProgress {
		current: number;
		total: number;
	}

	interface DownloadItem {
		version: string;
		activeType: SegmentKey | null;
		segments: Record<SegmentKey, SegmentProgress>;
		done: boolean;
	}

	function emptySegments(): Record<SegmentKey, SegmentProgress> {
		return {
			Library: { current: 0, total: 0 },
			Asset: { current: 0, total: 0 },
			Native: { current: 0, total: 0 },
			Client: { current: 0, total: 0 },
		};
	}

	let downloads = new SvelteMap<string, DownloadItem>();
	let expanded = $state(false);
	let activeCount = $derived(
		[...downloads.values()].filter((d) => !d.done).length,
	);
	let doneCount = $derived(
		[...downloads.values()].filter((d) => d.done).length,
	);

	onMount(() => {
		getDownloadQueue().then((queue) => {
			for (const item of queue) {
				if (!downloads.has(item.version)) {
					downloads.set(item.version, {
						version: item.version,
						activeType: null,
						segments: emptySegments(),
						done: item.status === "done",
					});
				}
			}
			if (queue.length > 0) {
				expanded = true;
			}
		});

		const unlisten = listen<AppEvent>("app-event", (event) => {
			const payload = event.payload;
			switch (payload.type) {
				case "DProgress": {
					const { version, current, total, d_type } = payload.data;
					const isNew = !downloads.has(version);
					const existing = downloads.get(version) ?? {
						version,
						activeType: null,
						segments: emptySegments(),
						done: false,
					};
					const key = d_type as SegmentKey;
					existing.segments[key] = { current, total };
					existing.activeType = key;
					existing.done = false;
					downloads.set(version, { ...existing });
					if (isNew) expanded = true;
					break;
				}
				case "DFinish": {
					const { version } = payload.data;
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

	function toggle() {
		expanded = !expanded;
	}

	function getSegmentPct(item: DownloadItem, key: SegmentKey): number {
		if (item.done) return 100;
		const s = item.segments[key];
		if (!s.total) return 0;
		return Math.round((s.current / s.total) * 100);
	}

	function getOverallPct(item: DownloadItem): number {
		if (item.done) return 100;
		const totalAll = SEGMENTS.reduce(
			(a, k) => a + item.segments[k].total,
			0,
		);
		const currentAll = SEGMENTS.reduce(
			(a, k) => a + item.segments[k].current,
			0,
		);
		return totalAll > 0 ? Math.round((currentAll / totalAll) * 100) : 0;
	}
</script>

<div class="dl-tray" class:expanded>
	<button type="button" class="dl-tray-tab" onclick={toggle}>
		<div class="dl-tray-tab-left">
			{#if activeCount > 0}
				<span class="dl-tray-spinner"></span>
				<span>{activeCount} descargando</span>
			{:else if doneCount > 0}
				<CheckIcon size={13} />
				<span>{doneCount} completadas</span>
			{:else}
				<DownloadIcon size={13} />
				<span>Sin descargas</span>
			{/if}
		</div>
		<ChevronDownIcon size={11} class="dl-tray-chevron" />
	</button>

	<div class="dl-tray-body">
		{#if downloads.size === 0}
			<div class="dl-tray-empty">
				<DownloadIcon size={26} />
				<span class="dl-tray-empty-title">No hay descargas activas</span
				>
				<span class="dl-tray-empty-sub"
					>Las versiones que descargues aparecerán aquí</span
				>
			</div>
		{:else}
			{#each [...downloads.values()] as item (item.version)}
				{@const overall = getOverallPct(item)}
				<div class="dl-tray-item" class:done={item.done}>
					<div class="dl-tray-item-header">
						<div class="dl-tray-item-left">
							{#if item.done}
								<CheckIcon size={10} />
							{:else}
								<span class="dl-tray-spinner-sm"></span>
							{/if}
							<span class="dl-tray-version">{item.version}</span>
						</div>
						<span class="dl-tray-pct" class:done={item.done}
							>{overall}%</span
						>
					</div>

					<div class="dl-tray-segments">
						{#each SEGMENTS as key (key)}
							{@const pct = getSegmentPct(item, key)}
							{@const color = SEGMENT_COLORS[key]}
							{@const isActive =
								item.activeType === key && !item.done}
							<div class="dl-tray-seg">
								<div class="dl-tray-seg-header">
									<span
										class="dl-tray-seg-label"
										style:color={pct > 0 || isActive
											? color
											: "var(--text-muted)"}
									>
										{SEGMENT_LABELS[key]}
									</span>
									<span
										class="dl-tray-seg-pct"
										style:color={pct > 0
											? color
											: "var(--text-muted)"}
									>
										{pct}%
									</span>
								</div>
								<div class="dl-tray-seg-track">
									<div
										class="dl-tray-seg-fill"
										class:active={isActive}
										style:width="{pct}%"
										style:background={color}
									></div>
								</div>
							</div>
						{/each}
					</div>

					<div class="dl-tray-overall-track">
						<div
							class="dl-tray-overall-fill"
							style:width="{overall}%"
						></div>
					</div>
				</div>
			{/each}
		{/if}
	</div>
</div>

<style>
	.dl-tray {
		position: fixed;
		top: 0;
		right: 0;
		z-index: 50;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
	}

	.dl-tray-tab {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 9px 28px 9px 18px;
		background: var(--bg-card);
		border: none;
		border-bottom: 1px solid var(--border-color);
		border-left: 1px solid var(--border-color);
		color: var(--text-muted);
		cursor: pointer;
		font-family: "Cantarell", system-ui, sans-serif;
		font-size: 0.75rem;
		font-weight: 600;
		user-select: none;
		letter-spacing: 0.3px;
		clip-path: polygon(0% 0%, 100% 0%, 100% 100%, 22px 100%);
		transition:
			background 0.15s ease,
			color 0.15s ease;
		min-width: 200px;
	}

	.dl-tray-tab:hover {
		background: var(--bg-item-active);
		color: var(--text-secondary);
	}

	.dl-tray-tab-left {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
	}

	.dl-tray-spinner {
		width: 11px;
		height: 11px;
		border: 1.5px solid var(--border-color);
		border-top-color: var(--text-muted);
		border-radius: 50%;
		animation: dl-spin 0.7s linear infinite;
		will-change: transform;
		flex-shrink: 0;
	}

	.dl-tray-spinner-sm {
		width: 9px;
		height: 9px;
		border: 1.5px solid var(--border-color);
		border-top-color: var(--text-muted);
		border-radius: 50%;
		animation: dl-spin 0.7s linear infinite;
		will-change: transform;
		flex-shrink: 0;
	}

	@keyframes dl-spin {
		to {
			transform: rotate(360deg);
		}
	}

	.dl-tray-body {
		width: 340px;
		max-height: 0;
		overflow: hidden;
		background: var(--bg-main);
		border-bottom: 1px solid var(--border-color);
		border-left: 1px solid var(--border-color);
		transition:
			max-height 0.3s cubic-bezier(0.4, 0, 0.2, 1),
			opacity 0.2s ease;
		opacity: 0;
		pointer-events: none;
	}

	.dl-tray.expanded .dl-tray-body {
		max-height: 360px;
		opacity: 1;
		overflow-y: auto;
		pointer-events: all;
	}

	.dl-tray-body:global(::-webkit-scrollbar) {
		width: 3px;
	}
	.dl-tray-body:global(::-webkit-scrollbar-thumb) {
		background: var(--border-color);
		border-radius: 3px;
	}

	.dl-tray-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 6px;
		padding: 28px 20px;
		text-align: center;
	}

	.dl-tray-empty-title {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--text-muted);
	}
	.dl-tray-empty-sub {
		font-size: 0.68rem;
		color: var(--text-muted);
		line-height: 1.4;
		max-width: 160px;
	}

	.dl-tray-item {
		padding: 10px 14px 12px;
		display: flex;
		flex-direction: column;
		gap: 8px;
		border-bottom: 1px solid var(--border-color);
	}

	.dl-tray-item:last-child {
		border-bottom: none;
	}

	.dl-tray-item-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.dl-tray-item-left {
		display: flex;
		align-items: center;
		gap: 7px;
	}

	.dl-tray-version {
		font-size: 0.8rem;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 220px;
	}

	.dl-tray-pct {
		font-size: 0.7rem;
		font-weight: 700;
		color: var(--text-muted);
		flex-shrink: 0;
		transition: color 0.3s;
	}

	.dl-tray-pct.done {
		color: var(--color-success);
	}

	.dl-tray-segments {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 6px;
	}

	.dl-tray-seg {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.dl-tray-seg-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.dl-tray-seg-label {
		font-size: 0.58rem;
		font-weight: 700;
		letter-spacing: 0.6px;
		transition: color 0.3s;
	}

	.dl-tray-seg-pct {
		font-size: 0.58rem;
		font-weight: 700;
		transition: color 0.3s;
	}

	.dl-tray-seg-track {
		height: 3px;
		background: var(--bg-input);
		border-radius: 2px;
		overflow: hidden;
	}

	.dl-tray-seg-fill {
		height: 100%;
		border-radius: 2px;
		transition: width 0.35s cubic-bezier(0.4, 0, 0.2, 1);
		opacity: 0.75;
	}

	.dl-tray-seg-fill.active {
		opacity: 1;
	}
	.dl-tray-item.done .dl-tray-seg-fill {
		opacity: 0.5;
	}

	.dl-tray-overall-track {
		width: 100%;
		height: 2px;
		background: var(--bg-input);
		border-radius: 1px;
		overflow: hidden;
		margin-top: 2px;
	}

	.dl-tray-overall-fill {
		height: 100%;
		background: var(--text-muted);
		border-radius: 1px;
		transition: width 0.35s cubic-bezier(0.4, 0, 0.2, 1);
	}
</style>
