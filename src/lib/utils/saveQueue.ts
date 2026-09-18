/** One write in flight and at most one trailing write, always reading the latest
 * state when it starts. A short batching window never postpones a write forever. */
export function createSaveQueue(save: () => Promise<void>) {
	type Batch = {
		promise: Promise<void>;
		resolve: () => void;
		reject: (error: unknown) => void;
	};
	let pending: Batch | undefined;
	let running: Batch | undefined;
	let timer: ReturnType<typeof setTimeout> | undefined;

	function run() {
		if (running || !pending) return;
		clearTimeout(timer);
		timer = undefined;
		const batch = pending;
		pending = undefined;
		running = batch;
		void Promise.resolve()
			.then(save)
			.then(
				() => finish(batch),
				(error: unknown) => finish(batch, { error }),
			);
	}

	function finish(batch: Batch, failure?: { error: unknown }) {
		running = undefined;
		if (failure) batch.reject(failure.error);
		else batch.resolve();
		run();
	}

	return {
		get busy() {
			return !!(pending || running);
		},
		request(delay = 0): Promise<void> {
			if (!pending) {
				let resolve!: () => void;
				let reject!: (error: unknown) => void;
				const promise = new Promise<void>((yes, no) => {
					resolve = yes;
					reject = no;
				});
				pending = { promise, resolve, reject };
				if (!running && delay > 0) timer = setTimeout(run, delay);
			}
			const promise = pending.promise;
			if (delay === 0) run();
			return promise;
		},
		flush(): Promise<void> {
			const promise =
				pending?.promise ?? running?.promise ?? Promise.resolve();
			run();
			return promise;
		},
	};
}
