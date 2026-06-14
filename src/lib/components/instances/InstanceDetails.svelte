<script lang="ts">
	import type { InstanceDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import { invoke } from "@tauri-apps/api/core";
	let { instance } = $props<{ instance: InstanceDto }>();

	const loaderColors: Record<string, string> = {
		Vanilla: "var(--clr-loader-vanilla, #78909c)",
		Fabric: "var(--clr-loader-fabric, #66bb6a)",
		Forge: "var(--clr-loader-forge, #ffa726)",
		Quilt: "var(--clr-loader-quilt, #ab47bc)",
	};
	const loaderColor = $derived(loaderColors[instance.loader] || "#78909c");

	const statusLabel = $derived(
		instance.status === "started"
			? t("instanceView.status.started")
			: instance.status === "starting"
				? t("instanceView.status.starting")
				: instance.status === "error"
					? "Error"
					: t("instanceView.status.idle"),
	);
	const statusClass = $derived(
		instance.status === "started"
			? "status-started"
			: instance.status === "starting"
				? "status-starting"
				: instance.status === "error"
					? "status-error"
					: "status-idle",
	);

	function openDir(subDir?: string) {
		invoke("open_instance_dir", {
			id: instance.uuid,
			subDir: subDir ?? null,
		});
	}

	function openLogs() {
		invoke("open_log_window", {
			instanceId: instance.uuid,
			instanceName: instance.name,
		});
	}

</script>

<div class="details-panel">
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
				onclick={() => openDir()}
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

	<div class="action-bar">
		<button type="button" class="action-chip" onclick={() => openDir()}>
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
			{t("instanceView.options.folder")}
		</button>
		<button
			type="button"
			class="action-chip"
			onclick={() => openDir("mods")}
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
				<rect x="3" y="3" width="7" height="7" /><rect
					x="14"
					y="3"
					width="7"
					height="7"
				/><rect x="14" y="14" width="7" height="7" /><rect
					x="3"
					y="14"
					width="7"
					height="7"
				/>
			</svg>
			{t("instanceView.tabs.mods")}
		</button>
		<button
			type="button"
			class="action-chip"
			onclick={() => openDir("screenshots")}
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
					d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"
				/><circle cx="12" cy="13" r="4" />
			</svg>
			{t("instanceView.tabs.screenshots")}
		</button>
		<button
			type="button"
			class="action-chip"
			onclick={() => openDir("resourcepacks")}
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
				<ellipse cx="12" cy="5" rx="9" ry="3" /><path
					d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"
				/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
			</svg>
			{t("instanceView.tabs.resources")}
		</button>
		<button
			type="button"
			class="action-chip"
			onclick={() => openLogs()}
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
				<polyline points="16 18 22 12 16 6" /><polyline
					points="8 6 2 12 8 18"
				/>
			</svg>
			{t("instanceView.tabs.logs")}
		</button>
	</div>

</div>

<style>
	.details-panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		gap: 16px;
		padding: 0;
	}

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
		font-family: "Cantarell", system-ui, sans-serif;
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

	.action-bar {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.action-chip {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 12px;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: 6px;
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 0.72rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s ease;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.action-chip:hover {
		background: rgba(255, 255, 255, 0.07);
		color: var(--text-primary);
		border-color: rgba(255, 255, 255, 0.15);
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
