<script lang="ts">
	import type { InstanceDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import { invoke } from "@tauri-apps/api/core";
	import { launcherStore } from "$lib/state/state.svelte";
	import InfoHeader from "./InfoHeader.svelte";
	import ActionChips from "./ActionChips.svelte";

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

	const lang = $derived(launcherStore.settings.language);
	const formatter = $derived(
		new Intl.DateTimeFormat(lang, {
			year: "numeric",
			month: "long",
			day: "2-digit",
			hour: "2-digit",
			minute: "2-digit",
		}),
	);

	const lastPlayedLabel = $derived.by(() => {
		if (instance.last_played < 1) {
			return t("instanceView.neverPlayed");
		}
		return formatter.format(new Date(instance.last_played * 1000));
	});

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
	<InfoHeader
		{instance}
		{loaderColor}
		{statusLabel}
		{statusClass}
		onOpenDir={openDir}
	/>

	<div class="last-played-row">
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
			<circle cx="12" cy="12" r="10" /><polyline
				points="12 6 12 12 16 14"
			/>
		</svg>
		<span
			>{t("instanceView.lastPlayed", { date: lastPlayedLabel })}</span
		>
	</div>

	<ActionChips onOpenDir={openDir} onOpenLogs={openLogs} />
</div>

<style>
	.details-panel {
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.last-played-row {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 0.8rem;
		color: var(--text-secondary);
	}

	.last-played-row svg {
		flex-shrink: 0;
	}
</style>
