<script lang="ts">
	import { t } from "$lib/i18n";
	import { startWebviewAuth } from "$lib/api/cubicApi";
	import { saveSettings } from "$lib/api/launcherService";
	import { launcherStore } from "$lib/state/state.svelte";
	import ModalBase from "./ModalBase.svelte";

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
			<div class="state-container">
				<div class="minimal-spinner"></div>
				<h3 class="state-title">
					{t("userMenu.authModal.loading") || "Cargando..."}
				</h3>
				<p class="state-subtitle">
					{t("userMenu.authModal.waiting") ||
						"Inicia sesión en la ventana que se abrirá..."}
				</p>
			</div>
		{:else if error}
			<div class="state-container">
				<div class="icon-wrapper error">
					<svg
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						><circle cx="12" cy="12" r="10"></circle><line
							x1="15"
							y1="9"
							x2="9"
							y2="15"
						></line><line x1="9" y1="9" x2="15" y2="15"></line></svg
					>
				</div>
				<h3 class="state-title">Error de autenticación</h3>
				<p class="state-subtitle error-text">
					{t("userMenu.authModal.error")?.replace("{error}", error) ||
						error}
				</p>
				<button
					type="button"
					class="action-btn retry"
					onclick={startAuth}
				>
					<span>Reintentar</span>
				</button>
			</div>
		{:else if success}
			<div class="state-container">
				<div class="icon-wrapper success">
					<svg
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"
						></path><polyline points="22 4 12 14.01 9 11.01"
						></polyline></svg
					>
				</div>
				<h3 class="state-title">¡Conectado!</h3>
				<p class="state-subtitle">
					{t("userMenu.authModal.success") ||
						"Tu cuenta ha sido vinculada."}
				</p>
			</div>
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

	.state-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		width: 100%;
		animation: fadeIn 0.4s ease;
	}

	.state-title {
		font-size: 1.2rem;
		font-weight: 700;
		margin: 0 0 0.5rem 0;
		color: var(--text-primary);
	}

	.state-subtitle {
		font-size: 0.85rem;
		color: var(--text-secondary);
		margin: 0;
		max-width: 80%;
		line-height: 1.5;
	}

	.minimal-spinner {
		width: 32px;
		height: 32px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
		will-change: transform;
		margin-bottom: 1.5rem;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.icon-wrapper {
		width: 64px;
		height: 64px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-bottom: 1.5rem;
		position: relative;
	}

	.icon-wrapper::after {
		content: "";
		position: absolute;
		inset: -4px;
		border-radius: 50%;
		opacity: 0;
		z-index: -1;
		transition: opacity 0.15s;
	}

	.icon-wrapper svg {
		width: 32px;
		height: 32px;
	}

	.icon-wrapper.success {
		background: rgba(var(--color-success-rgb), 0.1);
		color: var(--color-success);
		animation: flashPop 0.7s cubic-bezier(0.16, 1, 0.3, 1) forwards;
	}

	.icon-wrapper.success::after {
		background: radial-gradient(
			circle,
			rgba(var(--color-success-rgb), 0.6) 0%,
			transparent 70%
		);
		animation: flashGlow 0.7s cubic-bezier(0.16, 1, 0.3, 1) forwards;
	}

	.icon-wrapper.error {
		background: rgba(var(--color-error-rgb), 0.1);
		color: var(--color-error);
		animation: flashPop 0.7s cubic-bezier(0.16, 1, 0.3, 1) forwards;
	}

	.icon-wrapper.error::after {
		background: radial-gradient(
			circle,
			rgba(var(--color-error-rgb), 0.6) 0%,
			transparent 70%
		);
		animation: flashGlow 0.7s cubic-bezier(0.16, 1, 0.3, 1) forwards;
	}

	.action-btn.retry {
		margin-top: 1.5rem;
		background: var(--bg-input);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		padding: 0.65rem 2rem;
		border-radius: var(--border-radius-sm);
		font-weight: 600;
		font-size: 0.8rem;
		font-family: inherit;
		cursor: pointer;
		transition: all 0.15s;
	}

	.action-btn.retry:hover {
		background: rgba(255, 255, 255, 0.04);
		color: var(--text-primary);
		border-color: var(--text-muted);
	}

	@keyframes flashPop {
		0% {
			opacity: 0;
			transform: scale(0);
			filter: brightness(4) saturate(2);
		}
		15% {
			opacity: 1;
			transform: scale(1.15);
			filter: brightness(4) saturate(2);
		}
		35% {
			transform: scale(0.95);
			filter: brightness(1.5) saturate(1.3);
		}
		55% {
			transform: scale(1.03);
			filter: brightness(1.1) saturate(1.05);
		}
		100% {
			opacity: 1;
			transform: scale(1);
			filter: brightness(1) saturate(1);
		}
	}

	@keyframes flashGlow {
		0% {
			opacity: 0;
			transform: scale(0.3);
		}
		15% {
			opacity: 0.9;
			transform: scale(1.3);
		}
		35% {
			opacity: 0.4;
			transform: scale(1);
		}
		100% {
			opacity: 0.3;
			transform: scale(1);
		}
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
</style>
