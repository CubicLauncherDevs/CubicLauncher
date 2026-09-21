<script lang="ts">
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import { updater } from "$lib/api/updaterServices";
	import { updaterState } from "$lib/state/updaterState.svelte";
	import { launcherStore } from "$lib/state/state.svelte";
	import { saveSettings } from "$lib/api/launcherService";
	import { relaunch } from "@tauri-apps/plugin-process";

	const status = $derived(t(`updater.status.${updaterState.status}`));

	async function handleChannelChange() {
		await saveSettings();
		await relaunch();
	}
</script>

<div class="update-card">
	<div class="channel-row">
		<span class="channel-label">{t("updater.channel.label")}</span>
		<div class="channel-options">
			<label class="channel-option" class:active={launcherStore.settings.update_channel === "stable"}>
				<input
					type="radio"
					name="update-channel"
					value="stable"
					bind:group={launcherStore.settings.update_channel}
					onchange={handleChannelChange}
				/>
				<span>{t("updater.channel.stable")}</span>
			</label>
			<label class="channel-option" class:active={launcherStore.settings.update_channel === "prerelease"}>
				<input
					type="radio"
					name="update-channel"
					value="prerelease"
					bind:group={launcherStore.settings.update_channel}
					onchange={handleChannelChange}
				/>
				<span>{t("updater.channel.prerelease")}</span>
			</label>
		</div>
	</div>
	<div class="version-row">
		<div class="version-info">
			<span>{t("settings.launcher.currentVersion")}</span>
			<strong>v{__APP_VERSION__}</strong>
		</div>
		{#if updaterState.update}
			<div class="version-info">
				<span>{t("settings.launcher.available")}</span>
				<strong class="available">v{updaterState.update.version}</strong
				>
			</div>
		{/if}
	</div>
	<div class="update-status" role="status">
		<span
			>{status}{#if updaterState.status === "downloading" && updaterState.progress !== null}
				· {updaterState.progress}%{/if}</span
		>
	</div>
	<button type="button" onclick={updater.open}>
		<Icon
			name={updaterState.update ? "ui:download" : "ui:refresh"}
			size={16}
		/>
		{updaterState.update ||
		updaterState.status === "checking" ||
		updaterState.status === "error"
			? t("updater.view")
			: t("updater.check")}
	</button>
</div>

<style>
	.update-card {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 14px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-card);
	}
	.channel-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
	}
	.channel-label {
		font-size: 0.78rem;
		color: var(--text-secondary);
	}
	.channel-options {
		display: flex;
		gap: 2px;
		background: var(--bg-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 2px;
	}
	.channel-option {
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 4px 10px;
		border-radius: calc(var(--border-radius-sm) - 1px);
		font-size: 0.72rem;
		font-weight: 600;
		color: var(--text-tertiary);
		cursor: pointer;
		transition: all 0.15s ease;
	}
	.channel-option input {
		display: none;
	}
	.channel-option.active {
		background: var(--accent);
		color: var(--accent-text);
	}
	.channel-option:not(.active):hover {
		color: var(--text-primary);
	}
	.version-row {
		display: flex;
		flex-wrap: wrap;
		gap: 20px;
	}
	.version-info {
		display: flex;
		flex-direction: column;
		gap: 5px;
		min-width: 0;
	}
	.version-info span {
		font-size: 0.72rem;
		color: var(--text-secondary);
	}
	.version-info strong {
		font-size: 0.85rem;
		color: var(--text-primary);
		overflow-wrap: anywhere;
	}
	.version-info .available {
		color: var(--accent);
	}
	.update-status {
		font-size: 0.75rem;
		color: var(--text-secondary);
	}
	button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		align-self: flex-start;
		padding: 8px 12px;
		background: var(--surface-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
		font: inherit;
		font-size: 0.78rem;
		cursor: pointer;
	}
	button:hover {
		background: var(--surface-hover);
		border-color: var(--accent);
	}
	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 3px;
	}
</style>
