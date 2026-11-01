import { expect, test } from "bun:test";
import { createFrameLoop } from "../../../src/lib/utils/frameLoop.ts";

/** Un navegador de mentira: el tiempo solo avanza con `run`, a golpes de refresco de pantalla. */
function fakeBrowser() {
	let time = 0;
	let nextId = 1;
	const frames = new Map();
	const timers = new Map();
	let wakeups = 0;
	return {
		clock: {
			now: () => time,
			requestFrame(callback) {
				frames.set(nextId, callback);
				return nextId++;
			},
			cancelFrame: (id) => frames.delete(id),
			setTimer(callback, ms) {
				timers.set(nextId, { callback, at: time + ms });
				return nextId++;
			},
			clearTimer: (id) => timers.delete(id),
		},
		run(ms, hertz = 60) {
			const refresh = 1000 / hertz;
			const end = time + ms;
			while (time < end) {
				time += refresh;
				for (const [id, timer] of [...timers]) {
					if (timer.at <= time) {
						timers.delete(id);
						timer.callback();
					}
				}
				const due = [...frames.values()];
				frames.clear();
				if (due.length > 0) wakeups++;
				for (const callback of due) callback();
			}
		},
		skip(ms) {
			time += ms;
		},
		get wakeups() {
			return wakeups;
		},
		get scheduled() {
			return frames.size + timers.size;
		},
	};
}

function looped(browser) {
	const steps = [];
	return {
		steps,
		loop: createFrameLoop((step) => steps.push(step), browser.clock),
	};
}

test("sin animación ni peticiones no se programa nada", () => {
	const browser = fakeBrowser();
	const { steps, loop } = looped(browser);
	loop.setRate(0);
	browser.run(1000);
	expect(steps).toEqual([]);
	expect(browser.wakeups).toBe(0);
	expect(browser.scheduled).toBe(0);
});

test("varias peticiones en un refresco son un solo dibujo, que no avanza la animación", () => {
	const browser = fakeBrowser();
	const { steps, loop } = looped(browser);
	loop.request();
	loop.request();
	loop.request();
	browser.run(100);
	expect(steps).toEqual([0]);
	expect(browser.scheduled).toBe(0);
});

for (const hertz of [60, 144, 240]) {
	test(`a 24 fps con una pantalla de ${hertz} Hz solo despierta para dibujar`, () => {
		const browser = fakeBrowser();
		const { steps, loop } = looped(browser);
		loop.setRate(24);
		browser.run(2000, hertz);
		expect(Math.abs(steps.length - 48)).toBeLessThanOrEqual(1);
		expect(browser.wakeups).toBe(steps.length);
	});
}

test("los pasos de la animación suman el tiempo transcurrido", () => {
	const browser = fakeBrowser();
	const { steps, loop } = looped(browser);
	loop.setRate(24);
	browser.run(3000);
	const total = steps.reduce((sum, step) => sum + step, 0);
	expect(Math.abs(total - 3)).toBeLessThan(0.1);
});

test("parar no deja nada programado, y al seguir no hay salto", () => {
	const browser = fakeBrowser();
	const { steps, loop } = looped(browser);
	loop.setRate(24);
	browser.run(500);
	loop.setRate(0);
	expect(browser.scheduled).toBe(0);
	const before = steps.length;
	browser.run(10000);
	expect(steps.length).toBe(before);
	loop.setRate(24);
	browser.run(200);
	expect(steps[before]).toBe(0);
	expect(Math.max(...steps.slice(before))).toBeLessThan(0.1);
});

test("un dibujo pedido antes de parar se hace igualmente", () => {
	const browser = fakeBrowser();
	const { steps, loop } = looped(browser);
	loop.setRate(24);
	browser.run(100);
	const before = steps.length;
	loop.request();
	loop.setRate(0);
	browser.run(100);
	expect(steps.length).toBe(before + 1);
	expect(browser.scheduled).toBe(0);
});

test("una pausa larga del navegador no hace saltar la animación", () => {
	const browser = fakeBrowser();
	const { steps, loop } = looped(browser);
	loop.setRate(24);
	browser.run(200);
	// Ventana oculta: el refresco no llega durante cinco segundos.
	browser.skip(5000);
	browser.run(200);
	expect(Math.max(...steps)).toBeLessThanOrEqual(0.1);
});

test("al desmontar no queda nada programado y no se dibuja más", () => {
	const browser = fakeBrowser();
	const { steps, loop } = looped(browser);
	loop.setRate(24);
	browser.run(100);
	const before = steps.length;
	loop.request();
	loop.dispose();
	expect(browser.scheduled).toBe(0);
	browser.run(500);
	expect(steps.length).toBe(before);
});
