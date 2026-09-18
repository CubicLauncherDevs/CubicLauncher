import { SvelteMap } from "svelte/reactivity";

/** Only retain the current viewport. One IPC request in flight, no stale queue. */
export function createScreenshotThumbnails(
	load: (path: string) => Promise<string | null>,
) {
	const entries = new SvelteMap<string, string | null>();
	let wanted: string[] = [];
	let generation = 0;
	let loading = false;
	let disposed = false;
	let timer: ReturnType<typeof setTimeout> | undefined;

	function schedule(delay = 60) {
		clearTimeout(timer);
		if (
			!disposed &&
			!loading &&
			wanted.some((path) => !entries.has(path))
		) {
			timer = setTimeout(() => void next(), delay);
		}
	}

	async function next() {
		timer = undefined;
		if (disposed || loading) return;
		const path = wanted.find((path) => !entries.has(path));
		if (!path) return;
		const token = generation;
		loading = true;
		let image: string | null = null;
		try {
			image = await load(path);
		} catch {
			// A missing/corrupt file keeps its placeholder; the original is still accessible.
		} finally {
			if (!disposed && token === generation && wanted.includes(path))
				entries.set(path, image);
			loading = false;
			schedule(0);
		}
	}

	return {
		get: (path: string) => entries.get(path),
		request(paths: string[]) {
			if (disposed) return;
			wanted = paths;
			const keep = new Set(paths);
			for (const path of entries.keys())
				if (!keep.has(path)) entries.delete(path);
			schedule();
		},
		reset() {
			generation++;
			wanted = [];
			entries.clear();
			clearTimeout(timer);
		},
		destroy() {
			disposed = true;
			this.reset();
		},
		get count() {
			return entries.size;
		},
	};
}
