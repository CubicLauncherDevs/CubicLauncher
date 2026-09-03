<script lang="ts">
	import { t } from "$lib/i18n";
	import type { InstanceDto } from "$lib/types/types";
	import Icon from "$lib/icons/Icon.svelte";

	let {
		instance,
		loaderColor,
		statusLabel,
		statusClass,
		onOpenDir,
	}: {
		instance: InstanceDto;
		loaderColor: string;
		statusLabel: string;
		statusClass: string;
		onOpenDir: (subDir?: string) => void;
	} = $props();
</script>

<div class="info-header">
	<div class="badges">
		<span class="badge">{instance.version}</span>
		<span class="badge" style="color: {loaderColor}">{instance.loader}</span
		>
		<span class="badge {statusClass}">{statusLabel}</span>
	</div>
	<div class="path-row">
		<Icon name="instance:folder" size={12} />
		<span class="path-text" title={instance.path}>{instance.path}</span>
		<button
			type="button"
			class="icon-btn"
			onclick={() => onOpenDir()}
			title={t("instanceView.details.location")}
		>
			<Icon name="instance:external-link" size={13} />
		</button>
	</div>
</div>

<style>
	.info-header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px;
	}

	.badges {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		align-items: center;
	}

	.badge {
		font-size: 0.7rem;
		font-weight: 600;
		padding: 3px 10px;
		border-radius: 6px;
		background: var(--bg-card);
		color: var(--text-secondary);
		border: 1px solid var(--border);
	}

	.badge.status-idle {
		color: var(--text-tertiary);
	}

	.badge.status-starting {
		color: var(--color-status-starting);
	}

	.badge.status-started {
		color: var(--color-status-started);
	}

	.badge.status-error {
		color: var(--color-error);
	}

	.path-row {
		display: flex;
		align-items: center;
		gap: 5px;
		min-width: 0;
		color: var(--text-tertiary);
		font-size: 0.65rem;
	}

	.path-text {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 300px;
	}

	.icon-btn {
		background: transparent;
		border: none;
		color: var(--text-tertiary);
		cursor: pointer;
		padding: 2px;
		display: flex;
		align-items: center;
		transition: color 0.15s;
		flex-shrink: 0;
	}

	.icon-btn:hover {
		color: var(--text-primary);
	}

	@media (max-width: 550px) {
		.path-text {
			max-width: 160px;
		}
	}
</style>
