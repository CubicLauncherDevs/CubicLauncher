<script lang="ts">
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import { updater } from "$lib/api/updaterServices";
	import { updaterState } from "$lib/state/updaterState.svelte";

	const status = $derived(t(`updater.status.${updaterState.status}`));
</script>

<div class="update-card">
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
