<script lang="ts">
	import { t } from "$lib/i18n";
	import {
		getDeviceCode,
		authenticateWithDeviceCode,
	} from "$lib/api/cubicApi";
	import { saveSettings } from "$lib/api/launcherService";
	import type { DeviceCode } from "$lib/types/types";
	import { launcherStore } from "$lib/state/state.svelte";
	import ModalBase from "./ModalBase.svelte";
	import CopyIcon from "$lib/icons/CopyIcon.svelte";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";

	let { open = $bindable(false) } = $props<{ open: boolean }>();

	let deviceCode = $state<DeviceCode | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let success = $state(false);
	let copiedCode = $state(false);
	let copiedLink = $state(false);

	async function startAuth() {
		try {
			loading = true;
			error = null;
			deviceCode = await getDeviceCode();
			loading = false;

			// Start polling
			const user = await authenticateWithDeviceCode(
				deviceCode.device_code,
				deviceCode.interval,
				deviceCode.expires_in,
			);

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
			// Reset state when opened
			deviceCode = null;
			loading = true;
			error = null;
			success = false;
			copiedCode = false;
			copiedLink = false;
			startAuth();
		}
	});

	async function handleCopyCode() {
		if (deviceCode) {
			try {
				await navigator.clipboard.writeText(deviceCode.user_code);
				copiedCode = true;
				setTimeout(() => {
					copiedCode = false;
				}, 2000);
			} catch (err) {
				console.error("Failed to copy code:", err);
			}
		}
	}

	async function handleCopyLink() {
		if (deviceCode) {
			try {
				await navigator.clipboard.writeText(
					deviceCode.verification_uri,
				);
				copiedLink = true;
				setTimeout(() => {
					copiedLink = false;
				}, 2000);
			} catch (err) {
				console.error("Failed to copy link:", err);
			}
		}
	}
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
				<p class="state-subtitle">Conectando con Microsoft...</p>
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
		{:else if deviceCode}
			<div class="state-container device-auth">
				<p class="instruction-text">
					{t("userMenu.authModal.instruction") ||
						"Introduce el siguiente código en la página de Microsoft para vincular tu cuenta."}
				</p>

				<div class="code-card">
					<div class="field-group">
						<span class="field-label">Enlace de verificación</span>
						<div class="copy-box">
							<div
								class="url-display"
								title={deviceCode.verification_uri}
							>
								{deviceCode.verification_uri}
							</div>
							<button
								type="button"
								class="icon-btn {copiedLink ? 'copied' : ''}"
								onclick={handleCopyLink}
								title={copiedLink
									? "¡Copiado!"
									: "Copiar enlace"}
							>
								{#if copiedLink}
									<CheckIcon size={16} />
								{:else}
									<CopyIcon size={16} />
								{/if}
							</button>
						</div>
					</div>

					<div class="field-group">
						<span class="field-label">Código</span>
						<div class="copy-box code-box">
							<div class="code-display">
								{#each deviceCode.user_code.split("") as char, i (i)}
									<span
										class="code-char {char === '-'
											? 'dash'
											: ''}">{char}</span
									>
								{/each}
							</div>
							<button
								type="button"
								class="icon-btn {copiedCode ? 'copied' : ''}"
								onclick={handleCopyCode}
								title={copiedCode
									? "¡Copiado!"
									: "Copiar código"}
							>
								{#if copiedCode}
									<CheckIcon size={16} />
								{:else}
									<CopyIcon size={16} />
								{/if}
							</button>
						</div>
					</div>
				</div>

				<div class="waiting-box">
					<div class="minimal-dot"></div>
					<div class="waiting-text">
						<span class="status-title">Esperando autorización</span>
						<span class="status-subtitle"
							>{t("userMenu.authModal.waiting") ||
								"Completa los pasos en tu navegador"}</span
						>
					</div>
				</div>
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

	.instruction-text {
		font-size: 0.85rem;
		color: var(--text-secondary);
		margin-bottom: 1.5rem;
		line-height: 1.6;
		padding: 0 0.5rem;
	}

	.code-card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		padding: 1.25rem;
		width: 100%;
		max-width: 340px;
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 rgba(255, 255, 255, 0.03);
	}

	.field-group {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		width: 100%;
		gap: 0.5rem;
	}

	.field-label {
		font-size: 0.7rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.copy-box {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		padding: 0.5rem;
		gap: 0.5rem;
	}

	.code-box {
		padding: 0.5rem 0.5rem 0.5rem 1rem;
	}

	.url-display {
		font-size: 0.8rem;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		padding-left: 0.5rem;
		text-align: left;
		width: 100%;
	}

	.code-display {
		display: flex;
		align-items: center;
		gap: 0.2rem;

		font-size: 1.1rem;
		font-weight: 800;
		letter-spacing: 1px;
	}

	.code-char {
		background: var(--bg-main);
		padding: 0.15rem 0.25rem;
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
		border: 1px solid var(--border);
	}

	.code-char.dash {
		background: transparent;
		border: none;
		color: var(--text-muted);
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
		flex-shrink: 0;
	}

	.icon-btn:hover {
		color: var(--text-primary);
		border-color: var(--border-color);
		background: rgba(255, 255, 255, 0.03);
	}

	.icon-btn.copied {
		color: var(--color-success);
		border-color: rgba(var(--color-success-rgb), 0.2);
		background: rgba(var(--color-success-rgb), 0.08);
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

	.waiting-box {
		margin-top: 1.5rem;
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem 1rem;
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		width: 100%;
		max-width: 340px;
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 rgba(255, 255, 255, 0.03);
	}

	.minimal-dot {
		width: 8px;
		height: 8px;
		background: var(--accent);
		border-radius: 50%;
		flex-shrink: 0;
		animation: simplePulse 1.5s ease-in-out infinite;
	}

	.waiting-text {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		text-align: left;
	}

	.status-title {
		font-size: 0.85rem;
		font-weight: 700;
		color: var(--text-primary);
	}

	.status-subtitle {
		font-size: 0.7rem;
		color: var(--text-secondary);
		margin-top: 0.1rem;
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

	@keyframes simplePulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.3;
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
		.ms-logo-wrapper {
			margin-bottom: 0.75rem;
			padding: 0.5rem;
		}
		.ms-logo {
			width: 32px;
			height: 32px;
		}
		.code-card {
			padding: 1rem;
			gap: 1rem;
		}
		.waiting-box {
			margin-top: 1rem;
			padding: 0.5rem 0.75rem;
		}
		.field-group {
			gap: 0.25rem;
		}
		.state-title {
			font-size: 1.1rem;
			margin-bottom: 0.25rem;
		}
		.instruction-text {
			margin-bottom: 0.75rem;
			font-size: 0.8rem;
		}
	}
</style>
