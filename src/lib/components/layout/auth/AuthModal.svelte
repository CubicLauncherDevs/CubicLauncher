<script lang="ts">
	import { t } from "$lib/i18n";
	import { startWebviewAuth } from "$lib/api/cubicApi";
	import { saveSettings } from "$lib/api/launcherService";
	import { launcherStore } from "$lib/state/state.svelte";
	import ModalBase from "../ModalBase.svelte";
	import AuthLoading from "./AuthLoading.svelte";
	import AuthError from "./AuthError.svelte";
	import AuthSuccess from "./AuthSuccess.svelte";

	let { open = $bindable(false) } = $props<{ open: boolean }>();

	let loading = $state(true);
	let error = $state<string | null>(null);
	let success = $state(false);

	async function startAuth() {
		try {
			loading = true;
			error = null;

			const user = await startWebviewAuth();

			const idx = launcherStore.settings.user.findIndex(
				(u) => u.username === user.username,
			);
			if (idx >= 0) {
				launcherStore.settings.user[idx] = user;
				launcherStore.settings.active_user_idx = idx;
			} else {
				launcherStore.settings.user.push(user);
				launcherStore.settings.active_user_idx =
					launcherStore.settings.user.length - 1;
			}
			await saveSettings();
			success = true;

			setTimeout(() => {
				open = false;
			}, 2000);
		} catch (e: unknown) {
			console.error("Auth error:", e);
			error = String(e);
			loading = false;
		}
	}

	$effect(() => {
		if (open) {
			loading = true;
			error = null;
			success = false;
			startAuth();
		}
	});
</script>

<ModalBase bind:open title={t("userMenu.authModal.title")}>
	<div class="auth-container">
		<div class="ms-logo-wrapper">
			<svg
				class="ms-logo"
				viewBox="0 0 21 21"
				xmlns="http://www.w3.org/2000/svg"
			>
				<rect x="1" y="1" width="9" height="9" fill="#f25022" />
				<rect x="11" y="1" width="9" height="9" fill="#7fba00" />
				<rect x="1" y="11" width="9" height="9" fill="#00a4ef" />
				<rect x="11" y="11" width="9" height="9" fill="#ffb900" />
			</svg>
		</div>

		{#if loading}
			<AuthLoading
				title={t("userMenu.authModal.loading") || "Cargando..."}
				subtitle={t("userMenu.authModal.waiting") ||
					"Inicia sesión en la ventana que se abrirá..."}
			/>
		{:else if error}
			<AuthError
				title="Error de autenticación"
				message={t("userMenu.authModal.error")?.replace("{error}", error) ||
					error}
				onRetry={startAuth}
			/>
		{:else if success}
			<AuthSuccess
				subtitle={t("userMenu.authModal.success") ||
					"Tu cuenta ha sido vinculada."}
			/>
		{/if}
	</div>
</ModalBase>

<style>
	.auth-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		padding: 0;
		width: 100%;
		color: var(--text-primary);
		position: relative;
	}

	.ms-logo-wrapper {
		margin-bottom: 1.5rem;
		padding: 1rem;
		background: var(--bg-card);
		border-radius: var(--border-radius-sm);
		border: 1px solid var(--border-color);
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 rgba(255, 255, 255, 0.03);
		animation: slideDown 0.5s cubic-bezier(0.16, 1, 0.3, 1);
	}

	.ms-logo {
		width: 48px;
		height: 48px;
		display: block;
	}

	@keyframes slideDown {
		from {
			opacity: 0;
			transform: translateY(-20px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
</style>