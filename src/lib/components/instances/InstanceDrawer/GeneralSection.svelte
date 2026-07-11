<script lang="ts">
	import { t } from "$lib/i18n";
	import { INSTANCE_LOGOS } from "$lib/icons/logos";

	let {
		selectedIcon = $bindable<string | null>(null),
		instanceName = $bindable(""),
	}: {
		selectedIcon: string | null;
		instanceName: string;
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
				(selectedIcon = selectedIcon === iconPath ? null : iconPath)}
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

<style>
	.name-section {
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

</style>
