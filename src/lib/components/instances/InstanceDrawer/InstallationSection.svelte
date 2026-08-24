<script lang="ts">
	import { t } from "$lib/i18n";
	import VersionSelectorStep from "../CreateInstanceModal/VersionSelectorStep.svelte";

	let {
		selectedLoader = $bindable("vanilla"),
		selectedMcVersion = $bindable(""),
		selectedLoaderVersion = $bindable(""),
		onRepair,
		repairing = false,
	}: {
		selectedLoader: string;
		selectedMcVersion: string;
		selectedLoaderVersion: string;
		onRepair: () => void;
		repairing: boolean;
	} = $props();

	$effect(() => {
		if (selectedLoader == null || selectedLoader === "")
			selectedLoader = "vanilla";
		if (selectedMcVersion == null) selectedMcVersion = "";
		if (selectedLoaderVersion == null) selectedLoaderVersion = "";
	});

	let canRepair = $derived(
		selectedLoader === "vanilla"
			? !!selectedMcVersion
			: !!selectedMcVersion && !!selectedLoaderVersion,
	);
</script>

<div class="installation-section">
	<div class="version-warning">
		{t("instanceEditor.versionChangeWarning")}
	</div>

	<VersionSelectorStep
		bind:selectedLoader
		bind:selectedMcVersion
		bind:selectedLoaderVersion
		compact={true}
	/>

	<div class="repair-section">
		<button
			type="button"
			class="repair-btn"
			onclick={onRepair}
			disabled={repairing || !canRepair}
		>
			Repair
		</button>
		<span class="repair-hint">
			{t("instanceEditor.repairHint")}
		</span>
	</div>
</div>

<style>
	.installation-section {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.version-warning {
		font-size: 0.8rem;
		color: var(--warning, #f59e0b);
		line-height: 1.5;
		padding: 10px 12px;
		border: 1px solid var(--warning, #f59e0b);
		border-radius: var(--border-radius-sm);
		background: color-mix(
			in srgb,
			var(--warning, #f59e0b) 10%,
			transparent
		);
	}

	.repair-section {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.repair-btn {
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

	.repair-btn:hover:not(:disabled) {
		background: var(--bg-item-active);
		border-color: var(--border-color);
	}

	.repair-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.repair-hint {
		font-size: 0.75rem;
		color: var(--text-muted);
		line-height: 1.5;
	}
</style>
