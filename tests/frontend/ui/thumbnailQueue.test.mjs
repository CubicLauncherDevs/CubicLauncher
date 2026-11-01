import { expect, test } from "bun:test";
import {
	createThumbnailQueue,
	THUMBNAIL_CANCELLED,
} from "../../../src/lib/utils/thumbnailQueue.ts";

const later = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function setup(options = {}) {
	const log = { painters: 0, released: 0, painted: [], dropped: [] };
	let fail = new Set();
	const queue = createThumbnailQueue(
		async () => {
			log.painters++;
			return {
				async paint(url, model) {
					await later(1);
					if (fail.has(url)) throw new Error(`no ${url}`);
					log.painted.push(`${model}|${url}`);
					return new Blob([url]);
				},
				release() {
					log.released++;
				},
			};
		},
		{
			makeUrl: (blob) => `blob:${blob.size}:${log.painted.at(-1)}`,
			dropUrl: (url) => log.dropped.push(url),
			releaseAfterMs: 5,
			...options,
		},
	);
	return { queue, log, failOn: (url) => fail.add(url) };
}

test("una misma escena dibuja todas las imágenes pedidas", async () => {
	const { queue, log } = setup();
	const urls = await Promise.all(
		["a", "b", "c"].map((url) => queue.request(url, "default").promise),
	);
	expect(new Set(urls).size).toBe(3);
	expect(log.painters).toBe(1);
	expect(log.painted).toEqual(["default|a", "default|b", "default|c"]);
});

test("la misma skin pedida dos veces se dibuja una", async () => {
	const { queue, log } = setup();
	const [one, two] = await Promise.all([
		queue.request("a", "default").promise,
		queue.request("a", "default").promise,
	]);
	expect(one).toBe(two);
	expect(await queue.request("a", "default").promise).toBe(one);
	expect(log.painted).toEqual(["default|a"]);
});

test("la misma skin con otros brazos es otra imagen", async () => {
	const { queue, log } = setup();
	await queue.request("a", "default").promise;
	await queue.request("a", "slim").promise;
	expect(log.painted).toEqual(["default|a", "slim|a"]);
});

test("lo cancelado antes de su turno no se dibuja", async () => {
	const { queue, log } = setup();
	const first = queue.request("a", "default");
	const second = queue.request("b", "default");
	second.cancel();
	await first.promise;
	await expect(second.promise).rejects.toThrow(THUMBNAIL_CANCELLED);
	expect(log.painted).toEqual(["default|a"]);
});

test("si alguien más la espera, cancelar no la quita", async () => {
	const { queue, log } = setup();
	queue.request("a", "default");
	const one = queue.request("b", "default");
	const two = queue.request("b", "default");
	one.cancel();
	await two.promise;
	expect(log.painted).toContain("default|b");
});

test("olvida primero lo que lleva más tiempo sin usarse", async () => {
	const { queue, log } = setup({ most: 2 });
	const a = await queue.request("a", "default").promise;
	const b = await queue.request("b", "default").promise;
	await queue.request("a", "default").promise;
	await queue.request("c", "default").promise;
	expect(log.dropped).toEqual([b]);
	expect(log.dropped).not.toContain(a);
	expect(await queue.request("a", "default").promise).toBe(a);
});

test("una skin que falla no detiene las siguientes", async () => {
	const { queue, log, failOn } = setup();
	failOn("mala");
	const bad = queue.request("mala", "default");
	const good = queue.request("buena", "default");
	await expect(bad.promise).rejects.toThrow("no mala");
	await good.promise;
	expect(log.painted).toEqual(["default|buena"]);
});

test("sin trabajo suelta la escena, y la crea de nuevo si hace falta", async () => {
	const { queue, log } = setup();
	await queue.request("a", "default").promise;
	expect(log.released).toBe(0);
	await later(30);
	expect(log.released).toBe(1);
	await queue.request("b", "default").promise;
	expect(log.painters).toBe(2);
	await later(30);
	expect(log.released).toBe(2);
});

test("un pedido nuevo a tiempo evita soltar la escena", async () => {
	const { queue, log } = setup({ releaseAfterMs: 20 });
	await queue.request("a", "default").promise;
	await later(5);
	await queue.request("b", "default").promise;
	expect(log.released).toBe(0);
	expect(log.painters).toBe(1);
});
