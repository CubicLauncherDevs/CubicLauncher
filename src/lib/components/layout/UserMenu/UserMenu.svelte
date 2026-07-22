<script lang="ts">
	import { launcherStore, showError } from "$lib/state/state.svelte";
	import { getAvatar, setAvatar } from "$lib/state/avatarCache";
	import { SvelteMap } from "svelte/reactivity";
	import {
		saveSettings,
		markLocalSettingsChange,
	} from "$lib/api/launcherService";
	import { t } from "$lib/i18n";
	import { logout, switchUser, removeUser } from "$lib/api/cubicApi";
	import AuthModal from "../auth/AuthModal.svelte";
	import YggdrasilModal from "../YggdrasilModal/YggdrasilModal.svelte";
	import ModalBase from "../ModalBase.svelte";
	import AddAccountCard from "./AddAccountCard.svelte";
	import UserCard from "./UserCard.svelte";

	let { open = $bindable(false) } = $props<{ open: boolean }>();

	let editingIdx = $state<number | null>(null);
	let editingName = $state("");
	let showAuthModal = $state(false);
	let showYggdrasilModal = $state(false);
	let addingOffline = $state(false);
	let offlineName = $state("");
	let removingUserUuid = $state<string | null>(null);

	$effect(() => {
		if (open) {
			editingIdx = null;
			removingUserUuid = null;
			addingOffline = false;
			offlineName = "";
		}
	});

	async function handleSaveName(idx: number) {
		const regex = /^[a-zA-Z0-9_]{3,16}$/;
		if (!regex.test(editingName)) {
			showError(
				"Nombre Inválido",
				"El nombre debe tener entre 3 y 16 caracteres y solo contener letras, números y guiones bajos (_).",
			);
			return;
		}
		const user = launcherStore.settings.user[idx];
		if (user) {
			user.username = editingName;
			await saveSettings();
		}
		editingIdx = null;
	}

	async function handleLogout() {
		markLocalSettingsChange();
		await logout();
	}

	async function handleSwitchUser(idx: number) {
		if (idx === launcherStore.settings.active_user_idx) return;
		launcherStore.settings.active_user_idx = idx;
		removingUserUuid = null;
		editingIdx = null;
		markLocalSettingsChange();
		const user = await switchUser(idx);
		if (user) {
			launcherStore.settings.user[idx] = user;
		}
	}

	async function handleRemoveUser(uuid: string) {
		const idx = launcherStore.settings.user.findIndex(
			(u) => u.uuid === uuid,
		);
		if (idx === -1) return;
		launcherStore.settings.user.splice(idx, 1);
		if (launcherStore.settings.user.length === 0) {
			launcherStore.settings.user.push({
				username: "Steve",
				uuid: "",
				access_token: "",
				refresh_token: null,
				user_type: "Cracked",
			});
			launcherStore.settings.active_user_idx = 0;
		} else if (idx <= launcherStore.settings.active_user_idx) {
			launcherStore.settings.active_user_idx = Math.max(
				0,
				launcherStore.settings.active_user_idx - 1,
			);
		}
		removingUserUuid = null;
		markLocalSettingsChange();
		await removeUser(uuid);
	}

	async function handleAddOffline() {
		const name = offlineName.trim();
		if (!name) return;
		launcherStore.settings.user.push({
			username: name,
			uuid: "",
			access_token: "",
			refresh_token: null,
			user_type: "Cracked",
		});
		launcherStore.settings.active_user_idx =
			launcherStore.settings.user.length - 1;
		addingOffline = false;
		offlineName = "";
		await saveSettings();
	}

	let avatarSvgs = new SvelteMap<string, string>();

	async function loadAvatar(
		username: string,
		userType: string,
	): Promise<void> {
		const endpoint = userType === "Yggdrasil" ? "elyby" : "mojang";
		const url = `https://skins.cubiclauncher.org/api/${endpoint}/head/${username}`;

		const cached = getAvatar(url);
		if (cached !== undefined) {
			if (avatarSvgs.get(username) !== cached) {
				avatarSvgs.set(username, cached);
			}
			return;
		}

		try {
			const res = await fetch(url);
			const svg = await res.text();
			setAvatar(url, svg);
			avatarSvgs.set(username, svg);
		} catch {
			avatarSvgs.set(username, "");
		}
	}

	$effect(() => {
		if (!open) return;
		for (const u of launcherStore.settings.user) {
			loadAvatar(u.username, u.user_type);
		}
	});
</script>

<ModalBase bind:open title={t("userMenu.title")}>
	<div class="um-cards">
		<AddAccountCard
			bind:addingOffline
			bind:offlineName
			onAddOffline={handleAddOffline}
			onOpenAuth={() => (showAuthModal = true)}
			onOpenYggdrasil={() => (showYggdrasilModal = true)}
			{showYggdrasilModal}
		/>

		{#if launcherStore.settings.user.length > 0}
			<span class="section-label">{t("userMenu.savedAccounts")}</span>
			<div class="user-list">
				{#each launcherStore.settings.user as u, i (i)}
					<UserCard
						user={u}
						isActive={i === launcherStore.settings.active_user_idx}
						avatarSvg={avatarSvgs.get(u.username) ?? ""}
						isEditing={editingIdx === i}
						bind:editingName
						isConfirmingRemove={removingUserUuid === u.uuid}
						onswitch={() => handleSwitchUser(i)}
						onstartedit={() => {
							editingIdx = i;
							editingName = u.username;
						}}
						onsavename={() => handleSaveName(i)}
						oncancelname={() => (editingIdx = null)}
						onlogout={handleLogout}
						onstartremove={() => (removingUserUuid = u.uuid)}
						onconfirmremove={() => handleRemoveUser(u.uuid)}
						oncancelremove={() => (removingUserUuid = null)}
					/>
				{/each}
			</div>
		{/if}
	</div>
</ModalBase>

<AuthModal bind:open={showAuthModal} />
<YggdrasilModal bind:open={showYggdrasilModal} />

<style>
	.um-cards {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.section-label {
		font-size: 0.7rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--text-secondary);
		margin-top: 4px;
	}

	.user-list {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
</style>
