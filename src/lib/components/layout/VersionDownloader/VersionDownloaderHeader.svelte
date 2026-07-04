<script lang="ts">
	import { t } from "$lib/i18n";

	let {
		onclose,
		onrefresh,
		refreshing = false,
	}: {
		onclose?: () => void;
		onrefresh: () => void;
		refreshing?: boolean;
	} = $props();
</script>

<div class="qm-header">
	<span class="qm-label">{t("versionDownloader.title")}</span>
	<div style="display: flex; align-items: center; gap: 8px;">
		<button
			type="button"
			onclick={onrefresh}
			disabled={refreshing}
			title={t("versionDownloader.refreshBtn")}
			style="background: none; border: none; color: var(--text-muted); cursor: pointer; padding: 4px; display: flex; align-items: center; border-radius: 4px; transition: color 0.2s;"
			onmouseenter={(e) =>
				(e.currentTarget.style.color = "var(--text-primary)")}
			onmouseleave={(e) =>
				(e.currentTarget.style.color = "var(--text-muted)")}
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
				class:spin={refreshing}
				style={refreshing
					? "animation: spin 1s linear infinite; will-change: transform;"
					: ""}
			>
				<polyline points="23 4 23 10 17 10"></polyline>
				<path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
			</svg>
		</button>
		<button type="button" class="qm-close-btn" onclick={onclose}
			>✕</button
		>
	</div>
</div>

<style>
	@keyframes spin {
		from { transform: rotate(0deg); }
		to { transform: rotate(360deg); }
	}
</style>