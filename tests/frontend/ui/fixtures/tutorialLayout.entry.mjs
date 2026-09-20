import { mount, unmount } from "svelte";
import Tutorial from "$lib/components/layout/welcome/welcome.svelte";
import { launcherStore } from "$lib/state/state.svelte";
import { calls } from "$lib/api/cubicApi";
import { saves, hooks } from "$lib/api/launcherService";
import { checkProfileAccounts } from "./profileAccounts.entry.mjs";

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

function input(selector, value) {
	const element = document.querySelector(selector);
	element.value = value;
	element.dispatchEvent(new Event("input", { bubbles: true }));
}

async function chooseAccount(label) {
	const option = [...document.querySelectorAll(".account-option")].find(
		(button) => button.textContent.trim() === label,
	);
	assert(option, `Missing account option: ${label}`);
	option.click();
	await wait();
	centered(document.querySelector(".modal"));
	assert(
		document.querySelector(".tut-tip").inert,
		"Tutorial must pause during account setup",
	);
}

async function closeAccount() {
	document.querySelector(".modal-header .action-btn").click();
	await wait();
	assert(
		!document.querySelector(".tut-tip").inert,
		"Tutorial did not resume",
	);
	assert(
		document.querySelector(".account-options"),
		"Closing account setup changed the step",
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

	// Account dialogs must sit above the tutorial without inheriting its position.
	document.querySelectorAll(".tut-dot")[1].click();
	await wait();
	assert(
		document.querySelectorAll(".account-option").length === 5,
		"Expected five providers",
	);
	assert(
		tip.getBoundingClientRect().bottom <= innerHeight - 9,
		"Account choices overflow",
	);
	await chooseAccount("Offline");
	input("#account-offline-name", "x!");
	assert(
		!document.querySelector("#account-offline-name").checkValidity(),
		"Invalid offline name accepted",
	);
	input("#account-offline-name", "TestPlayer");
	await wait();
	hooks.failSave = true;
	document.querySelector(".offline-form").requestSubmit();
	await wait();
	assert(
		document
			.querySelector('[role="alert"]')
			.textContent.includes("Save failed"),
		"Save error not shown",
	);
	hooks.failSave = false;
	document.querySelector(".offline-form").requestSubmit();
	await wait();
	assert(
		!document.querySelector(".modal"),
		"Offline setup did not close after saving",
	);
	assert(
		launcherStore.settings.user.length === 1,
		"Retry duplicated offline account",
	);
	assert(saves.length === 1, "Offline account was not saved");
	assert(
		document
			.querySelector('[role="status"]')
			.textContent.includes("TestPlayer"),
		"Active account not shown",
	);
	await chooseAccount("Premium (Microsoft)");
	assert(calls.includes("microsoft"), "Premium did not start Microsoft auth");
	assert(
		launcherStore.settings.user[launcherStore.settings.active_user_idx]
			.username === "PremiumPlayer",
		"Premium was not activated",
	);
	await closeAccount();

	for (const [provider, url] of [
		["Ely.by", "https://account.ely.by/api/authlib-injector"],
		["CubicAuth", "https://auth.cubiclauncher.org"],
		["Authinject", "https://custom.example.test/api/yggdrasil"],
	]) {
		await chooseAccount(provider);
		const serverInput = document.querySelector("#ygg-server-url");
		assert(
			serverInput.value === (provider === "Authinject" ? "" : url),
			"Wrong server preset",
		);
		input("#ygg-server-url", url);
		await wait();
		document.querySelector(".form-step .action-btn.primary").click();
		await wait();
		assert(calls.includes(url), "Wrong server used for login");
		input("#ygg-username", "ServerPlayer");
		input("#ygg-password", "test-password");
		await wait();
		document.querySelector(".form-actions .action-btn.primary").click();
		await wait();
		assert(
			launcherStore.settings.user[launcherStore.settings.active_user_idx]
				.yggdrasil_server_url === url,
			"Server account not activated",
		);
		await closeAccount();
	}
	await chooseAccount("Offline");
	await closeAccount();
	assert(
		launcherStore.settings.user.length === 5,
		"Cancelling added an account",
	);
	document.querySelector(".tut-nav .btn-primary").click();
	await wait();
	assert(
		!document.querySelector(".account-options"),
		"Cannot continue after account setup",
	);
	document.querySelectorAll(".tut-dot")[0].click();
	await wait();

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
	await checkProfileAccounts();
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
