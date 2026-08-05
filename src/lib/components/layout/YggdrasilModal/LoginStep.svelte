<script lang="ts">
	import { t } from "$lib/i18n";
	import type { YggdrasilServerInfo } from "$lib/types/types";

	let {
		serverInfo,
		username = $bindable(""),
		password = $bindable(""),
		onback,
		onlogin,
	}: {
		serverInfo: YggdrasilServerInfo | null;
		username?: string;
		password?: string;
		onback: () => void;
		onlogin: () => void;
	} = $props();

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === "Enter") onlogin();
	}
</script>

<div class="form-step">
	{#if serverInfo}
		<div class="server-badge">
			<span class="server-name">{serverInfo.server_name}</span>
		</div>
	{/if}
	<p class="instruction-text">
		{t("userMenu.yggdrasilModal.loginInstruction")}
	</p>
	<div class="form-group">
		<label class="form-label" for="ygg-username">
			{serverInfo?.non_email_login
				? t("userMenu.yggdrasilModal.usernameLabel")
				: t("userMenu.yggdrasilModal.emailLabel")}
		</label>
		<input
			id="ygg-username"
			type={serverInfo?.non_email_login ? "text" : "email"}
			class="form-input"
			placeholder={serverInfo?.non_email_login
				? t("userMenu.yggdrasilModal.usernamePlaceholder")
				: t("userMenu.yggdrasilModal.emailPlaceholder")}
			bind:value={username}
			onkeydown={handleKeydown}
		/>
	</div>
	<div class="form-group">
		<label class="form-label" for="ygg-password"
			>{t("userMenu.yggdrasilModal.passwordLabel")}</label
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
		<button type="button" class="action-btn secondary" onclick={onback}>
			{t("userMenu.yggdrasilModal.back")}
		</button>
		<button
			type="button"
			class="action-btn primary"
			onclick={onlogin}
			disabled={!username.trim() || !password}
		>
			{t("userMenu.yggdrasilModal.login")}
		</button>
	</div>
</div>

<style>
	.form-step {
		display: flex;
		flex-direction: column;
		align-items: center;
		width: 100%;
		animation: fadeIn 0.4s ease;
	}

	.instruction-text {
		font-size: 0.85rem;
		color: var(--text-secondary);
		margin-bottom: 1.25rem;
		line-height: 1.6;
		padding: 0 0.5rem;
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
		background: rgba(var(--surface-rgb), 0.04);
		color: var(--text-primary);
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
