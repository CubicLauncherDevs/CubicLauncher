<script lang="ts">
	import { onDestroy, tick } from "svelte";
	import { t } from "$lib/i18n";
	import {
		startWebviewAuth,
		getDeviceCode,
		authenticateWithDeviceCode,
	} from "$lib/api/cubicApi";
	import type { DeviceCode, MinecraftUser } from "$lib/types/types";
	import { saveSettings } from "$lib/api/launcherService";
	import { launcherStore } from "$lib/state/state.svelte";
	import ModalBase from "../ModalBase.svelte";
	import AuthLoading from "./AuthLoading.svelte";
	import AuthError from "./AuthError.svelte";
	import AuthSuccess from "./AuthSuccess.svelte";

	let {
		open = $bindable(false),
		onsuccess,
		reauthUsername,
	} = $props<{
		open: boolean;
		onsuccess?: () => void;
		reauthUsername?: string;
	}>();

	type AuthMethod = "launcher" | "code";
	let method = $state<AuthMethod | null>(null);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let success = $state(false);
	let deviceCode = $state<DeviceCode | null>(null);
	let closeTimer: ReturnType<typeof setTimeout> | undefined;
	let attempt = 0;

	onDestroy(() => {
		clearTimeout(closeTimer);
		attempt++;
	});

	async function startAuth(selectedMethod: AuthMethod) {
		const currentAttempt = ++attempt;
		method = selectedMethod;
		try {
			loading = true;
			error = null;
			deviceCode = null;

			let user: MinecraftUser;
			if (selectedMethod === "code") {
				const code = await getDeviceCode();
				if (currentAttempt !== attempt || !open) return;
				deviceCode = code;
				await tick();
				user = await authenticateWithDeviceCode(
					code.device_code,
					code.interval,
					code.expires_in,
				);
			} else {
				user = await startWebviewAuth();
			}
			if (currentAttempt !== attempt || !open) return;

			const idx = launcherStore.settings.user.findIndex(
				(u) => u.uuid === user.uuid,
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
			loading = false;
			success = true;
			onsuccess?.();

			closeTimer = setTimeout(() => {
				open = false;
			}, 2000);
		} catch (e: unknown) {
			if (currentAttempt !== attempt || !open) return;
			console.error("Auth error:", e);
			error = String(e);
			loading = false;
		}
	}

	$effect(() => {
		if (open) {
			method = null;
			loading = false;
			error = null;
			success = false;
			deviceCode = null;
		} else {
			attempt++;
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

		{#if reauthUsername}
			<p class="reauth-message">
				{t("userMenu.authModal.sessionExpired", {
					username: reauthUsername,
				})}
			</p>
		{/if}

		{#if loading}
			<AuthLoading
				title={t("userMenu.authModal.loading")}
				subtitle={deviceCode
					? t("userMenu.authModal.instruction")
					: t("userMenu.authModal.waiting")}
			/>
			{#if deviceCode}
				<div class="device-code" role="status">
					<span>{deviceCode.verification_uri}</span>
					<code>{deviceCode.user_code}</code>
				</div>
			{/if}
		{:else if error}
			<AuthError
				title="Error de autenticación"
				message={t("userMenu.authModal.error", { error }) || error}
				onRetry={() => method && startAuth(method)}
			/>
			<button
				type="button"
				class="method-button"
				onclick={() => {
					method = null;
					error = null;
				}}
			>
				{t("userMenu.authModal.otherMethod")}
			</button>
		{:else if success}
			<AuthSuccess
				subtitle={t("userMenu.authModal.success") ||
					"Tu cuenta ha sido vinculada."}
			/>
		{:else}
			<div class="method-choices">
				<button
					type="button"
					class="method-button"
					onclick={() => startAuth("launcher")}
				>
					{t("userMenu.authModal.fromLauncher")}
				</button>
				<button
					type="button"
					class="method-button"
					onclick={() => startAuth("code")}
				>
					{t("userMenu.authModal.withCode")}
				</button>
			</div>
		{/if}
	</div>
</ModalBase>

<style>
	.reauth-message {
		margin: 0 0 1rem;
		color: var(--text-secondary);
	}

	.method-choices {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		width: 100%;
	}

	.method-button {
		padding: 0.7rem 1rem;
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		color: var(--text-primary);
		font: inherit;
		cursor: pointer;
	}

	.method-button:hover {
		border-color: var(--accent);
	}

	.device-code {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-top: 1rem;
		font-size: 0.9rem;
	}

	.device-code code {
		font-size: 1.4rem;
		font-weight: 700;
		letter-spacing: 0.12em;
		user-select: all;
	}
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
			inset 0 1px 0 var(--surface-selected);
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
