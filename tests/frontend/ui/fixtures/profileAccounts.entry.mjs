import { mount, unmount } from "svelte";
import ProfileView from "$lib/components/profiles/ProfileView.svelte";
import { launcherStore } from "$lib/state/state.svelte";
import { calls } from "$lib/api/cubicApi";

const wait = (ms = 250) => new Promise((resolve) => setTimeout(resolve, ms));
function assert(value, message) {
	if (!value) throw Error(message);
}
function input(selector, value) {
	const element = document.querySelector(selector);
	element.value = value;
	element.dispatchEvent(new Event("input", { bubbles: true }));
}
function escape() {
	document.activeElement.dispatchEvent(
		new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
	);
}

export async function checkProfileAccounts() {
	launcherStore.settings.user = [];
	launcherStore.settings.active_user_idx = 0;
	let closed = false;
	const profile = mount(ProfileView, {
		target: document.body,
		props: {
			onclose: () => {
				closed = true;
			},
		},
	});
	await wait();
	assert(
		document.querySelectorAll(".provider-option").length === 5,
		"Profile must offer five providers",
	);

	// The picker adapts to its actual available width, including the desktop sidebar.
	for (const width of [1200, 800, 360]) {
		window.frameElement.style.width = `${width}px`;
		await wait();
		const grid = document.querySelector(".provider-grid");
		const columns =
			getComputedStyle(grid).gridTemplateColumns.split(" ").length;
		assert(
			columns === (width === 360 ? 1 : 2),
			`Wrong provider columns at ${width}px`,
		);
		const gridBounds = grid.getBoundingClientRect();
		for (const card of grid.children) {
			const bounds = card.getBoundingClientRect();
			assert(
				bounds.left >= gridBounds.left - 1 &&
					bounds.right <= gridBounds.right + 1,
				"Provider card overflows",
			);
			assert(
				card.scrollWidth <= card.clientWidth + 1,
				"Provider text overflows",
			);
		}
		assert(
			document.documentElement.scrollWidth <= innerWidth,
			"Profile overflows horizontally",
		);
	}
	window.frameElement.style.width = "1000px";
	await wait();

	for (const [provider, name, url] of [
		["offline", "ProfilePlayer", null],
		["premium", "PremiumPlayer", null],
		["ely", "ElyPlayer", "https://account.ely.by/api/authlib-injector"],
		["cubicAuth", "CubicPlayer", "https://auth.cubiclauncher.org"],
		[
			"authinject",
			"CustomPlayer",
			"https://custom.example.test/api/yggdrasil",
		],
	]) {
		const trigger = document.querySelector(`[data-provider="${provider}"]`);
		trigger.click();
		await wait();
		assert(
			document.querySelector(".profile-view").inert,
			"Profile must pause during setup",
		);
		if (provider === "offline") {
			assert(
				document.activeElement.id === "account-offline-name",
				"Offline input did not receive focus",
			);
			input("#account-offline-name", name);
			await wait();
			document.querySelector(".offline-form").requestSubmit();
		} else if (url) {
			assert(
				document.querySelector("#ygg-server-url").value ===
					(provider === "authinject" ? "" : url),
				"Incorrect profile server preset",
			);
			if (provider === "authinject") input("#ygg-server-url", url);
			await wait();
			document.querySelector(".form-step .action-btn.primary").click();
			await wait();
			assert(
				calls.includes(url),
				"Profile connected to the wrong server",
			);
			input("#ygg-username", name);
			input("#ygg-password", "test-password");
			await wait();
			document.querySelector(".form-actions .action-btn.primary").click();
		}
		await wait();
		assert(
			document.querySelector(".hero-name").textContent.trim() === name,
			"New account is not selected in profile",
		);
		assert(
			launcherStore.settings.user[launcherStore.settings.active_user_idx]
				.username === name,
			"New account is not active",
		);
		if (provider === "ely" || provider === "cubicAuth") {
			const label = provider === "ely" ? "Ely.by" : "CubicAuth";
			assert(
				document.querySelector(".hero-type").textContent.trim() ===
					label,
				"Profile provider label is wrong",
			);
			assert(
				document
					.querySelector(".account-item.selected .account-type")
					.textContent.trim() === label,
				"Saved account provider label is wrong",
			);
		}
		if (provider !== "offline") {
			escape();
			await wait();
		}
		assert(!closed, "Escape from account setup closed the profile");
		assert(
			!document.querySelector(".modal"),
			"Account dialog did not close",
		);
		assert(
			document.activeElement === trigger,
			"Focus did not return to the provider card",
		);
	}

	// Cancelling preserves both the selected account and the account list.
	document.querySelector('[data-provider="offline"]').click();
	await wait();
	escape();
	await wait();
	assert(
		!closed && launcherStore.settings.user.length === 5,
		"Cancelling altered accounts or closed profile",
	);
	assert(
		document.querySelector(".hero-name").textContent.trim() ===
			"CustomPlayer",
		"Cancelling changed the selection",
	);
	const sidebar = document.querySelector(".profile-list");
	sidebar.scrollTop = sidebar.scrollHeight;
	assert(
		sidebar.scrollTop > 0,
		"Saved accounts should remain reachable below the picker",
	);
	escape();
	assert(
		closed,
		"Escape should close the profile when no account dialog is open",
	);
	await unmount(profile);
	window.frameElement.style.width = "800px";
}
