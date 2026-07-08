<script lang="ts">
	import { t } from "$lib/i18n";
	import type { InstanceDto } from "$lib/types/types";

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
		<svg
			width="12"
			height="12"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<path
				d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
			/>
		</svg>
		<span class="path-text" title={instance.path}>{instance.path}</span>
		<button
			type="button"
			class="icon-btn"
			onclick={() => onOpenDir()}
			title={t("instanceView.details.location")}
		>
			<svg
				width="13"
				height="13"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path
					d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"
				/><polyline points="15 3 21 3 21 9" /><line
					x1="10"
					y1="14"
					x2="21"
					y2="3"
				/>
			</svg>
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
		color: #64b5f6;
	}

	.badge.status-started {
		color: #81c784;
	}

	.badge.status-error {
		color: #e57373;
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
