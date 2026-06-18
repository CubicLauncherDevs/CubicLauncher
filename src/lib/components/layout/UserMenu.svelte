<script lang="ts">
	import { launcherStore, showError } from "$lib/state/state.svelte";
	import { SvelteMap } from "svelte/reactivity";
	import {
		saveSettings,
		markLocalSettingsChange,
	} from "$lib/api/launcherService";
	import { t } from "$lib/i18n";
	import { logout, switchUser, removeUser } from "$lib/api/cubicApi";
	import AuthModal from "./AuthModal.svelte";
	import YggdrasilModal from "./YggdrasilModal.svelte";
	import ModalBase from "./ModalBase.svelte";
	import CloseIcon from "$lib/icons/CloseIcon.svelte";

	let { open = $bindable(false) } = $props<{ open: boolean }>();

	let editingIdx = $state<number | null>(null);
	let editingName = $state("");
	let showAuthModal = $state(false);
	let showYggdrasilModal = $state(false);
	let addingOffline = $state(false);
	let offlineName = $state("");
	let removingUser = $state<string | null>(null);

	$effect(() => {
		if (open) {
			editingIdx = null;
			removingUser = null;
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

	function handleNameKeydown(e: KeyboardEvent, idx: number) {
		if (e.key === "Enter") handleSaveName(idx);
		if (e.key === "Escape") {
			editingIdx = null;
		}
	}

	async function handleLogout() {
		markLocalSettingsChange();
		await logout();
	}

	async function handleSwitchUser(idx: number) {
		if (idx === launcherStore.settings.active_user_idx) return;
		launcherStore.settings.active_user_idx = idx;
		removingUser = null;
		editingIdx = null;
		markLocalSettingsChange();
		await switchUser(idx);
	}

	async function handleRemoveUser(username: string) {
		const idx = launcherStore.settings.user.findIndex(
			(u) => u.username === username,
		);
		if (idx === -1) return;
		launcherStore.settings.user = [
			...launcherStore.settings.user.slice(0, idx),
			...launcherStore.settings.user.slice(idx + 1),
		];
		if (launcherStore.settings.user.length === 0) {
			launcherStore.settings.user = [
				{
					username: "Steve",
					uuid: "",
					access_token: "",
					refresh_token: null,
					user_type: "Cracked",
				},
			];
			launcherStore.settings.active_user_idx = 0;
		} else if (idx <= launcherStore.settings.active_user_idx) {
			launcherStore.settings.active_user_idx = Math.max(
				0,
				launcherStore.settings.active_user_idx - 1,
			);
		}
		removingUser = null;
		markLocalSettingsChange();
		await removeUser(username);
	}

	async function handleAddOffline() {
		const name = offlineName.trim();
		if (!name) return;
		launcherStore.settings.user = [
			...launcherStore.settings.user,
			{
				username: name,
				uuid: "",
				access_token: "",
				refresh_token: null,
				user_type: "Cracked",
			},
		];
		launcherStore.settings.active_user_idx =
			launcherStore.settings.user.length - 1;
		addingOffline = false;
		offlineName = "";
		await saveSettings();
	}

	const avatarCache = new SvelteMap<string, string>();

	let avatarSvgs = $state<Record<string, string>>({});

	async function loadAvatar(
		username: string,
		userType: string,
	): Promise<void> {
		const endpoint = userType === "Yggdrasil" ? "elyby" : "mojang";
		const url = `https://bohrium-js.cubiclauncher.com/api/${endpoint}/head/${username}`;

		const cached = avatarCache.get(url);
		if (cached !== undefined) {
			if (avatarSvgs[username] !== cached) {
				avatarSvgs = { ...avatarSvgs, [username]: cached };
			}
			return;
		}

		try {
			const res = await fetch(url);
			const svg = await res.text();
			avatarCache.set(url, svg);
			avatarSvgs = { ...avatarSvgs, [username]: svg };
		} catch {
			avatarSvgs = { ...avatarSvgs, [username]: "" };
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
		<!-- Add account card -->
		<div class="card add-card">
			<div class="add-toggle">
				<button
					type="button"
					class="add-toggle-btn"
					class:active={addingOffline}
					onclick={() => (addingOffline = true)}
				>
					{t("userMenu.addOffline")}
				</button>
				<button
					type="button"
					class="add-toggle-btn"
					class:active={!addingOffline && !showYggdrasilModal}
					onclick={() => (showAuthModal = true)}
				>
					{t("userMenu.loginMicrosoft")}
				</button>
				<button
					type="button"
					class="add-toggle-btn ygg"
					onclick={() => (showYggdrasilModal = true)}
				>
					Authlib
				</button>
			</div>
			{#if addingOffline}
				<div class="add-form">
					<input
						type="text"
						bind:value={offlineName}
						placeholder={t("userMenu.usernamePlaceholder")}
						maxlength="16"
						class="env-input"
						onkeydown={(e) =>
							e.key === "Enter" && handleAddOffline()}
					/>
					<div class="add-form-actions">
						<button
							type="button"
							class="btn-primary"
							onclick={handleAddOffline}
							>{t("userMenu.add")}</button
						>
						<button
							type="button"
							class="btn-secondary"
							onclick={() => {
								addingOffline = false;
								offlineName = "";
							}}>{t("userMenu.cancel")}</button
						>
					</div>
				</div>
			{/if}
		</div>

		<!-- Saved Accounts -->
		{#if launcherStore.settings.user.length > 0}
			<span class="section-label">{t("userMenu.savedAccounts")}</span>
			<div class="user-list">
				{#each launcherStore.settings.user as u, i (i)}
					<div
						class="card user-card"
						class:active={i ===
							launcherStore.settings.active_user_idx}
						onclick={() => handleSwitchUser(i)}
						role="button"
						tabindex="0"
						onkeydown={(e) =>
							e.key === "Enter" && handleSwitchUser(i)}
					>
						<div class="user-card-row">
							<div class="user-avatar-wrapper">
								{#if avatarSvgs[u.username]}
									{@html avatarSvgs[u.username]}
								{/if}
							</div>
							<div class="user-info">
								{#if u.user_type === "Cracked" && editingIdx === i}
									<!-- svelte-ignore a11y_autofocus -->
									<input
										type="text"
										bind:value={editingName}
										onkeydown={(e) =>
											handleNameKeydown(e, i)}
										onblur={() => {
											if (editingIdx === i)
												handleSaveName(i);
										}}
										maxlength="16"
										class="user-name-input"
										autofocus
									/>
								{:else}
									<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
									<span
										class="user-name"
										class:clickable={u.user_type ===
											"Cracked"}
										onclick={(e) => {
											if (u.user_type === "Cracked") {
												e.stopPropagation();
												editingIdx = i;
												editingName = u.username;
											}
										}}
										role={u.user_type === "Cracked"
											? "button"
											: undefined}
										tabindex={u.user_type === "Cracked"
											? 0
											: undefined}
										onkeydown={(e) => {
											if (
												u.user_type === "Cracked" &&
												(e.key === "Enter" ||
													e.key === " ")
											) {
												e.stopPropagation();
												editingIdx = i;
												editingName = u.username;
											}
										}}>{u.username}</span
									>
								{/if}
								<span class="user-type">
									{#if u.user_type === "Yggdrasil"}
										{t("userMenu.authInjector")} - {u.yggdrasil_server_url
											?.split("//")[1]
											?.split("/")[0] ?? "Servidor"}
									{:else if u.user_type === "Microsoft"}
										{t("userMenu.premium")}
									{:else}
										{t("userMenu.offline")}
									{/if}
								</span>
							</div>
							<div class="user-badges">
								{#if i === launcherStore.settings.active_user_idx}
									<span class="active-badge"
										>{t("userMenu.active")}</span
									>
								{/if}
							</div>
							<div class="user-actions">
								{#if i === launcherStore.settings.active_user_idx && u.user_type === "Microsoft"}
									<button
										type="button"
										class="icon-btn"
										title={t("userMenu.logout")}
										onclick={(e) => {
											e.stopPropagation();
											handleLogout();
										}}
									>
										<svg
											width="14"
											height="14"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2"
											stroke-linecap="round"
											stroke-linejoin="round"
											><path
												d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"
											/><polyline
												points="16 17 21 12 16 7"
											/><line
												x1="21"
												y1="12"
												x2="9"
												y2="12"
											/></svg
										>
									</button>
								{/if}
								{#if removingUser === u.username}
									<div class="confirm-group">
										<!-- svelte-ignore a11y_consider_explicit_label -->
										<button
											type="button"
											class="icon-btn confirm-yes"
											onclick={(e) => {
												e.stopPropagation();
												handleRemoveUser(u.username);
											}}
										>
											<svg
												width="12"
												height="12"
												viewBox="0 0 24 24"
												fill="none"
												stroke="currentColor"
												stroke-width="3"
												><polyline
													points="20 6 9 17 4 12"
												/></svg
											>
										</button>
										<button
											type="button"
											class="icon-btn confirm-no"
											onclick={(e) => {
												e.stopPropagation();
												removingUser = null;
											}}
										>
											<CloseIcon size={12} />
										</button>
									</div>
								{:else}
									<button
										type="button"
										class="icon-btn remove"
										title={t("userMenu.removeUser")}
										onclick={(e) => {
											e.stopPropagation();
											removingUser = u.username;
										}}
									>
										<svg
											width="12"
											height="12"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2"
											><polyline
												points="3 6 5 6 21 6"
											/><path
												d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
											/></svg
										>
									</button>
								{/if}
							</div>
						</div>
					</div>
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

	/* ── Card base ─────────────────────────────────────── */

	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 rgba(255, 255, 255, 0.03);
		overflow: hidden;
	}

	/* ── Add account card (toggle + form) ──────────────── */

	.add-card {
		display: flex;
		flex-direction: column;
	}

	.add-toggle {
		display: flex;
		gap: 0;
	}

	.add-toggle-btn {
		flex: 1;
		padding: 10px;
		font-size: 0.78rem;
		font-weight: 600;
		cursor: pointer;
		text-align: center;
		font-family: inherit;
		background: var(--bg-input);
		border: none;
		color: var(--text-secondary);
		transition: all 0.15s;
	}
	.add-toggle-btn:first-child {
		border-right: 1px solid var(--border-color);
	}
	.add-toggle-btn:last-child {
		border-left: 1px solid var(--border-color);
	}
	.add-toggle-btn.active {
		background: var(--accent);
		color: var(--accent-text);
	}
	.add-toggle-btn:hover:not(.active) {
		background: rgba(255, 255, 255, 0.04);
		color: var(--text-primary);
	}

	.add-form {
		display: flex;
		gap: 8px;
		padding: 12px 14px;
		border-top: 1px solid var(--border-color);
		align-items: center;
	}

	.add-form-actions {
		display: flex;
		gap: 6px;
		flex-shrink: 0;
	}

	/* ── Env-style input ───────────────────────────────── */

	.env-input {
		flex: 1;
		min-width: 0;
		width: 0;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 4px 8px;
		border-radius: var(--border-radius-sm);
		font-size: 0.8rem;
		height: 28px;
		box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.2);
		box-sizing: border-box;
	}
	.env-input:focus {
		outline: none;
		border-color: var(--text-muted);
	}

	/* ── Section label ─────────────────────────────────── */

	.section-label {
		font-size: 0.7rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--text-secondary);
		margin-top: 4px;
	}

	/* ── User list ─────────────────────────────────────── */

	.user-list {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.user-card {
		cursor: pointer;
		transition: border-color 0.15s;
	}
	.user-card:hover {
		border-color: rgba(255, 255, 255, 0.1);
	}
	.user-card.active {
		border-color: var(--accent);
	}

	.user-card-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 12px;
	}

	/* ── Avatar ────────────────────────────────────────── */

	.user-avatar-wrapper {
		width: 32px;
		height: 32px;
		border-radius: var(--border-radius-sm);
		border: 1px solid var(--border-color);
		flex-shrink: 0;
		background: url("/images/cubic.svg") center/60% no-repeat;
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.user-avatar-wrapper :global(svg) {
		width: 100%;
		height: 100%;
		display: block;
		border-radius: inherit;
	}

	/* ── User info ─────────────────────────────────────── */

	.user-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
		flex: 1;
	}

	.user-name {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.user-name.clickable {
		cursor: text;
		border-bottom: 1px dashed rgba(255, 255, 255, 0.1);
	}
	.user-name.clickable:hover {
		border-bottom-color: var(--accent);
	}

	.user-name-input {
		font-size: 0.85rem;
		font-weight: 600;
		padding: 2px 4px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid var(--accent);
		border-radius: 3px;
		color: var(--text-primary);
		outline: none;
		width: 100%;
		box-sizing: border-box;
	}

	.user-type {
		font-size: 0.6rem;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.3px;
	}

	/* ── Badges ────────────────────────────────────────── */

	.user-badges {
		display: flex;
		gap: 4px;
		flex-shrink: 0;
	}

	.active-badge {
		font-size: 0.55rem;
		background: var(--accent);
		color: var(--bg-main);
		padding: 2px 6px;
		border-radius: 4px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.3px;
	}

	/* ── Action icons ──────────────────────────────────── */

	.user-actions {
		display: flex;
		gap: 2px;
		flex-shrink: 0;
	}

	.icon-btn {
		background: none;
		border: 1px solid transparent;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 4px;
		border-radius: 4px;
		display: flex;
		transition: all 0.15s;
	}
	.icon-btn:hover {
		color: var(--text-primary);
		border-color: var(--border-color);
		background: rgba(255, 255, 255, 0.03);
	}
	.icon-btn.remove:hover {
		color: var(--color-error);
		border-color: rgba(var(--color-error-rgb), 0.2);
		background: rgba(var(--color-error-rgb), 0.08);
	}

	.confirm-group {
		display: flex;
		gap: 2px;
	}
	.confirm-yes:hover {
		color: var(--color-success);
		border-color: rgba(var(--color-success-rgb), 0.2);
		background: rgba(var(--color-success-rgb), 0.08);
	}
	.confirm-no:hover {
		color: var(--color-error);
		border-color: rgba(var(--color-error-rgb), 0.2);
		background: rgba(var(--color-error-rgb), 0.08);
	}

	/* ── Shared button styles ──────────────────────────── */

	.btn-primary {
		background: var(--accent);
		color: var(--accent-text);
		border: none;
		cursor: pointer;
		padding: 5px 14px;
		border-radius: var(--border-radius-sm);
		font-size: 0.75rem;
		font-weight: 600;
		transition: opacity 0.15s;
	}
	.btn-primary:hover {
		opacity: 0.85;
	}

	.btn-secondary {
		background: transparent;
		border: 1px solid var(--border-color);
		color: var(--text-secondary);
		cursor: pointer;
		padding: 5px 14px;
		border-radius: var(--border-radius-sm);
		font-size: 0.75rem;
		font-weight: 600;
		transition: all 0.15s;
	}
	.btn-secondary:hover {
		background: rgba(255, 255, 255, 0.03);
		color: var(--text-primary);
	}
</style>
