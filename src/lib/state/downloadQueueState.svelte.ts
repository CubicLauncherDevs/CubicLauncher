import { SvelteMap, SvelteSet } from "svelte/reactivity";
import { getDownloadQueue } from "$lib/api/cubicApi";
import { onAppEvent } from "$lib/api/launcherService";
import { t } from "$lib/i18n";

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

export interface DownloadQueueItem {
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

const downloads = new SvelteMap<string, DownloadQueueItem>();
const removalTimers = new SvelteSet<ReturnType<typeof setTimeout>>();

let initialized = false;
let unsubs: (() => void)[] = [];

export { downloads };

export function getActiveDownloadCount(): number {
	return [...downloads.values()].filter((d) => !d.done && !d.error).length;
}

export function getDoneDownloadCount(): number {
	return [...downloads.values()].filter((d) => d.done && !d.error).length;
}

export function hasDownloads(): boolean {
	return downloads.size > 0;
}

export function getDownloadQueueItems(): DownloadQueueItem[] {
	return [...downloads.values()];
}

function removeDownload(version: string, delay: number): void {
	const t = setTimeout(() => {
		removalTimers.delete(t);
		downloads.delete(version);
	}, delay);
	removalTimers.add(t);
}

export function initDownloadQueueState(): void {
	if (initialized) return;
	initialized = true;

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
	});

	unsubs.push(
		onAppEvent("DEnqueue", (payload) => {
			const { version } = payload.data as { version: string };
			if (!downloads.has(version)) {
				downloads.set(version, {
					version,
					activeType: null,
					segs: emptySegs(),
					done: false,
					error: null,
				});
			}
		}),
	);

	unsubs.push(
		onAppEvent("DProgress", (payload) => {
			const {
				version,
				stage,
				item_current,
				item_total,
				bytes_current,
				bytes_total,
			} = payload.data as {
				version: string;
				stage: string;
				item_current: number;
				item_total: number;
				bytes_current: number;
				bytes_total: number;
			};
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
				error: null,
			});
		}),
	);

	unsubs.push(
		onAppEvent("DStage", (payload) => {
			const { version, stage } = payload.data as {
				version: string;
				stage: string;
			};
			const existing = downloads.get(version);
			if (existing) {
				downloads.set(version, {
					...existing,
					activeType: normalizeStage(stage),
				});
			}
		}),
	);

	unsubs.push(
		onAppEvent("DFinish", (payload) => {
			const { version } = payload.data as { version: string };
			const item = downloads.get(version);
			if (item) {
				downloads.set(version, {
					...item,
					done: true,
					activeType: null,
				});
			}
			removeDownload(version, 4000);
		}),
	);

	unsubs.push(
		onAppEvent("DError", (payload) => {
			const { version, message } = payload.data as {
				version: string;
				message?: string;
			};
			const item = downloads.get(version);
			if (item) {
				downloads.set(version, {
					...item,
					done: true,
					activeType: null,
					error: message ?? null,
				});
			} else {
				downloads.set(version, {
					version,
					activeType: null,
					segs: emptySegs(),
					done: true,
					error: message ?? null,
				});
			}
			removeDownload(version, 8000);
		}),
	);
}

export function destroyDownloadQueueState(): void {
	for (const unsub of unsubs) unsub();
	unsubs = [];
	for (const t of removalTimers) clearTimeout(t);
	removalTimers.clear();
	downloads.clear();
	initialized = false;
}

export function getOverallPct(item: DownloadQueueItem): number {
	if (item.done) return 100;
	const active = item.activeType;
	if (active) {
		const s = item.segs[active];
		if (s.total > 0) {
			return Math.round((s.current / s.total) * 100);
		}
	}
	const totalAll = SEGS.reduce((a, k) => a + item.segs[k].total, 0);
	const curAll = SEGS.reduce((a, k) => a + item.segs[k].current, 0);
	return totalAll > 0 ? Math.round((curAll / totalAll) * 100) : 0;
}

export function getStatusLabel(activeType: SegKey | null): string {
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
