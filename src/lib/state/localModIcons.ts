import { SvelteMap } from "svelte/reactivity";
import type { ModIconRequest, ModIconResult } from "$lib/api/cubicApi";
import type { MarketProject } from "$lib/types/market";

const MAX_ENTRIES = 160;
const MAX_BYTES = 8 * 1024 * 1024;
const BATCH_SIZE = 24;

/** One debounced IPC request at a time, for the current viewport only. */
export function createLocalModIcons(
	load: (files: ModIconRequest[]) => Promise<ModIconResult[]>,
) {
	const entries = new SvelteMap<string, ModIconResult>();
	let known = new Map<string, string>();
	let wanted: ModIconRequest[] = [];
	const attempted = new Set<string>();
	let bytes = 0;
	let loading = false;
	let disposed = false;
	let timer: ReturnType<typeof setTimeout> | undefined;
	const key = (file: ModIconRequest) => `${file.filename}\n${file.revision}`;
	const size = (file: ModIconResult) => (file.icon?.length ?? 0) * 2;

	function remove(filename: string) {
		const old = entries.get(filename);
		if (old) bytes -= size(old);
		entries.delete(filename);
	}
	function store(file: ModIconResult) {
		remove(file.filename);
		// Oversized logos must not consume the entire viewport cache.
		const value = size(file) > MAX_BYTES ? { ...file, icon: null } : file;
		while (entries.size >= MAX_ENTRIES || bytes + size(value) > MAX_BYTES) {
			const oldest = entries.keys().next().value;
			if (oldest === undefined) break;
			remove(oldest);
		}
		bytes += size(value);
		entries.set(file.filename, value);
	}
	function schedule() {
		clearTimeout(timer);
		if (!disposed && !loading && wanted.length)
			timer = setTimeout(() => void flush(), 60);
	}
	async function flush() {
		timer = undefined;
		if (disposed || loading) return;
		const batch = wanted
			.filter(
				(file) =>
					entries.get(file.filename)?.revision !== file.revision &&
					!attempted.has(key(file)),
			)
			.slice(0, BATCH_SIZE);
		if (!batch.length) return;
		for (const file of batch) attempted.add(key(file));
		loading = true;
		try {
			const result = await load(batch);
			if (disposed) return;
			for (const file of result) {
				if (
					known.get(file.filename) === file.revision &&
					batch.some((requested) => key(requested) === key(file))
				)
					store(file);
			}
		} catch (error) {
			if (!disposed) console.warn("Mod icon batch failed:", error);
		} finally {
			loading = false;
			schedule();
		}
	}
	return {
		get(project: MarketProject): string | null {
			if (project.icon) return project.icon;
			const mod = project.installed;
			if (!mod?.icon_revision) return null;
			const cached = entries.get(mod.filename);
			return cached?.revision === mod.icon_revision ? cached.icon : null;
		},
		sync(projects: MarketProject[]) {
			known = new Map(
				projects.flatMap((project) =>
					project.installed?.icon_revision
						? [
								[
									project.installed.filename,
									project.installed.icon_revision,
								] as const,
							]
						: [],
				),
			);
			for (const [filename, file] of entries)
				if (known.get(filename) !== file.revision) remove(filename);
			attempted.clear();
			wanted = wanted.flatMap((file) => {
				const revision = known.get(file.filename);
				return revision ? [{ filename: file.filename, revision }] : [];
			});
			schedule();
		},
		request(projects: MarketProject[]) {
			if (disposed) return;
			const next = projects.flatMap((project) =>
				project.installed?.icon_revision && !project.icon
					? [
							{
								filename: project.installed.filename,
								revision: project.installed.icon_revision,
							},
						]
					: [],
			);
			if (
				wanted.length === next.length &&
				wanted.every((file, i) => key(file) === key(next[i]))
			)
				return;
			wanted = next;
			attempted.clear();
			schedule();
		},
		get count() {
			return entries.size;
		},
		get retainedBytes() {
			return bytes;
		},
		destroy() {
			disposed = true;
			clearTimeout(timer);
			wanted = [];
			known.clear();
			attempted.clear();
			entries.clear();
			bytes = 0;
		},
	};
}
