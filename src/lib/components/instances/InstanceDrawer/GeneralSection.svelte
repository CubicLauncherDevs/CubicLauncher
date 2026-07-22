<script lang="ts">
	import { t } from "$lib/i18n";
	import { INSTANCE_LOGOS } from "$lib/icons/logos";
	import { uploadCustomIcon } from "$lib/api/cubicApi";
	import { launcherStore } from "$lib/state/state.svelte";
	import { open as openDialog } from "@tauri-apps/plugin-dialog";

	let {
		selectedIcon = $bindable<string | null>(null),
		instanceName = $bindable(""),
		instanceUuid = "",
	}: {
		selectedIcon: string | null;
		instanceName: string;
		instanceUuid?: string;
	} = $props();

	async function handleUpload() {
		try {
			const selected = await openDialog({
				multiple: false,
				filters: [
					{
						name: t("createInstance.iconFilter"),
						extensions: ["png", "jpg", "jpeg", "webp", "gif"],
					},
				],
			});
			if (selected && instanceUuid) {
				const path = await uploadCustomIcon(instanceUuid, selected);
				if (path) {
					selectedIcon = path;
					const idx = launcherStore.loadedInstances.findIndex(
						(i) => i.uuid === instanceUuid,
					);
					if (idx !== -1) {
						launcherStore.loadedInstances[idx] = {
							...launcherStore.loadedInstances[idx],
							icon: path,
						};
					}
				}
			}
		} catch (e) {
			console.error("Error selecting icon:", e);
		}
	}
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
	<button
		type="button"
		class="icon-option upload-option"
		onclick={handleUpload}
		title={t("createInstance.uploadIcon")}
	>
		<svg
			width="16"
			height="16"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
			<polyline points="17 8 12 3 7 8" />
			<line x1="12" y1="3" x2="12" y2="15" />
		</svg>
	</button>
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

	.upload-option {
		color: var(--text-tertiary);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.upload-option:hover {
		color: var(--accent);
		border-color: var(--accent);
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
