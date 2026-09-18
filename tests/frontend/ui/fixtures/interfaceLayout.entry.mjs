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
let blockSave;
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
			onsave: async () => {
				saves.push({ ...launcherStore.settings.interface_preferences });
				if (blockSave) await blockSave;
			},
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
	const noOpSaves = saves.length;
	const noOpZoom = zoomCalls.length;
	const interfaceReference = launcherStore.settings.interface_preferences;
	await choose("interface-scale", "100 %");
	await choose("interface-density", "Theme default");
	reset.click();
	await settle();
	const notificationReset = document.querySelector(
		".notification-settings .actions button:last-child",
	);
	notificationReset.click();
	await settle();
	assert(
		saves.length === noOpSaves && zoomCalls.length === noOpZoom,
		"unchanged preferences performed work",
	);
	assert(
		launcherStore.settings.interface_preferences === interfaceReference,
		"interface preference identity replaced",
	);
	let releaseSave;
	blockSave = new Promise((resolve) => {
		releaseSave = resolve;
	});
	await choose("interface-density", "Compact");
	assert(
		!document.querySelector("#interface-density button").disabled,
		"disk write blocked the controls",
	);
	await choose("interface-density", "Comfortable");
	releaseSave();
	blockSave = undefined;
	await settle();
	assert(
		launcherStore.settings.interface_preferences === interfaceReference,
		"density replaced preference object",
	);
	reset.click();
	await settle();
	await verifyNotifications();
	const beforeClose = saves.length;
	const titleInput = document.querySelector("#notification-title_size");
	titleInput.value = "21";
	titleInput.dispatchEvent(new Event("input", { bubbles: true }));
	flushSync();
	await unmount(component);
	component = null;
	assert(
		saves.length === beforeClose + 1,
		"unmount lost an uncommitted slider value",
	);
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

async function verifyNotifications() {
	const preferences = launcherStore.settings.notification_preferences;
	const toggle = document.querySelector("#notification-customization");
	toggle.click();
	await settle();
	const duration = document.querySelector("#notification-duration_seconds");
	duration.value = "6";
	duration.dispatchEvent(new Event("input", { bubbles: true }));
	duration.dispatchEvent(new Event("change", { bubbles: true }));
	await settle();
	const timers = new Map();
	let timerStarts = 0;
	const nativeSetTimeout = window.setTimeout,
		nativeClearTimeout = window.clearTimeout;
	window.setTimeout = (callback, delay, ...args) => {
		if (delay !== 6000) return nativeSetTimeout(callback, delay, ...args);
		const id = -++timerStarts;
		timers.set(id, () => callback(...args));
		return id;
	};
	window.clearTimeout = (id) => {
		if (!timers.delete(id)) nativeClearTimeout(id);
	};
	try {
		launcherStore.notifications = [
			{
				id: "timed",
				type: "info",
				title: "Timed notice",
				message: "Long notification message",
				timeout: 5000,
			},
			{
				id: "persistent",
				type: "info",
				title: "Persistent notice",
				timeout: 0,
			},
			{ id: "download", type: "info", title: "Download", progress: 10 },
		];
		await settle();
		assert(timerStarts === 1, "unexpected notification timers");
		const countdown = document.querySelector(".progress-ring.countdown");
		assert(countdown, "timed notification has no countdown");
		await verifyNotificationAppearance(countdown);
		const title = document.querySelector("#notification-title_size");
		const before = saves.length;
		for (let i = 0; i < 100; i++) {
			title.value = String(12 + (i % 13));
			title.dispatchEvent(new Event("input", { bubbles: true }));
			flushSync();
		}
		assert(saves.length === before, "slider saved during dragging");
		assert(
			launcherStore.settings.notification_preferences === preferences,
			"slider replaced the preference object",
		);
		title.dispatchEvent(new Event("change", { bubbles: true }));
		await settle();
		assert(saves.length === before + 1, "slider did not commit once");
		title.dispatchEvent(new Event("change", { bubbles: true }));
		await settle();
		assert(saves.length === before + 1, "unchanged slider saved again");
		preferences.position = "bottom-left";
		preferences.size = "wide";
		preferences.message_size = 22;
		preferences.uppercase_title = true;
		preferences.bold_title = true;
		const progressRing = document.querySelectorAll(".progress-ring")[2];
		for (const progress of [-5, 0, 25, 40, 80, 99]) {
			launcherStore.notifications[2].progress = progress;
			await settle();
			await Promise.all(
				progressRing
					.getAnimations()
					.map((animation) => animation.finished),
			);
			assert(
				near(
					parseFloat(
						getComputedStyle(progressRing).getPropertyValue(
							"--notification-progress-angle",
						),
					),
					Math.max(0, progress) * 3.6,
				),
				"download progress does not match its border indicator",
			);
		}
		assert(
			timerStarts === 1 && timers.size === 1,
			"visual changes or progress restarted notification timers",
		);
		assert(
			document.querySelector(".progress-ring.countdown") === countdown,
			"visual changes remounted the countdown",
		);
		launcherStore.notifications[2].progress = 100;
		await settle();
		assert(
			timerStarts === 2 && timers.size === 2,
			"download completion did not start one timer",
		);
		for (const callback of [...timers.values()]) callback();
		await settle();
		await new Promise((resolve) => nativeSetTimeout(resolve, 150));
		await settle();
		assert(
			launcherStore.notifications.length === 1 &&
				launcherStore.notifications[0].id === "persistent",
			"notification dismissal changed persistent notice lifetime",
		);
	} finally {
		window.setTimeout = nativeSetTimeout;
		window.clearTimeout = nativeClearTimeout;
	}
}

