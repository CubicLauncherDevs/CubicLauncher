<script lang="ts">
	import { t } from "$lib/i18n";
	import { INSTANCE_LOGOS } from "$lib/icons/logos";
	import Select from "$lib/components/layout/Select.svelte";

	let {
		selectedIcon = $bindable<string | null>(null),
		instanceName = $bindable(""),
		instGameVersion = $bindable(""),
		versionOptions = [] as { value: string; label: string }[],
		saving = false,
		onVersionChange,
		onReinstall,
	}: {
		selectedIcon: string | null;
		instanceName: string;
		instGameVersion: string;
		versionOptions: { value: string; label: string }[];
		saving: boolean;
		onVersionChange: (version: string) => void;
		onReinstall: () => void;
	} = $props();
</script>

<div style="margin-bottom: 4px;">
	{t("createInstance.iconLabel")}
</div>
<div id="icon-selector" class="icon-selector">
	{#each INSTANCE_LOGOS as iconName (iconName)}
		{@const iconPath = `/images/instances/${iconName}`}
		<button
			type="button"
			class="icon-option"
			class:selected={selectedIcon === iconPath}
			onclick={() =>
				(selectedIcon =
					selectedIcon === iconPath ? null : iconPath)}
			title={iconName}
		>
			<img src={iconPath} alt={iconName} />
		</button>
	{/each}
</div>
<div class="name-section">
	<span>{t("createInstance.nameLabel")}</span>
	<input
		placeholder={t("createInstance.namePlaceholder")}
		id="name-input"
		type="text"
		class="text-input"
		bind:value={instanceName}
	/>
</div>
<div class="version-section">
	<Select
		value={instGameVersion}
		options={versionOptions}
		label={t("createInstance.versionLabel")}
		onchange={onVersionChange}
	/>
</div>
<div class="reinstall">
	<button
		type="button"
		class="qm-save-btn"
		onclick={onReinstall}
		disabled={saving}
	>
		{t("instanceEditor.reinstall")}
	</button>
</div>

<style>
	.name-section {
		margin-top: 1vb;
	}

	.version-section {
		margin-top: 1vb;
	}

	.icon-selector {
		display: flex;
		gap: 8px;
		margin-top: 4px;
	}

	.icon-option {
		width: 42px;
		height: 42px;
		border-radius: 8px;
		background: rgba(255, 255, 255, 0.04);
		border: 2px solid var(--border-color);
		cursor: pointer;
		padding: 6px;
		transition: all 0.15s;
	}

	.icon-option:hover {
		background: rgba(255, 255, 255, 0.08);
		border-color: var(--text-secondary);
	}

	.icon-option.selected {
		border-color: var(--accent);
		background: rgba(255, 255, 255, 0.1);
	}

	.icon-option img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}

	.text-input {
		width: 100%;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 8px 12px;
		border-radius: var(--border-radius-sm);
		font-size: 0.9rem;
		box-sizing: border-box;
	}

	.text-input:focus {
		outline: none;
		border-color: var(--text-muted);
	}

	.qm-save-btn {
		width: 100%;
		background: var(--bg-card);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		padding: 10px 12px;
		border-radius: var(--border-radius-sm);
		font-family: var(--font-family);
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.15s,
			border-color 0.15s;
		box-shadow: var(--shadow-sm);
	}

	.qm-save-btn:hover:not(:disabled) {
		background: var(--bg-item-active);
		border-color: var(--border-color);
	}

	.qm-save-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.reinstall {
		padding: 2ch 0px 0px 0px;
		border-top: 1px solid var(--border-color);
	}
</style>
