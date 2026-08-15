<script lang="ts">
	import { launcherStore, showError } from "$lib/state/state.svelte";
	import {
		getAvatar,
		setAvatar,
		buildAvatarUrl,
	} from "$lib/state/avatarCache.svelte";
	import { SvelteMap } from "svelte/reactivity";
	import { fade, fly } from "svelte/transition";
	import {
		saveSettings,
		markLocalSettingsChange,
	} from "$lib/api/launcherService";
	import { t } from "$lib/i18n";
	import { logout, switchUser, removeUser } from "$lib/api/cubicApi";
	import AuthModal from "$lib/components/layout/auth/AuthModal.svelte";
	import YggdrasilModal from "$lib/components/layout/YggdrasilModal/YggdrasilModal.svelte";
	import AddAccountCard from "$lib/components/layout/UserMenu/AddAccountCard.svelte";
	import AccountListItem from "./AccountListItem.svelte";
	import SkinCapeManager from "./SkinCapeManager.svelte";
	import CloseIcon from "$lib/icons/CloseIcon.svelte";
	import Lupa from "$lib/icons/Lupa.svelte";

	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();

	let editingIdx = $state<number | null>(null);
	let editingName = $state("");
	let showAuthModal = $state(false);
	let showYggdrasilModal = $state(false);
	let addingOffline = $state(false);
	let offlineName = $state("");
	let removingUserUuid = $state<string | null>(null);
	let selectedIdx = $state(launcherStore.settings.active_user_idx ?? 0);
	let closing = $state(false);

	function handleClose() {
		if (closing) return;
		closing = true;
		onclose();
	}

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
		selectedIdx = launcherStore.settings.active_user_idx;
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
		selectedIdx = launcherStore.settings.active_user_idx;
		await saveSettings();
	}

	function userKey(u: {
		uuid: string;
		username: string;
		user_type: string;
	}): string {
		return u.uuid || `${u.username}:${u.user_type}`;
	}

	let avatarSvgs = new SvelteMap<string, string>();

	async function loadAvatar(u: {
		uuid: string;
		username: string;
		user_type: string;
	}): Promise<void> {
		const key = userKey(u);
		const url = buildAvatarUrl(u.uuid, u.username, u.user_type);

		const cached = getAvatar(url);
		if (cached !== undefined) {
			if (avatarSvgs.get(key) !== cached) {
				avatarSvgs.set(key, cached);
			}
			return;
		}

		try {
			const res = await fetch(url);
			const svg = await res.text();
			setAvatar(url, svg);
			avatarSvgs.set(key, svg);
		} catch {
			avatarSvgs.set(key, "");
		}
	}

	$effect(() => {
		for (const u of launcherStore.settings.user) {
			loadAvatar(u);
		}
	});

	function getYggdrasilServer(user: {
		yggdrasil_server_url?: string | null;
	}): string {
		return (
			user.yggdrasil_server_url?.split("//")[1]?.split("/")[0] ??
			"Servidor"
		);
	}

	function getUserTypeLabel(
		userType: string,
		yggdrasilServer?: string | null,
	): string {
		if (userType === "Yggdrasil") {
			return `${t("userMenu.authInjector")} - ${getYggdrasilServer({ yggdrasil_server_url: yggdrasilServer })}`;
		}
		if (userType === "Microsoft") {
			return t("userMenu.premium");
		}
		return t("userMenu.offline");
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === "Escape") handleClose();
	}

	function startEditingName() {
		if (!selectedUser) return;
		editingName = selectedUser.username;
		editingIdx = selectedIdx;
	}

	function cancelEditingName() {
		editingIdx = null;
	}

	const selectedUser = $derived(launcherStore.settings.user[selectedIdx]);
	const selectedUserKey = $derived(selectedUser ? userKey(selectedUser) : "");
	const selectedAvatarSvg = $derived(
		selectedUserKey ? (avatarSvgs.get(selectedUserKey) ?? "") : "",
	);
	const activeUserIdx = $derived(launcherStore.settings.active_user_idx ?? 0);
	const totalAccounts = $derived(launcherStore.settings.user.length);
</script>

<svelte:window onkeydown={onKeydown} />

