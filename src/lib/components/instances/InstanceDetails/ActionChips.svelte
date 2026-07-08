<script lang="ts">
	import { t } from "$lib/i18n";

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
			<path
				d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
			/>
		</svg>
	</button>
	<button
		type="button"
		class="action-btn"
		onclick={() => onOpenDir("mods")}
		title={t("instanceView.tabs.mods")}
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
	</button>
	<button
		type="button"
		class="action-btn"
		onclick={() => onOpenDir("screenshots")}
		title={t("instanceView.tabs.screenshots")}
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
			<path
				d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"
			/><circle cx="12" cy="13" r="4" />
		</svg>
	</button>
	<button
		type="button"
		class="action-btn"
		onclick={() => onOpenDir("resourcepacks")}
		title={t("instanceView.tabs.resources")}
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
			<ellipse cx="12" cy="5" rx="9" ry="3" /><path
				d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"
			/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
		</svg>
	</button>
	<button
		type="button"
		class="action-btn"
		onclick={() => onOpenLogs()}
		title={t("instanceView.tabs.logs")}
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
			<polyline points="16 18 22 12 16 6" /><polyline
				points="8 6 2 12 8 18"
			/>
		</svg>
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
