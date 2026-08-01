<script lang="ts">
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";

	let {
		onOpenDir,
		onOpenLogs,
	}: {
		onOpenDir: (subDir?: string) => void;
		onOpenLogs: () => void;
	} = $props();
</script>

<div class="action-bar">
	<button
		type="button"
		class="action-btn"
		onclick={() => onOpenDir()}
		title={t("instanceView.options.folder")}
	>
		<Icon src="/images/icons/instance/folder.svg" size={16} />
	</button>
	<button
		type="button"
		class="action-btn"
		onclick={() => onOpenDir("mods")}
		title={t("instanceView.tabs.mods")}
	>
		<Icon src="/images/icons/instance/grid.svg" size={16} />
	</button>
	<button
		type="button"
		class="action-btn"
		onclick={() => onOpenDir("screenshots")}
		title={t("instanceView.tabs.screenshots")}
	>
		<Icon src="/images/icons/instance/image.svg" size={16} />
	</button>
	<button
		type="button"
		class="action-btn"
		onclick={() => onOpenDir("resourcepacks")}
		title={t("instanceView.tabs.resources")}
	>
		<Icon src="/images/icons/instance/database.svg" size={16} />
	</button>
	<button
		type="button"
		class="action-btn"
		onclick={() => onOpenLogs()}
		title={t("instanceView.tabs.logs")}
	>
		<Icon src="/images/icons/instance/code.svg" size={16} />
	</button>
</div>

<style>
	.action-bar {
		display: flex;
		gap: 4px;
	}

	.action-btn {
		width: 32px;
		height: 32px;
		border-radius: 8px;
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-tertiary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s ease;
	}

	.action-btn:hover {
		background: var(--bg-card);
		color: var(--text-primary);
		border-color: var(--text-tertiary);
	}
</style>
