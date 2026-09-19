import { mount, unmount } from "svelte";
import Tutorial from "$lib/components/layout/welcome/welcome.svelte";

const wait = (ms = 350) => new Promise((resolve) => setTimeout(resolve, ms));
function assert(value, message) {
	if (!value) throw Error(message);
}
function centered(element) {
	const r = element.getBoundingClientRect();
	assert(
		Math.abs(r.x + r.width / 2 - innerWidth / 2) < 2,
		"Not centered horizontally",
	);
	assert(
		Math.abs(r.y + r.height / 2 - innerHeight / 2) < 2,
		"Not centered vertically",
	);
}

try {
	// A missing anchor must not leave the tutorial at its initial (0, 0).
	let tutorial = mount(Tutorial, {
		target: document.body,
		props: { open: true },
	});
	await wait();
	centered(document.querySelector(".tut-tip"));
	await unmount(tutorial);

	const anchor = document.createElement("div");
	anchor.dataset.tutorial = "sidebar-header";
	anchor.style.cssText =
		"position:fixed;left:20px;top:100px;width:180px;height:40px";
	document.body.append(anchor);
	tutorial = mount(Tutorial, {
		target: document.body,
		props: { open: true },
	});
	await wait();
	const tip = document.querySelector(".tut-tip");
	const original = tip.getBoundingClientRect();
	assert(
		Math.abs(original.left - 212) < 2,
		"Tutorial is not beside its anchor",
	);

	for (const selector of [".tut-skip", ".tut-close", ".tut-overlay"]) {
		document.querySelector(selector).click();
		await wait();
		centered(document.querySelector(".modal"));
		document.querySelector(".modal-footer .btn-secondary").click();
		await wait();
		const restored = tip.getBoundingClientRect();
		assert(
			Math.abs(restored.x - original.x) < 2 &&
				Math.abs(restored.y - original.y) < 2,
			"Cancel moved the tutorial",
		);
	}

	// Simulate a translation/content change while the current step is open.
	tip.style.height = "460px";
	await wait();
	assert(
		tip.getBoundingClientRect().bottom <= innerHeight - 9,
		"Resized tutorial overflows the viewport",
	);
	tip.style.height = "";
	await wait();
	document.querySelector(".tut-skip").click();
	await wait();
	document.querySelector(".modal-footer .btn-primary").click();
	await wait();
	assert(
		document.querySelector(".license-check"),
		"Skipping must reach the license step",
	);
	centered(tip);
	await unmount(tutorial);
	await fetch("/result", {
		method: "POST",
		body: JSON.stringify({ ok: true }),
	});
} catch (error) {
	await fetch("/result", {
		method: "POST",
		body: JSON.stringify({ error: error.message, stack: error.stack }),
	});
}
