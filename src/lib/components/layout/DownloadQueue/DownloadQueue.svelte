<script lang="ts">
	import { onMount } from "svelte";
	import { slide } from "svelte/transition";
	import { SvelteMap } from "svelte/reactivity";
	import { listen } from "@tauri-apps/api/event";
	import { getDownloadQueue } from "$lib/api/cubicApi";
	import type { AppEvent } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import DownloadQueueHeader from "./DownloadQueueHeader.svelte";
	import DownloadQueueItem from "./DownloadQueueItem.svelte";

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
	function normalizeStage(stage: string): SegKey {
		const map: Record<string, SegKey> = {
			library: "Library",
			asset: "Asset",
			native: "Native",
			client: "Client",
			verifying: "Verifying",
			generic: "Generic",
			processing: "Processing",
			jre: "Jre",
			extracting: "Jre",
			resolving: "Generic",
			mc: "Generic",
		};
		return map[stage.toLowerCase()] ?? "Generic";
	}

	let open = $state(false);
	let counts = $derived.by(() => {
		let active = 0,
			done = 0;
		for (const d of downloads.values()) {
			if (d.done) done++;
			else active++;
		}
		return { active, done };
	});

	function pct(item: DlItem): number {
		if (item.done) return 100;
		const active = item.activeType;
		if (active) {
			const s = item.segs[active];
			if (s.total > 0) {
				return Math.round((s.current / s.total) * 100);
			}
		}
		// Fallback when no active segment is known.
		const totalAll = SEGS.reduce((a, k) => a + item.segs[k].total, 0);
		const curAll = SEGS.reduce((a, k) => a + item.segs[k].current, 0);
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
						});
						open = true;
					}
					break;
				}
			case "DProgress": {
				const {
					version,
					stage,
					item_current,
					item_total,
					bytes_current,
					bytes_total,
				} = p.data;
				const existing = downloads.get(version) ?? {
					version,
					activeType: null,
					segs: emptySegs(),
					done: false,
					error: null,
				};
				const key = normalizeStage(stage);
				const useBytes = bytes_total > 0;
				const current = useBytes ? bytes_current : item_current;
				const total = useBytes ? bytes_total : item_total;
				downloads.set(version, {
					...existing,
					segs: { ...existing.segs, [key]: { current, total } },
					activeType: key,
					done: false,
				});
				break;
			}
			case "DStage": {
				const { version, stage } = p.data;
				const existing = downloads.get(version);
				if (existing) {
					downloads.set(version, {
						...existing,
						activeType: normalizeStage(stage),
					});
				}
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
				case "DError": {
					const { version, message } = p.data;
					const item = downloads.get(version);
					if (item) {
						downloads.set(version, {
							...item,
							done: true,
							activeType: null,
							error: message,
						});
					} else {
						downloads.set(version, {
							version,
							activeType: null,
							segs: emptySegs(),
							done: true,
							error: message,
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
	<DownloadQueueHeader
		bind:open
		activeCount={counts.active}
		doneCount={counts.done}
	/>
	{#if open}
		<div class="sd-body" transition:slide={{ duration: 150 }}>
			{#if downloads.size === 0}
				<div class="sd-empty">{t("sidebar.noDownloadDesc")}</div>
			{:else}
				{#each [...downloads.values()] as item (item.version)}
					{@const overall = pct(item)}
					{@const label =
						!item.done && !item.error && item.activeType
							? statusLabel(item.activeType)
							: null}
					<DownloadQueueItem
						version={item.version}
						{overall}
						done={item.done}
						error={item.error}
						statusLabel={label}
					/>
				{/each}
			{/if}
		</div>
	{/if}
</div>

<style>
	.sd-root {
		border-bottom: 1px solid var(--border-color);
	}

	.sd-body {
		overflow: hidden;
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
</style>
