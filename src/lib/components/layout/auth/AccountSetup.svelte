<script lang="ts">
	import { onMount, tick } from "svelte";
	import { t } from "$lib/i18n";
	import { launcherStore } from "$lib/state/state.svelte";
	import { saveSettings } from "$lib/api/launcherService";
	import ModalBase from "../ModalBase.svelte";
	import AuthModal from "./AuthModal.svelte";
	import YggdrasilModal from "../YggdrasilModal/YggdrasilModal.svelte";
	import {
		ACCOUNT_SERVER_PRESETS,
		type AccountProvider,
	} from "$lib/utils/accountProviders";

	let {
		provider,
		onclose,
		onsuccess,
	}: {
		provider: AccountProvider;
		onclose: () => void;
		onsuccess?: () => void;
	} = $props();

	let open = $state(true);
	let username = $state("");
	let saving = $state(false);
	let error = $state("");
	let container: HTMLDivElement;

	onMount(() => {
		void tick().then(() => {
			const target =
				container?.querySelector<HTMLElement>(".modal-body input") ??
				container?.querySelector<HTMLElement>(
					".modal-body button, .modal-header button",
				);
			target?.focus();
		});
	});

	function handleKeydown(event: KeyboardEvent) {
		if (event.key !== "Escape" || !open) return;
		event.preventDefault();
		event.stopPropagation();
		open = false;
	}

	$effect(() => {
		if (!open) onclose();
	});

	async function addOffline() {
		const name = username.trim();
		if (saving) return;
		if (!/^[a-zA-Z0-9_]{3,16}$/.test(name)) {
			error = t("userMenu.accountSetup.invalidName");
			return;
		}
		saving = true;
		error = "";
		try {
			let idx = launcherStore.settings.user.findIndex(
				(user) =>
					user.user_type === "Cracked" && user.username === name,
			);
			if (idx < 0) {
				launcherStore.settings.user.push({
					username: name,
					uuid: "",
					access_token: "",
					refresh_token: null,
					user_type: "Cracked",
				});
				idx = launcherStore.settings.user.length - 1;
			}
			launcherStore.settings.active_user_idx = idx;
			await saveSettings();
			onsuccess?.();
			open = false;
		} catch (e: unknown) {
			error = String(e);
		} finally {
			saving = false;
		}
	}
</script>

<svelte:window onkeydowncapture={handleKeydown} />

<div bind:this={container}>
	{#if provider === "offline"}
		<ModalBase bind:open title={t("userMenu.accountSetup.offline")}>
			<form
				class="offline-form"
				onsubmit={(event) => {
					event.preventDefault();
					void addOffline();
				}}
			>
				<label for="account-offline-name"
					>{t("userMenu.usernamePlaceholder")}</label
				>
				<input
					id="account-offline-name"
					bind:value={username}
					placeholder={t("userMenu.usernamePlaceholder")}
					minlength="3"
					maxlength="16"
					pattern={"[a-zA-Z0-9_]{3,16}"}
					title={t("userMenu.accountSetup.invalidName")}
					autocomplete="username"
					required
					disabled={saving}
				/>
				{#if error}<p role="alert">{error}</p>{/if}
				<button
					type="submit"
					class="btn-primary"
					disabled={saving || !username.trim()}
				>
					{t("userMenu.accountSetup.useAccount")}
				</button>
			</form>
		</ModalBase>
	{:else if provider === "premium"}
		<AuthModal bind:open {onsuccess} />
	{:else}
		<YggdrasilModal
			bind:open
			title={t(`userMenu.accountSetup.${provider}`)}
			initialServerUrl={ACCOUNT_SERVER_PRESETS[provider] ?? ""}
			serverInstruction={provider === "cubicAuth"
				? t("userMenu.accountSetup.cubicAuthServer")
				: undefined}
			signupUrl={provider === "cubicAuth"
				? "https://accounts.cubiclauncher.org/"
				: undefined}
			autoConnect={provider === "cubicAuth"}
			{onsuccess}
		/>
	{/if}
</div>

<style>
	.offline-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-md);
		color: var(--text-primary);
	}

	.offline-form input {
		width: 100%;
		box-sizing: border-box;
		padding: 10px 12px;
		background: var(--bg-input);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		font: inherit;
	}

	.offline-form input:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
</style>
