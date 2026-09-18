import { mount, unmount, flushSync, tick } from "svelte";
import Layout from "./interfaceLayout.svelte";
import { launcherStore, errors } from "$lib/state/state.svelte";
import { applyTheme } from "$lib/api/themeManager";

const assert = (value, message) => {
	if (!value) throw Error(message);
};
const near = (a, b) => Math.abs(a - b) < 0.6;
const settle = async () => {
	flushSync();
	await tick();
	await new Promise((resolve) =>
		requestAnimationFrame(() => requestAnimationFrame(resolve)),
	);
	flushSync();
	await tick();
};
const themeCSS = () => document.querySelector("#cubic-theme-vars").textContent;
const heights = () => ({
	list: document
		.querySelector(".list-host .installed-item")
		.getBoundingClientRect().height,
	grid: document
		.querySelector(".grid-host .installed-item")
		.getBoundingClientRect().height,
});
const assertBounds = (element, name) => {
	const r = element.getBoundingClientRect();
	assert(
		r.left >= -1 &&
			r.right <= innerWidth + 1 &&
			r.top >= -1 &&
			r.bottom <= innerHeight + 1,
		`${name} outside viewport (${innerWidth}x${innerHeight}): ${JSON.stringify(r)}`,
	);
};
async function choose(id, label) {
	const trigger = document.querySelector(`#${id} button`);
	trigger.scrollIntoView({ block: "nearest" });
	await settle();
	trigger.click();
	await settle();
	const menu = document.querySelector('[role="listbox"]');
	assert(menu, `${id}: menu did not open for ${label}`);
	assertBounds(menu, id + " menu");
	const a = trigger.getBoundingClientRect(),
		b = menu.getBoundingClientRect();
	assert(near(a.left, b.left), id + " menu not aligned with trigger");
	const option = [...menu.querySelectorAll('[role="option"]')].find(
		(node) => node.textContent.trim() === label,
	);
	assert(option, `Missing option ${label}`);
	option.click();
	await settle();
}

let failZoom = false;
const zoomCalls = [],
	saves = [];
globalThis.isTauri = true;
window.__TAURI_INTERNALS__ = {
	metadata: {
		currentWindow: { label: "main" },
		currentWebview: { label: "main" },
	},
	convertFileSrc: (path) => path,
	async invoke(command, payload) {
		if (command === "plugin:webview|set_webview_zoom") {
			zoomCalls.push(payload.value);
			if (failZoom) {
				failZoom = false;
				throw Error("Simulated zoom failure");
			}
			// Native page zoom changes the CSS viewport, unlike CSS transforms.
			// Exercise the resulting minimum-window layout for every supported scale.
			frameElement.style.width = `${800 / payload.value}px`;
			frameElement.style.height = `${600 / payload.value}px`;
			return;
		}
		if (command === "get_user_theme")
			return {
				name: "Custom layout",
				variables: {
					"--font-size-base": "16px",
					"--font-family": "serif",
					"--accent": "#ea9d34",
					"--drawer-width": "380px",
					"--market-installed-row-height": "calc(5rem + 14px)",
					"--market-installed-row-gap": "0.5rem",
					"--market-grid-gap": "1rem",
				},
				fonts: [],
				icons: {},
				inject_css:
					".market-grid { --market-row-height: calc(15rem + 16px); }",
			};
		throw Error(`Unexpected IPC ${command}`);
	},
};

let cases = 0,
	component;
