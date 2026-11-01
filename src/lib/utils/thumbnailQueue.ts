/**
 * Turnos para dibujar imágenes con una sola escena: la crea cuando hace falta, la suelta cuando
 * no queda trabajo y recuerda lo ya dibujado.
 */

/** Los brazos, con los nombres de skin3d. */
export type FigureModel = "default" | "slim";

export interface ThumbnailPainter {
	paint(url: string, model: FigureModel): Promise<Blob>;
	release(): void;
}

export interface ThumbnailJob {
	promise: Promise<string>;
	/** Quien la pedía ya no la necesita; si nadie más la espera, no se dibuja. */
	cancel(): void;
}

export interface ThumbnailQueueOptions {
	/** Cuántas imágenes se guardan; al pasarse se olvidan las que llevan más tiempo sin usarse. */
	most?: number;
	/** Cuánto sigue viva la escena tras la última imagen, por si llegan más. */
	releaseAfterMs?: number;
	makeUrl?: (blob: Blob) => string;
	dropUrl?: (url: string) => void;
}

interface Pending {
	url: string;
	model: FigureModel;
	waiting: number;
	promise: Promise<string>;
	resolve: (url: string) => void;
	reject: (reason: Error) => void;
}

export const THUMBNAIL_CANCELLED = "thumbnail-cancelled";

export function createThumbnailQueue(
	makePainter: () => Promise<ThumbnailPainter>,
	options: ThumbnailQueueOptions = {},
) {
	const most = options.most ?? 128;
	const releaseAfterMs = options.releaseAfterMs ?? 1500;
	const makeUrl = options.makeUrl ?? ((blob) => URL.createObjectURL(blob));
	const dropUrl = options.dropUrl ?? ((url) => URL.revokeObjectURL(url));

	const done = new Map<string, string>();
	const pending = new Map<string, Pending>();
	let painter: ThumbnailPainter | null = null;
	let working = false;
	let releaseTimer: ReturnType<typeof setTimeout> | null = null;

	function remember(key: string, url: string) {
		done.set(key, url);
		for (const [oldKey, oldUrl] of done) {
			if (done.size <= most) break;
			done.delete(oldKey);
			dropUrl(oldUrl);
		}
	}

	function releaseSoon() {
		if (releaseTimer !== null) clearTimeout(releaseTimer);
		releaseTimer = setTimeout(() => {
			releaseTimer = null;
			if (working) return;
			painter?.release();
			painter = null;
		}, releaseAfterMs);
	}

	async function work() {
		if (working) return;
		working = true;
		if (releaseTimer !== null) {
			clearTimeout(releaseTimer);
			releaseTimer = null;
		}
		for (const [key, job] of pending) {
			if (job.waiting === 0) {
				pending.delete(key);
				job.reject(new Error(THUMBNAIL_CANCELLED));
				continue;
			}
			// Sigue en la lista mientras se dibuja: quien la pida ahora espera esta misma imagen.
			try {
				painter ??= await makePainter();
				const url = makeUrl(await painter.paint(job.url, job.model));
				remember(key, url);
				job.resolve(url);
			} catch (error) {
				job.reject(
					error instanceof Error ? error : new Error(String(error)),
				);
			}
			pending.delete(key);
		}
		working = false;
		releaseSoon();
	}

	function request(url: string, model: FigureModel): ThumbnailJob {
		const key = `${model}|${url}`;
		const known = done.get(key);
		if (known !== undefined) {
			// Vuelve al final: es la que menos tiempo lleva sin usarse.
			done.delete(key);
			done.set(key, known);
			return { promise: Promise.resolve(known), cancel() {} };
		}
		let job = pending.get(key);
		if (!job) {
			let resolve!: Pending["resolve"];
			let reject!: Pending["reject"];
			const promise = new Promise<string>((res, rej) => {
				resolve = res;
				reject = rej;
			});
			job = { url, model, waiting: 0, promise, resolve, reject };
			pending.set(key, job);
		}
		const taken = job;
		taken.waiting++;
		void work();
		let cancelled = false;
		return {
			promise: taken.promise,
			cancel() {
				if (cancelled) return;
				cancelled = true;
				taken.waiting--;
			},
		};
	}

	return { request };
}
