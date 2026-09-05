import { afterEach, beforeEach, expect, mock, test } from "bun:test";
import { animateHeight } from "../src/lib/utils/animateHeight.ts";

let originals,
	node,
	content,
	style,
	media,
	browser,
	observer,
	animations,
	action;
let frames;

beforeEach(() => {
	originals = new Map();
	animations = [];
	action = undefined;
	observer = undefined;
	frames = new Map();
	let nextFrame = 1;
	class Element {
		height = 100;
		style = { height: "" };
		animatedHeight = null;
		firstElementChild = null;
		getBoundingClientRect = mock(() => {
			const inline = parseFloat(this.style.height);
			const height =
				this.animatedHeight ??
				(Number.isFinite(inline) ? inline : this.height);
			const max = this === node ? parseFloat(style.maxHeight) : NaN;
			return {
				height: Math.min(height, Number.isFinite(max) ? max : Infinity),
			};
		});
		animate = mock((keyframes) => {
			this.animatedHeight = parseFloat(keyframes.height[0]);
			const animation = {
				cancel: mock(() => {
					this.animatedHeight = null;
				}),
				onfinish: null,
				finish: () => {
					this.animatedHeight = null;
					animation.onfinish?.();
				},
			};
			animations.push(animation);
			return animation;
		});
	}
	node = new Element();
	content = node.firstElementChild = new Element();
	style = {
		paddingTop: "0px",
		paddingBottom: "0px",
		borderTopWidth: "0px",
		borderBottomWidth: "0px",
		maxHeight: "none",
	};
	media = Object.assign(new EventTarget(), { matches: false });
	browser = Object.assign(new EventTarget(), {
		matchMedia: mock(() => media),
	});
	const globals = {
		HTMLElement: Element,
		window: browser,
		getComputedStyle: mock(() => style),
		requestAnimationFrame: mock((callback) => {
			const id = nextFrame++;
			frames.set(id, callback);
			return id;
		}),
		cancelAnimationFrame: mock((id) => {
			frames.delete(id);
		}),
		ResizeObserver: class {
			constructor(callback) {
				this.notify = () => callback([], this);
				observer = this;
			}
			observe = mock();
			disconnect = mock();
		},
	};
	for (const [name, value] of Object.entries(globals)) {
		originals.set(name, Object.getOwnPropertyDescriptor(globalThis, name));
		Object.defineProperty(globalThis, name, {
			value,
			configurable: true,
			writable: true,
		});
	}
});

afterEach(() => {
	try {
		action?.destroy();
	} finally {
		for (const [name, descriptor] of originals) {
			if (descriptor) Object.defineProperty(globalThis, name, descriptor);
			else delete globalThis[name];
		}
	}
});

function start(duration = 200) {
	action = animateHeight(node, duration);
}

function flushFrame() {
	for (const [id, callback] of [...frames]) {
		if (!frames.delete(id)) continue;
		callback(0);
	}
}

function resize(height, naturalHeight = height) {
	content.height = height;
	node.height = naturalHeight;
	observer.notify();
	flushFrame();
}

test("initial observation and unchanged sizes do not animate", () => {
	start();
	expect(observer.observe).toHaveBeenCalledWith(content);
	observer.notify();
	observer.notify();
	browser.dispatchEvent(new Event("resize"));
	flushFrame();
	expect(node.animate).not.toHaveBeenCalled();

	resize(180);
	observer.notify();
	flushFrame();
	expect(node.animate).toHaveBeenCalledTimes(1);
	expect(animations[0].cancel).not.toHaveBeenCalled();
});

test("resize notifications coalesce into one frame without a first-frame jump", () => {
	start();
	expect(node.style.height).toBe("100px");
	content.height = node.height = 180;
	observer.notify();
	content.height = node.height = 220;
	observer.notify();
	browser.dispatchEvent(new Event("resize"));
	expect(frames.size).toBe(1);
	expect(globalThis.requestAnimationFrame).toHaveBeenCalledTimes(1);
	expect(node.getBoundingClientRect().height).toBe(100);
	expect(node.animate).not.toHaveBeenCalled();
	expect(globalThis.getComputedStyle).not.toHaveBeenCalled();
	flushFrame();
	expect(frames.size).toBe(0);
	expect(node.animate).toHaveBeenCalledTimes(1);
	expect(node.animate.mock.lastCall[0]).toEqual({
		height: ["100px", "220px"],
	});
	animations[0].finish();
	expect(node.style.height).toBe("220px");
	expect(node.getBoundingClientRect().height).toBe(220);
});

test("growth and shrinkage include padding and borders and respect max-height", () => {
	Object.assign(style, {
		paddingTop: "10px",
		paddingBottom: "15px",
		borderTopWidth: "2px",
		borderBottomWidth: "3px",
		maxHeight: "210px",
	});
	node.height = 130;
	start();
	observer.notify();
	flushFrame();
	expect(node.animate).not.toHaveBeenCalled();
	resize(240, 210);
	expect(node.animate).toHaveBeenLastCalledWith(
		{ height: ["130px", "210px"] },
		expect.objectContaining({ duration: 200 }),
	);
	animations[0].finish();
	expect(node.style.height).toBe("210px");
	resize(260, 210);
	expect(node.animate).toHaveBeenCalledTimes(1);
	resize(50, 80);
	expect(node.animate.mock.lastCall[0]).toEqual({
		height: ["210px", "80px"],
	});
	expect(animations[0].cancel).not.toHaveBeenCalled();
	animations[1].finish();
	expect(node.style.height).toBe("80px");
	observer.notify();
	flushFrame();
	expect(node.animate).toHaveBeenCalledTimes(2);
});