try {
	component = mount(Layout, {
		target: document.querySelector("main"),
		props: {
			onsave: async () =>
				saves.push({ ...launcherStore.settings.interface_preferences }),
		},
	});
	await settle();
	for (const theme of [
		"dark",
		"light",
		"lima",
		"rose-pine",
		"rose-pine-dawn",
		"user:custom-layout",
	]) {
		await applyTheme(theme);
		await settle();
		// Theme assets finish after applyTheme resolves. Take the baseline only
		// once its background/fonts have loaded, not during their own updates.
		for (let attempt = 0; attempt < 100; attempt++) {
			const style = document.documentElement.style;
			if (
				style.getPropertyValue("--font-loaded") !== "0" &&
				(style.getPropertyValue("--bg-image-loaded") !== "0" ||
					style.getPropertyValue("--bg-image") === "none")
			)
				break;
			await settle();
		}
		await document.fonts.ready;
		const injectedCSS = document.querySelector("#cubic-theme-css");
		if (injectedCSS && !injectedCSS.sheet) {
			await new Promise((resolve, reject) => {
				injectedCSS.addEventListener("load", resolve, { once: true });
				injectedCSS.addEventListener("error", reject, { once: true });
			});
		}
		await settle();
		const css = themeCSS();
		const original = heights();
		const originalRow = document
			.querySelector(".virtual-list-item-wrapper")
			.getBoundingClientRect().height;
		const originalGridGap = parseFloat(
			getComputedStyle(
				document.querySelector(".market-grid"),
			).getPropertyValue("--grid-gap"),
		);
		const style = document.documentElement.getAttribute("style");
		for (const scale of [90, 100, 110, 125]) {
			await choose("interface-scale", `${scale} %`);
			assert(
				launcherStore.settings.interface_preferences.scale === scale,
				"scale did not update",
			);
			assertBounds(document.querySelector(".drawer"), "settings drawer");
			const scroll = document.querySelector(".qm-scroll");
			assert(
				scroll.scrollWidth <= scroll.clientWidth + 1,
				"settings overflow horizontally",
			);
			for (const density of ["compact", "comfortable", "theme"]) {
				await choose(
					"interface-density",
					{
						compact: "Compact",
						comfortable: "Comfortable",
						theme: "Theme default",
					}[density],
				);
				assert(
					launcherStore.settings.interface_preferences.density ===
						density,
					"density control did not update preferences",
				);
				const factor = { compact: 0.5, comfortable: 1.5, theme: 1 }[
					density
				];
				// ResizeObserver delivers theme measurements on a rendering frame,
				// which may be throttled for an iframe in headless Firefox.
				for (let attempt = 0; attempt < 40; attempt++) {
					const row = document
						.querySelector(".virtual-list-item-wrapper")
						.getBoundingClientRect().height;
					const gap = parseFloat(
						getComputedStyle(
							document.querySelector(".market-grid"),
						).getPropertyValue("--grid-gap"),
					);
					if (
						near(
							row - heights().list,
							(originalRow - original.list) * factor,
						) &&
						near(gap, originalGridGap * factor)
					)
						break;
					await settle();
				}
				const next = heights();
				assert(
					near(original.list, next.list),
					`${theme}/${density}: list content resized`,
				);
				assert(
					near(original.grid, next.grid),
					`${theme}/${density}: grid content resized (${original.grid} -> ${next.grid})`,
				);
				const row = document
					.querySelector(".virtual-list-item-wrapper")
					.getBoundingClientRect().height;
				assert(
					near(
						row - next.list,
						(originalRow - original.list) * factor,
					),
					`${theme}/${scale}/${density}: list gap ${row - next.list}, expected ${(originalRow - original.list) * factor} (row ${row}, original ${originalRow}, content ${next.list})`,
				);
				const gap = parseFloat(
					getComputedStyle(
						document.querySelector(".market-grid"),
					).getPropertyValue("--grid-gap"),
				);
				assert(
					near(gap, originalGridGap * factor),
					"grid density did not adjust its gap",
				);
				assert(themeCSS() === css, "theme variables overwritten");
				assert(
					document.documentElement.getAttribute("style") === style,
					"inline theme styles overwritten",
				);
				const wrappers = [
					...document.querySelectorAll(".virtual-list-item-wrapper"),
				];
				for (let i = 1; i < wrappers.length; i++) {
					assert(
						near(
							wrappers[i - 1].getBoundingClientRect().bottom,
							wrappers[i].getBoundingClientRect().top,
						),
						"virtual row positions out of sync",
					);
				}
				cases++;
			}
		}
	}
	// A locally scoped, asynchronously loaded Inject.css override must still win.
	assert(near(heights().grid, 240), "custom Inject.css row height lost");
	await choose("interface-density", "Compact");
	await applyTheme("dark");
	await settle();
	assert(
		document.documentElement.dataset.interfaceDensity === "compact",
		"theme change lost density",
	);
	assert(near(heights().grid, 212), "theme change kept previous metrics");
	await choose("interface-density", "Comfortable");
	for (const selector of [".virtual-list-container", ".market-grid"]) {
		const scroller = document.querySelector(selector);
		scroller.scrollTop = scroller.scrollHeight;
	}
	await settle();
	await choose("interface-density", "Compact");
	for (const selector of [".virtual-list-container", ".market-grid"]) {
		const scroller = document.querySelector(selector);
		const viewport = scroller.getBoundingClientRect();
		assert(
			[...scroller.querySelectorAll(".installed-item")].some((item) => {
				const r = item.getBoundingClientRect();
				return r.bottom > viewport.top && r.top < viewport.bottom;
			}),
			"density change left the end of the virtual list blank",
		);
	}
	await choose("interface-scale", "100 %");
	const beforeFailure = saves.length;
	failZoom = true;
	await choose("interface-scale", "125 %");
	assert(saves.length === beforeFailure, "failed zoom was saved");
	assert(
		launcherStore.settings.interface_preferences.scale === 100,
		"failed zoom changed preferences",
	);
	assert(
		document
			.querySelector("#interface-scale .selected-value")
			.textContent.trim() === "100 %",
		"failed zoom left stale control value",
	);
	assert(errors.length === 1, "native failure was not reported");
	await choose("interface-scale", "125 %");
	const reset = document.querySelector(".interface-settings .detect-btn");
	reset.scrollIntoView();
	reset.click();
	await settle();
	assert(
		!document.documentElement.hasAttribute("data-interface-density"),
		"reset kept density override",
	);
	assert(zoomCalls.at(-1) === 1, "reset did not restore native scale");
	assert(
		JSON.stringify(saves.at(-1)) ===
			JSON.stringify({ scale: 100, density: "theme" }),
		"reset not persisted",
	);
	await unmount(component);
	component = null;
	assert(
		!document.querySelector('[role="listbox"]'),
		"portal leaked after unmount",
	);
	await fetch("/result", { method: "POST", body: JSON.stringify({ cases }) });
} catch (error) {
	await fetch("/result", {
		method: "POST",
		body: JSON.stringify({
			error: String(error),
			stack: error.stack,
			cases,
		}),
	});
} finally {
	if (component) await unmount(component);
}
