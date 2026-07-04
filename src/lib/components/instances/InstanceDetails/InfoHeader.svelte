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

<div class="details-header">
	<div class="info-badges">
		<span class="badge badge-version">{instance.version}</span>
		<span
			class="badge badge-loader"
			style="background: {loaderColor}20; color: {loaderColor}; border-color: {loaderColor}40;"
		>
			{instance.loader}
		</span>
		<span class="badge badge-status {statusClass}">{statusLabel}</span>
	</div>
	<div class="path-row">
		<span class="path-text" title={instance.path}>{instance.path}</span>
		<button
			type="button"
			class="icon-btn"
			onclick={() => onOpenDir()}
			title={t("instanceView.details.location")}
		>
			<svg
				width="14"
				height="14"
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
		</button>
	</div>
</div>

<style>
	.details-header {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		justify-content: space-between;
		gap: 12px;
		padding: 16px;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: 10px;
	}

	.info-badges {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		align-items: center;
	}

	.badge {
		font-size: 0.7rem;
		font-weight: 700;
		padding: 3px 10px;
		border-radius: 20px;
		letter-spacing: 0.5px;
		text-transform: uppercase;
		white-space: nowrap;
	}

	.badge-version {
		background: rgba(255, 255, 255, 0.06);
		color: var(--text-primary);
		border: 1px solid var(--border);
	}

	.badge-loader {
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid var(--border);
	}

	.badge-status {
		font-size: 0.62rem;
	}

	.badge-status.status-idle {
		background: rgba(120, 144, 156, 0.15);
		color: #90a4ae;
		border: 1px solid rgba(120, 144, 156, 0.3);
	}

	.badge-status.status-starting {
		background: rgba(33, 150, 243, 0.15);
		color: #64b5f6;
		border: 1px solid rgba(33, 150, 243, 0.3);
	}

	.badge-status.status-started {
		background: rgba(76, 175, 80, 0.15);
		color: #81c784;
		border: 1px solid rgba(76, 175, 80, 0.3);
	}

	.badge-status.status-error {
		background: rgba(244, 67, 54, 0.15);
		color: #e57373;
		border: 1px solid rgba(244, 67, 54, 0.3);
	}

	.path-row {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		max-width: 100%;
	}

	.path-text {
		font-size: 0.68rem;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 280px;
		opacity: 0.7;
	}

	.icon-btn {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		width: 26px;
		height: 26px;
		border-radius: 5px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		transition: all 0.15s ease;
	}

	.icon-btn:hover {
		background: rgba(255, 255, 255, 0.06);
		color: var(--text-primary);
		border-color: rgba(255, 255, 255, 0.2);
	}

	@media (max-width: 550px) {
		.details-header {
			flex-direction: column;
		}

		.path-text {
			max-width: 180px;
		}
	}
</style>