test("window resize re-evaluates the max-height constraint", () => {
	style.maxHeight = "70px";
	start();
	style.maxHeight = "120px";
	browser.dispatchEvent(new Event("resize"));
	flushFrame();
	expect(node.animate.mock.lastCall[0]).toEqual({
		height: ["70px", "100px"],
	});
});

test("interruption cancels the old animation and starts at the measured height", () => {
	start();
	resize(200);
	expect(node.animate.mock.lastCall[0]).toEqual({
		height: ["100px", "200px"],
	});
	node.animatedHeight = 145;
	resize(80);
	expect(animations[0].cancel).toHaveBeenCalledTimes(1);
	expect(node.animate).toHaveBeenCalledTimes(2);
	expect(node.animate.mock.lastCall[0]).toEqual({
		height: ["145px", "80px"],
	});
	expect(animations[1].cancel).not.toHaveBeenCalled();
});

for (const mode of ["zero duration", "reduced motion"]) {
	test(`${mode} skips growth and shrink animations`, () => {
		node.style.height = "auto";
		media.matches = mode === "reduced motion";
		start(mode === "zero duration" ? 0 : 200);
		expect(browser.matchMedia).toHaveBeenCalledWith(
			"(prefers-reduced-motion: reduce)",
		);
		resize(200);
		resize(60);
		expect(node.animate).not.toHaveBeenCalled();
		expect(node.style.height).toBe("auto");
		expect(node.getBoundingClientRect().height).toBe(60);
	});

	test(`switching to ${mode} cancels animation and pending frame and restores height`, () => {
		node.style.height = "auto";
		start();
		resize(200);
		node.animatedHeight = 140;
		content.height = node.height = 250;
		observer.notify();
		const [pending] = frames.keys();
		expect(frames.size).toBe(1);
		if (mode === "zero duration") action.update(0);
		else {
			media.matches = true;
			media.dispatchEvent(new Event("change"));
		}
		expect(animations[0].cancel).toHaveBeenCalledTimes(1);
		expect(globalThis.cancelAnimationFrame).toHaveBeenCalledWith(pending);
		expect(frames.size).toBe(0);
		flushFrame();
		expect(node.style.height).toBe("auto");
		expect(node.getBoundingClientRect().height).toBe(250);
		resize(250);
		expect(node.animate).toHaveBeenCalledTimes(1);

		if (mode === "zero duration") action.update(300);
		else {
			media.matches = false;
			media.dispatchEvent(new Event("change"));
		}
		expect(node.style.height).toBe("250px");
		resize(80);
		expect(node.animate).toHaveBeenCalledTimes(2);
		expect(node.animate).toHaveBeenLastCalledWith(
			{ height: ["250px", "80px"] },
			expect.objectContaining({
				duration: mode === "zero duration" ? 300 : 200,
			}),
		);
	});
}

test("destroy disconnects observer, removes listeners, cancels animation/frame and restores height", () => {
	node.style.height = "120px";
	start();
	resize(200);
	content.height = node.height = 300;
	observer.notify();
	const [pending] = frames.keys();
	expect(frames.size).toBe(1);
	action.destroy();
	action = undefined;
	expect(observer.disconnect).toHaveBeenCalledTimes(1);
	expect(animations[0].cancel).toHaveBeenCalledTimes(1);
	expect(globalThis.cancelAnimationFrame).toHaveBeenCalledWith(pending);
	expect(frames.size).toBe(0);
	expect(node.style.height).toBe("120px");
	expect(node.getBoundingClientRect().height).toBe(120);
	node.getBoundingClientRect.mockClear();
	globalThis.getComputedStyle.mockClear();
	content.height = 300;
	browser.dispatchEvent(new Event("resize"));
	media.matches = true;
	media.dispatchEvent(new Event("change"));
	expect(frames.size).toBe(0);
	flushFrame();
	expect(globalThis.getComputedStyle).not.toHaveBeenCalled();
	expect(node.getBoundingClientRect).not.toHaveBeenCalled();
	expect(node.animate).toHaveBeenCalledTimes(1);
	expect(animations[0].cancel).toHaveBeenCalledTimes(1);
});

test("a node without a child is a no-op", () => {
	node.firstElementChild = null;
	start();
	expect(action).toBeUndefined();
	expect(observer).toBeUndefined();
	expect(browser.matchMedia).not.toHaveBeenCalled();
	expect(node.getBoundingClientRect).not.toHaveBeenCalled();
	expect(node.animate).not.toHaveBeenCalled();
	expect(frames.size).toBe(0);
	expect(node.style.height).toBe("");
});
