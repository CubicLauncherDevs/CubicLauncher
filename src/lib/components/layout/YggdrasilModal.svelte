<script lang="ts">
	import { t } from "$lib/i18n";
	import {
		getYggdrasilServerInfo,
		yggdrasilAuthenticate,
	} from "$lib/api/cubicApi";
	import { saveSettings } from "$lib/api/launcherService";
	import type { YggdrasilServerInfo } from "$lib/types/types";
	import { launcherStore } from "$lib/state/state.svelte";
	import ModalBase from "./ModalBase.svelte";

	let { open = $bindable(false) } = $props<{ open: boolean }>();

	type Step = "server" | "login" | "loading" | "success" | "error";

	let step = $state<Step>("server");
	let serverUrl = $state("");
	let username = $state("");
	let password = $state("");
	let serverInfo = $state<YggdrasilServerInfo | null>(null);
	let error = $state<string | null>(null);

	$effect(() => {
		if (open) {
			step = "server";
			serverUrl = "";
			username = "";
			password = "";
			serverInfo = null;
			error = null;
		}
	});

	async function handleConnectServer() {
		if (!serverUrl.trim()) return;
		step = "loading";
		error = null;
		try {
			serverInfo = await getYggdrasilServerInfo(serverUrl.trim());
			step = "login";
		} catch (e: unknown) {
			error = String(e);
			step = "error";
		}
	}

	async function handleLogin() {
		if (!username.trim() || !password) return;
		step = "loading";
		error = null;
		try {
			const user = await yggdrasilAuthenticate(
				serverUrl.trim(),
				username.trim(),
				password,
			);

			const idx = launcherStore.settings.user.findIndex(
				(u) =>
					u.username === user.username &&
					u.yggdrasil_server_url === user.yggdrasil_server_url,
			);
			if (idx >= 0) {
				launcherStore.settings.user[idx] = user;
				launcherStore.settings.active_user_idx = idx;
			} else {
				launcherStore.settings.user = [
					...launcherStore.settings.user,
					user,
				];
				launcherStore.settings.active_user_idx =
					launcherStore.settings.user.length - 1;
			}
			await saveSettings();
			step = "success";
			setTimeout(() => {
				open = false;
			}, 2000);
		} catch (e: unknown) {
			error = String(e);
			step = "error";
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === "Enter") {
			if (step === "server") handleConnectServer();
			else if (step === "login") handleLogin();
		}
	}
</script>

