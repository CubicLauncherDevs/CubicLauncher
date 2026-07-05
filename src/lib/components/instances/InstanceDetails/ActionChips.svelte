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
	<button type="button" class="action-chip" onclick={() => onOpenDir()}>
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
	<button type="button" class="action-chip" onclick={() => onOpenDir("mods")}>
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
		onclick={() => onOpenDir("screenshots")}
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
		onclick={() => onOpenDir("resourcepacks")}
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
	<button type="button" class="action-chip" onclick={() => onOpenLogs()}>
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

<style>
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
</style>
