<script lang="ts">
	import { t } from "$lib/i18n";

	let {
		filter = $bindable("release"),
		showSnapshots = false,
		showAlpha = false,
	}: {
		filter?: string;
		showSnapshots?: boolean;
		showAlpha?: boolean;
	} = $props();
</script>

<div class="qm-tabs">
	<button
		type="button"
		class="qm-tab-btn"
		class:active={filter === "release"}
		onclick={() => (filter = "release")}
	>
		<span class="qm-tab-label"
			>{t("versionDownloader.tabs.releases")}</span
		>
	</button>
	{#if showSnapshots}
		<button
			type="button"
			class="qm-tab-btn"
			class:active={filter === "snapshot"}
			onclick={() => (filter = "snapshot")}
		>
			<span class="qm-tab-label"
				>{t("versionDownloader.tabs.snapshots")}</span
			>
		</button>
	{/if}
	{#if showAlpha}
		<button
			type="button"
			class="qm-tab-btn"
			class:active={filter === "alpha"}
			onclick={() => (filter = "alpha")}
		>
			<span class="qm-tab-label"
				>{t("versionDownloader.tabs.alphas")}</span
			>
		</button>
	{/if}
	<button
		type="button"
		class="qm-tab-btn"
		class:active={filter === "fabric"}
		onclick={() => (filter = "fabric")}
	>
		<span class="qm-tab-label"
			>{t("versionDownloader.tabs.fabric")}</span
		>
	</button>
	<button
		type="button"
		class="qm-tab-btn"
		class:active={filter === "forge"}
		onclick={() => (filter = "forge")}
	>
		<span class="qm-tab-label">{t("versionDownloader.tabs.forge")}</span
		>
	</button>
	<button
		type="button"
		class="qm-tab-btn"
		class:active={filter === "quilt"}
		onclick={() => (filter = "quilt")}
	>
		<span class="qm-tab-label">{t("versionDownloader.tabs.quilt")}</span
		>
	</button>
</div>