<ModalBase bind:open title="Authlib Injector">
	<div class="ygg-container">
		<div class="ygg-logo-wrapper">
			<svg
				class="ygg-logo"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="1.5"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M12 2L2 7l10 5 10-5-10-5z" />
				<path d="M2 17l10 5 10-5" />
				<path d="M2 12l10 5 10-5" />
			</svg>
		</div>

		{#if step === "loading"}
			<div class="state-container">
				<div class="minimal-spinner"></div>
				<h3 class="state-title">Conectando...</h3>
				{#if serverInfo}
					<p class="state-subtitle">{serverInfo.server_name}</p>
				{/if}
			</div>
		{:else if step === "error"}
			<div class="state-container">
				<div class="icon-wrapper error">
					<svg
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
					>
						<circle cx="12" cy="12" r="10"></circle>
						<line x1="15" y1="9" x2="9" y2="15"></line>
						<line x1="9" y1="9" x2="15" y2="15"></line>
					</svg>
				</div>
				<h3 class="state-title">Error</h3>
				<p class="state-subtitle error-text">{error}</p>
				<button
					type="button"
					class="action-btn retry"
					onclick={() => {
						step = "server";
						error = null;
					}}
				>
					Reintentar
				</button>
			</div>
		{:else if step === "success"}
			<div class="state-container">
				<div class="icon-wrapper success">
					<svg
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
					>
						<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
						<polyline points="22 4 12 14.01 9 11.01"></polyline>
					</svg>
				</div>
				<h3 class="state-title">¡Conectado!</h3>
				<p class="state-subtitle">Tu cuenta ha sido vinculada.</p>
			</div>
		{:else if step === "server"}
			<div class="state-container form-state">
				<p class="instruction-text">
					Ingresa la URL del servidor de autenticación Yggdrasil.
				</p>
				<div class="form-group">
					<label class="form-label" for="ygg-server-url"
						>Servidor</label
					>
					<input
						id="ygg-server-url"
						type="text"
						class="form-input"
						placeholder="ej: littlesk.in"
						bind:value={serverUrl}
						onkeydown={handleKeydown}
					/>
				</div>
				<button
					type="button"
					class="action-btn primary"
					onclick={handleConnectServer}
					disabled={!serverUrl.trim()}
				>
					Conectar
				</button>
			</div>
		{:else if step === "login"}
			<div class="state-container form-state">
				{#if serverInfo}
					<div class="server-badge">
						<span class="server-name">{serverInfo.server_name}</span
						>
					</div>
				{/if}
				<p class="instruction-text">
					Ingresa tus credenciales para iniciar sesión.
				</p>
				<div class="form-group">
					<label class="form-label" for="ygg-username">
						{serverInfo?.non_email_login
							? "Usuario"
							: "Correo electrónico"}
					</label>
					<input
						id="ygg-username"
						type={serverInfo?.non_email_login ? "text" : "email"}
						class="form-input"
						placeholder={serverInfo?.non_email_login
							? "Tu usuario"
							: "correo@ejemplo.com"}
						bind:value={username}
						onkeydown={handleKeydown}
					/>
				</div>
				<div class="form-group">
					<label class="form-label" for="ygg-password"
						>Contraseña</label
					>
					<input
						id="ygg-password"
						type="password"
						class="form-input"
						placeholder="••••••••"
						bind:value={password}
						onkeydown={handleKeydown}
					/>
				</div>
				<div class="form-actions">
					<button
						type="button"
						class="action-btn secondary"
						onclick={() => {
							step = "server";
							serverInfo = null;
						}}
					>
						Atrás
					</button>
					<button
						type="button"
						class="action-btn primary"
						onclick={handleLogin}
						disabled={!username.trim() || !password}
					>
						Iniciar sesión
					</button>
				</div>
			</div>
		{/if}
	</div>
</ModalBase>

<style>
	.ygg-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		padding: 0;
		width: 100%;
		color: var(--text-primary);
	}

	.ygg-logo-wrapper {
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

	.ygg-logo {
		width: 48px;
		height: 48px;
		display: block;
		color: var(--accent);
	}

	.state-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		width: 100%;
		animation: fadeIn 0.4s ease;
	}

	.form-state {
		gap: 0;
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

	.instruction-text {
		font-size: 0.85rem;
		color: var(--text-secondary);
		margin-bottom: 1.25rem;
		line-height: 1.6;
		padding: 0 0.5rem;
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
		opacity: 0.3;
		z-index: -1;
	}

	.icon-wrapper svg {
		width: 32px;
		height: 32px;
	}

	.icon-wrapper.success {
		background: rgba(var(--color-success-rgb), 0.1);
		color: var(--color-success);
	}

	.icon-wrapper.success::after {
		background: radial-gradient(
			circle,
			rgba(var(--color-success-rgb), 0.5) 0%,
			transparent 70%
		);
		animation: pop 0.5s cubic-bezier(0.175, 0.885, 0.32, 1.275);
	}

	.icon-wrapper.error {
		background: rgba(var(--color-error-rgb), 0.1);
		color: var(--color-error);
	}

	.icon-wrapper.error::after {
		background: radial-gradient(
			circle,
			rgba(var(--color-error-rgb), 0.5) 0%,
			transparent 70%
		);
	}

	.error-text {
		color: var(--color-error);
		max-width: 90%;
		word-break: break-word;
	}

	.server-badge {
		margin-bottom: 1rem;
		padding: 0.5rem 1rem;
		background: var(--accent);
		color: var(--accent-text);
		border-radius: var(--border-radius-sm);
		font-size: 0.8rem;
		font-weight: 700;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		width: 100%;
		max-width: 340px;
		gap: 0.4rem;
		margin-bottom: 1rem;
	}

	.form-label {
		font-size: 0.7rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.form-input {
		width: 100%;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 0.55rem 0.75rem;
		border-radius: var(--border-radius-sm);
		font-size: 0.85rem;
		font-family: inherit;
		box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.2);
		box-sizing: border-box;
		transition: border-color 0.15s;
	}

	.form-input:focus {
		outline: none;
		border-color: var(--accent);
	}

	.form-input::placeholder {
		color: var(--text-muted);
	}

	.form-actions {
		display: flex;
		gap: 8px;
		width: 100%;
		max-width: 340px;
	}

	.action-btn {
		padding: 0.6rem 1.5rem;
		border-radius: var(--border-radius-sm);
		font-size: 0.8rem;
		font-weight: 600;
		font-family: inherit;
		cursor: pointer;
		transition: all 0.15s;
		border: none;
		flex: 1;
	}

	.action-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.action-btn.primary {
		background: var(--accent);
		color: var(--accent-text);
	}

	.action-btn.primary:hover:not(:disabled) {
		opacity: 0.85;
	}

	.action-btn.secondary {
		background: var(--bg-input);
		color: var(--text-secondary);
		border: 1px solid var(--border-color);
	}

	.action-btn.secondary:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.04);
		color: var(--text-primary);
	}

	.action-btn.retry {
		margin-top: 1.5rem;
		background: var(--bg-input);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		flex: none;
	}

	.action-btn.retry:hover {
		background: rgba(255, 255, 255, 0.04);
		border-color: var(--text-muted);
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

	@keyframes pop {
		0% {
			transform: scale(0.8);
			opacity: 0;
		}
		100% {
			transform: scale(1);
			opacity: 1;
		}
	}

	@media (max-height: 700px) {
		.ygg-logo-wrapper {
			margin-bottom: 0.75rem;
			padding: 0.5rem;
		}
		.ygg-logo {
			width: 32px;
			height: 32px;
		}
		.instruction-text {
			margin-bottom: 0.75rem;
			font-size: 0.8rem;
		}
	}
</style>
