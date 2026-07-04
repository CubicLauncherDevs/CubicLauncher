<script lang="ts">
	import {
		getYggdrasilServerInfo,
		yggdrasilAuthenticate,
	} from "$lib/api/cubicApi";
	import { saveSettings } from "$lib/api/launcherService";
	import type { YggdrasilServerInfo } from "$lib/types/types";
	import { launcherStore } from "$lib/state/state.svelte";
	import ModalBase from "../ModalBase.svelte";
	import AuthLoading from "../auth/AuthLoading.svelte";
	import AuthError from "../auth/AuthError.svelte";
	import AuthSuccess from "../auth/AuthSuccess.svelte";
	import ServerStep from "./ServerStep.svelte";
	import LoginStep from "./LoginStep.svelte";

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
				launcherStore.settings.user.push(user);
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
			<AuthLoading subtitle={serverInfo?.server_name} />
		{:else if step === "error"}
			<AuthError
				title="Error"
				message={error ?? "Error desconocido"}
				onRetry={() => {
					step = "server";
					error = null;
				}}
			/>
		{:else if step === "success"}
			<AuthSuccess />
		{:else if step === "server"}
			<ServerStep bind:serverUrl onconnect={handleConnectServer} />
		{:else if step === "login"}
			<LoginStep
				{serverInfo}
				bind:username
				bind:password
				onback={() => {
					step = "server";
					serverInfo = null;
				}}
				onlogin={handleLogin}
			/>
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

	@media (max-height: 700px) {
		.ygg-logo-wrapper {
			margin-bottom: 0.75rem;
			padding: 0.5rem;
		}
		.ygg-logo {
			width: 32px;
			height: 32px;
		}
	}
</style>