<div class="profile-view" transition:fly={{ y: 16, duration: 200 }}>
	<header class="profile-header">
		<h2 class="profile-title">{t("userMenu.title")}</h2>
		<button
			type="button"
			class="close-btn"
			onclick={handleClose}
			title={t("userMenu.close")}
			aria-label={t("userMenu.close")}
		>
			<CloseIcon size={18} />
		</button>
	</header>

	<div class="profile-layout">
		<section class="profile-hero">
			{#if selectedUser}
				<div class="hero-banner"></div>

				<div class="hero-content">
					<div class="hero-identity">
						<div class="hero-avatar">
							{#if selectedAvatarSvg}
								{@html selectedAvatarSvg}
							{/if}
						</div>
						<div class="hero-meta">
							{#if editingIdx === selectedIdx}
								<div class="hero-edit-row">
									<!-- svelte-ignore a11y_autofocus -->
									<input
										type="text"
										bind:value={editingName}
										maxlength="16"
										autofocus
										class="hero-name-input"
										onkeydown={(e) => {
											if (e.key === "Enter")
												handleSaveName(selectedIdx);
											if (e.key === "Escape")
												cancelEditingName();
										}}
									/>
									<button
										type="button"
										class="btn-icon btn-icon-primary"
										onclick={() =>
											handleSaveName(selectedIdx)}
									>
										✓
									</button>
									<button
										type="button"
										class="btn-icon btn-icon-secondary"
										onclick={cancelEditingName}
									>
										✕
									</button>
								</div>
							{:else}
								<h3 class="hero-name">
									{selectedUser.username}
								</h3>
							{/if}
							<span class="hero-type">
								{getUserTypeLabel(
									selectedUser.user_type,
									selectedUser.yggdrasil_server_url,
								)}
							</span>
						</div>

						{#if selectedIdx === activeUserIdx}
							<span class="hero-status"
								>{t("userMenu.active")}</span
							>
						{:else}
							<button
								type="button"
								class="btn-primary"
								onclick={() => handleSwitchUser(selectedIdx)}
							>
								{t("userMenu.activate")}
							</button>
						{/if}
					</div>

					<div class="hero-stats">
						<div class="stat-card">
							<span class="stat-value">{totalAccounts}</span>
							<span class="stat-label">
								{totalAccounts === 1
									? t("userMenu.accountSingular")
									: t("userMenu.accountPlural")}
							</span>
						</div>
						<div class="stat-card">
							<span class="stat-value">
								{#if selectedUser.user_type === "Microsoft"}
									Microsoft
								{:else if selectedUser.user_type === "Yggdrasil"}
									Authlib
								{:else}
									Offline
								{/if}
							</span>
							<span class="stat-label">{t("userMenu.type")}</span>
						</div>
						{#if selectedUser.user_type === "Yggdrasil"}
							<div class="stat-card wide">
								<span class="stat-value">
									{getYggdrasilServer(selectedUser)}
								</span>
								<span class="stat-label"
									>{t("userMenu.server")}</span
								>
							</div>
						{/if}
					</div>

					<div class="hero-actions">
						{#if selectedUser.user_type === "Cracked"}
							{#if editingIdx !== selectedIdx}
								<button
									type="button"
									class="btn-secondary"
									onclick={startEditingName}
								>
									{t("userMenu.editName")}
								</button>
							{/if}
						{/if}

						{#if selectedUser.user_type === "Microsoft" && selectedIdx === activeUserIdx}
							<button
								type="button"
								class="btn-secondary"
								onclick={handleLogout}
							>
								{t("userMenu.logout")}
							</button>
						{/if}

						{#if removingUserUuid === selectedUser.uuid}
							<div class="confirm-group">
								<button
									type="button"
									class="btn-primary confirm-yes"
									onclick={() =>
										handleRemoveUser(selectedUser.uuid)}
								>
									{t("userMenu.yes")}
								</button>
								<button
									type="button"
									class="btn-secondary confirm-no"
									onclick={() => (removingUserUuid = null)}
								>
									{t("userMenu.no")}
								</button>
							</div>
						{:else}
							<button
								type="button"
								class="btn-danger"
								onclick={() =>
									(removingUserUuid = selectedUser.uuid)}
							>
								{t("userMenu.removeUser")}
							</button>
						{/if}
					</div>

					{#key selectedUserKey}
						{#if selectedUser.user_type === "Microsoft"}
							<div
								class="hero-skin-cape"
								in:fade={{ duration: 150 }}
							>
								<SkinCapeManager uuid={selectedUser.uuid} />
							</div>
						{/if}
					{/key}
				</div>
			{:else}
				<div class="hero-empty">
					<span class="hero-empty-icon">
						<Lupa width="48" height="48" />
					</span>
					<h3 class="hero-empty-title">
						{t("userMenu.noAccountsTitle")}
					</h3>
					<p class="hero-empty-subtitle">
						{t("userMenu.noAccountsSubtitle")}
					</p>
				</div>
			{/if}
		</section>

		<aside class="profile-list">
			<AddAccountCard
				bind:addingOffline
				bind:offlineName
				onAddOffline={handleAddOffline}
				onOpenAuth={() => (showAuthModal = true)}
				onOpenYggdrasil={() => (showYggdrasilModal = true)}
				{showYggdrasilModal}
			/>

			{#if launcherStore.settings.user.length > 0}
				<h3 class="list-title">{t("userMenu.savedAccounts")}</h3>
				<div class="account-list">
					{#each launcherStore.settings.user as u, i (i)}
						<AccountListItem
							user={u}
							typeLabel={getUserTypeLabel(
								u.user_type,
								u.yggdrasil_server_url,
							)}
							isActive={i === activeUserIdx}
							isSelected={i === selectedIdx}
							avatarSvg={avatarSvgs.get(userKey(u)) ?? ""}
							onselect={() => {
								selectedIdx = i;
								editingIdx = null;
							}}
						/>
					{/each}
				</div>
			{:else}
				<div class="list-empty">
					<p>{t("userMenu.addFirstAccount")}</p>
				</div>
			{/if}
		</aside>
	</div>
</div>

<AuthModal bind:open={showAuthModal} />
<YggdrasilModal bind:open={showYggdrasilModal} />

<style>
	.profile-view {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--bg-main);
		z-index: 5;
	}

	.profile-header {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 28px;
		border-bottom: 1px solid var(--border);
		background: var(--bg-card);
	}

	.profile-title {
		margin: 0;
		font-size: 1rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 1px;
		color: var(--text-primary);
	}

	.close-btn {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		border-radius: var(--border-radius-sm);
		padding: 6px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			background 0.15s ease,
			color 0.15s ease,
			border-color 0.15s ease;
	}

	.close-btn:hover {
		background: var(--surface-selected);
		color: var(--text-primary);
		border-color: var(--text-muted);
	}

	.profile-layout {
		display: flex;
		flex: 1;
		overflow: hidden;
	}

	.profile-hero {
		flex: 3;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow-y: auto;
		background: var(--bg-main);
		position: relative;
	}

	.hero-banner {
		height: 180px;
		flex-shrink: 0;
		background: linear-gradient(
			135deg,
			color-mix(in srgb, var(--accent) 70%, transparent) 0%,
			color-mix(in srgb, var(--accent) 20%, transparent) 100%
		);
		position: relative;
	}

	.hero-banner::after {
		content: "";
		position: absolute;
		inset: 0;
		background: linear-gradient(
			to bottom,
			transparent 0%,
			var(--bg-main) 100%
		);
	}

	.hero-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		padding: 0 40px 40px;
		margin-top: -72px;
		position: relative;
		z-index: 1;
		gap: 24px;
	}

	.hero-identity {
		display: flex;
		align-items: flex-end;
		gap: 20px;
		flex-wrap: wrap;
	}

	.hero-avatar {
		width: 144px;
		height: 144px;
		border-radius: var(--border-radius);
		border: 4px solid var(--bg-main);
		background: var(--cubic-logo) center/50% no-repeat;
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.35);
	}

	.hero-avatar :global(svg) {
		width: 100%;
		height: 100%;
		display: block;
		border-radius: inherit;
	}

	.hero-meta {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
		padding-bottom: 12px;
		flex: 1;
	}

	.hero-name {
		margin: 0;
		font-size: 2.2rem;
		font-weight: 800;
		color: var(--text-primary);
		line-height: 1.1;
		word-break: break-word;
	}

	.hero-edit-row {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 4px;
	}

	.hero-name-input {
		font-family: inherit;
		font-size: 1.8rem;
		font-weight: 700;
		color: var(--text-primary);
		background: var(--bg-input);
		border: 1px solid var(--accent);
		border-radius: var(--border-radius-sm);
		padding: 6px 10px;
		outline: none;
		min-width: 0;
		width: 280px;
	}

	.hero-type {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.hero-status {
		font-size: 0.75rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		background: var(--accent);
		color: var(--accent-text);
		padding: 8px 16px;
		border-radius: 999px;
		margin-bottom: 12px;
		flex-shrink: 0;
	}

	.hero-stats {
		display: flex;
		gap: 12px;
		flex-wrap: wrap;
	}

	.stat-card {
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		padding: 16px 22px;
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 120px;
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 var(--surface-selected);
	}

	.stat-card.wide {
		min-width: 180px;
		flex: 1;
	}

	.stat-value {
		font-size: 1.15rem;
		font-weight: 700;
		color: var(--text-primary);
	}

	.stat-label {
		font-size: 0.65rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.hero-actions {
		display: flex;
		gap: 10px;
		align-items: center;
		flex-wrap: wrap;
		margin-top: auto;
		padding-top: 8px;
	}

	.hero-skin-cape {
		width: 100%;
		max-width: 640px;
		border-top: 1px solid var(--border);
		padding-top: 20px;
	}

	.btn-primary,
	.btn-secondary,
	.btn-danger {
		font-family: inherit;
		font-size: 0.82rem;
		font-weight: 600;
		padding: 10px 20px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition: all 0.15s ease;
		border: 1px solid transparent;
	}

	.btn-primary {
		background: var(--accent);
		color: var(--accent-text);
	}

	.btn-primary:hover {
		opacity: 0.85;
	}

	.btn-secondary {
		background: transparent;
		border-color: var(--border);
		color: var(--text-secondary);
	}

	.btn-secondary:hover {
		background: var(--surface-selected);
		color: var(--text-primary);
	}

	.btn-danger {
		background: rgba(var(--color-error-rgb), 0.1);
		border-color: rgba(var(--color-error-rgb), 0.25);
		color: var(--color-error);
	}

	.btn-danger:hover {
		background: rgba(var(--color-error-rgb), 0.18);
	}

	.btn-icon {
		width: 34px;
		height: 34px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--border-radius-sm);
		font-size: 1rem;
		cursor: pointer;
		border: 1px solid transparent;
		transition: all 0.15s ease;
	}

	.btn-icon-primary {
		background: var(--accent);
		color: var(--accent-text);
	}

	.btn-icon-primary:hover {
		opacity: 0.85;
	}

	.btn-icon-secondary {
		background: transparent;
		border-color: var(--border);
		color: var(--text-secondary);
	}

	.btn-icon-secondary:hover {
		background: var(--surface-selected);
		color: var(--text-primary);
	}

	.confirm-group {
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.confirm-yes:hover {
		background: var(--color-success);
		color: var(--bg-main);
		opacity: 1;
	}

	.confirm-no:hover {
		background: var(--color-error);
		color: var(--bg-main);
		opacity: 1;
	}

	.hero-empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 12px;
		color: var(--text-secondary);
		text-align: center;
		padding: 40px;
	}

	.hero-empty-icon {
		opacity: 0.7;
	}

	.hero-empty-title {
		margin: 0;
		font-size: 1.3rem;
		font-weight: 700;
		color: var(--text-primary);
	}

	.hero-empty-subtitle {
		margin: 0;
		font-size: 0.85rem;
		color: var(--text-muted);
	}

	.profile-list {
		flex: 2;
		min-width: 300px;
		max-width: 440px;
		background: var(--bg-sidebar);
		border-left: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		overflow: hidden;
		padding: 20px;
		gap: 16px;
	}

	.list-title {
		margin: 0;
		font-size: 0.75rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.6px;
	}

	.account-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
		overflow-y: auto;
		flex: 1;
	}

	.list-empty {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 20px;
		color: var(--text-muted);
		font-size: 0.85rem;
		text-align: center;
		flex: 1;
	}

	@media (max-width: 900px) {
		.profile-layout {
			flex-direction: column;
			overflow-y: auto;
		}

		.profile-hero {
			flex: none;
			min-height: auto;
		}

		.hero-banner {
			height: 140px;
		}

		.hero-content {
			margin-top: -56px;
			padding: 0 20px 28px;
			gap: 20px;
		}

		.hero-avatar {
			width: 112px;
			height: 112px;
		}

		.hero-name {
			font-size: 1.8rem;
		}

		.hero-name-input {
			font-size: 1.4rem;
			width: 200px;
		}

		.hero-identity {
			gap: 14px;
		}

		.profile-list {
			flex: none;
			max-width: none;
			min-width: 0;
			border-left: none;
			border-top: 1px solid var(--border);
			padding: 20px;
			overflow: visible;
		}

		.account-list {
			overflow: visible;
			flex: none;
		}
	}
</style>
