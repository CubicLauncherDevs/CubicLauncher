import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { HistoryLogLine } from "$lib/components/log/logState.svelte";
import {
	CONSOLE_HISTORY_MAX,
	type RawLogEvent,
} from "$lib/components/log/logHelpers";

/** Subscribe before reading history; merge the overlapping snapshot by line ID. */
export function openLogStream(
	instanceId: string,
	limit: number,
	onHistory: (lines: HistoryLogLine[]) => void,
	onBatch: (lines: RawLogEvent[]) => void,
	onError: (error: unknown) => void,
): () => void {
	let disposed = false;
	let ready = false;
	let unlisten: (() => void) | undefined;
	let pending: RawLogEvent[] = [];
	let lastId = -1;
	const maximum = Math.max(100, Math.min(limit, CONSOLE_HISTORY_MAX));
	function deliver(lines: RawLogEvent[]) {
		const fresh = lines.filter((line) => line.id > lastId);
		if (!fresh.length) return;
		lastId = fresh[fresh.length - 1].id;
		onBatch(fresh);
	}
	void (async () => {
		const off = await listen<{ id: string; lines: RawLogEvent[] }>(
			"instance-log-batch",
			(event) => {
				if (disposed || event.payload.id !== instanceId) return;
				if (ready) deliver(event.payload.lines);
				else
					pending = pending
						.concat(event.payload.lines)
						.slice(-maximum);
			},
			{ target: { kind: "WebviewWindow", label: `log-${instanceId}` } },
		);
		if (disposed) {
			off();
			return;
		}
		unlisten = off;
		const history = await invoke<HistoryLogLine[]>("get_log_history_cmd", {
			instanceId,
			limit: maximum,
		});
		if (disposed) return;
		const merged = new Map(history.map((line) => [line.id, line]));
		for (const line of pending)
			merged.set(line.id, { ...line, text: line.line });
		const lines = [...merged.values()]
			.sort((a, b) => a.id - b.id)
			.slice(-maximum);
		pending = [];
		lastId = lines.at(-1)?.id ?? -1;
		onHistory(lines);
		ready = true;
	})().catch((error) => {
		if (disposed) return;
		ready = true;
		deliver(pending);
		pending = [];
		onError(error);
	});
	return () => {
		if (disposed) return;
		disposed = true;
		unlisten?.();
		pending = [];
	};
}

export function subscribeLogPreview(
	instanceId: string,
	onLine: (line: string) => void,
): () => void {
	const subscriptionId = crypto.randomUUID();
	let disposed = false;
	let subscribed = false;
	let unlisten: (() => void) | undefined;
	const release = () =>
		invoke("set_log_preview", { subscriptionId, instanceId: null }).catch(
			(error) => console.warn("Log preview cleanup failed:", error),
		);
	void (async () => {
		const off = await listen<{ id: string; line: string }>(
			"instance-log-preview",
			(event) => {
				if (!disposed && event.payload.id === instanceId)
					onLine(event.payload.line);
			},
			{ target: { kind: "WebviewWindow", label: "main" } },
		);
		if (disposed) {
			off();
			return;
		}
		unlisten = off;
		await invoke("set_log_preview", { subscriptionId, instanceId });
		if (disposed) {
			await release();
			return;
		}
		subscribed = true;
	})().catch((error) => {
		unlisten?.();
		unlisten = undefined;
		if (!disposed) console.warn("Log preview subscription failed:", error);
	});
	return () => {
		if (disposed) return;
		disposed = true;
		unlisten?.();
		if (subscribed) {
			subscribed = false;
			void release();
		}
	};
}