async function verifyNotificationAppearance(countdown) {
	const root = document.documentElement;
	const toast = countdown.closest(".notification-toast");
	const icon = countdown.closest(".notification-icon");
	const wrap = icon.parentElement;
	const animation = countdown.getAnimations()[0];
	assert(animation, "countdown animation did not start");
	animation.pause();
	animation.currentTime = 3000;
	const variables = ["--toast-radius", "--toast-blur"];
	const original = variables.map((name) => root.style.getPropertyValue(name));
	const flags = [
		"data-no-blur",
		"data-reduce-motion",
		"data-no-infinite-animations",
	];
	const attributes = flags.map((name) => root.getAttribute(name));
	try {
		for (const radius of [
			"50%",
			"0px",
			"8px",
			"0px 8px 14px 3px / 0px 12px 6px 3px",
		]) {
			root.style.setProperty("--toast-radius", radius);
			for (const size of [32, 44]) {
				wrap.style.setProperty("--notification-icon-size", `${size}px`);
				await settle();
				const shape = getComputedStyle(icon);
				const ring = getComputedStyle(countdown);
				for (const corner of [
					"borderTopLeftRadius",
					"borderTopRightRadius",
					"borderBottomRightRadius",
					"borderBottomLeftRadius",
				]) {
					assert(
						ring[corner] === shape[corner],
						`loader does not follow ${corner}: ${radius}`,
					);
				}
				const iconBounds = icon.getBoundingClientRect();
				const ringBounds = countdown.getBoundingClientRect();
				for (const edge of ["left", "top", "right", "bottom"]) {
					assert(
						near(iconBounds[edge], ringBounds[edge]),
						`loader detached from icon at ${size}px`,
					);
				}
				assert(
					countdown.getAnimations()[0] === animation,
					"theme change restarted countdown",
				);
				assert(
					near(
						parseFloat(
							ring.getPropertyValue(
								"--notification-progress-angle",
							),
						),
						180,
					),
					"countdown does not interpolate at half duration",
				);
			}
		}
		// Injected theme CSS can override the icon itself, independently of tokens.
		icon.style.borderRadius = "0px";
		await settle();
		assert(
			getComputedStyle(countdown).borderRadius === "0px",
			"direct square icon override lost",
		);
		icon.style.removeProperty("border-radius");
		for (const blur of ["12px", "0px"]) {
			root.style.setProperty("--toast-blur", blur);
			await settle();
			assert(
				getComputedStyle(toast).backdropFilter === `blur(${blur})`,
				"toast blur customization ignored",
			);
		}
		root.style.setProperty("--toast-blur", "12px");
		for (const flag of flags) {
			root.setAttribute(flag, "");
			await settle();
			assert(
				getComputedStyle(countdown).animationDuration === "6s",
				`${flag} changed countdown duration`,
			);
			if (flag !== "data-no-infinite-animations") {
				assert(
					getComputedStyle(toast).backdropFilter === "none",
					`${flag} did not disable toast blur`,
				);
			}
			root.removeAttribute(flag);
		}
	} finally {
		variables.forEach((name, index) => {
			if (original[index]) root.style.setProperty(name, original[index]);
			else root.style.removeProperty(name);
		});
		flags.forEach((name, index) => {
			if (attributes[index] === null) root.removeAttribute(name);
			else root.setAttribute(name, attributes[index]);
		});
		wrap.style.removeProperty("--notification-icon-size");
		icon.style.removeProperty("border-radius");
		animation.play();
	}
}
