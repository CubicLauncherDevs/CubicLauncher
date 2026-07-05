<script lang="ts">
	import type { InstanceDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import { invoke } from "@tauri-apps/api/core";
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
	<ActionChips onOpenDir={openDir} onOpenLogs={openLogs} />
</div>

<style>
	.details-panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		gap: 16px;
		padding: 0;
	}
</style>
