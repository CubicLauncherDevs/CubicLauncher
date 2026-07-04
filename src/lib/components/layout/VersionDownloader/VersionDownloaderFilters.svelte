<script lang="ts">
	import { t } from "$lib/i18n";
	import Select from "../Select.svelte";

	let {
		search = $bindable(""),
		installStatusFilter = $bindable("all"),
		majorVersionFilter = $bindable("all"),
		fabricStabilityFilter = $bindable("stable"),
		majorVersionOptions = [],
		filter = "release",
	}: {
		search?: string;
		installStatusFilter?: string;
		majorVersionFilter?: string;
		fabricStabilityFilter?: string;
		majorVersionOptions?: Array<{ value: string; label: string }>;
		filter?: string;
	} = $props();
</script>

<div
	class="qm-search-container"
	style="padding: 10px 20px; display: flex; flex-direction: column; gap: 10px;"
>
	<input
		type="text"
		placeholder={t("versionDownloader.searchPlaceholder")}
		bind:value={search}
		style="width: 100%; background: var(--bg-input); border: 1px solid var(--border-color); color: var(--text-primary); padding: 8px 12px; border-radius: 8px; font-size: 0.85rem;"
	/>
	<div
		class="qm-filters-grid"
		style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 16px; margin-top: 4px; padding-bottom: 8px;"
	>
		<Select
			label={t("versionDownloader.filters.installStatus")}
			options={[
				{ value: "all", label: t("versionDownloader.filters.all") },
				{
					value: "installed",
					label: t("versionDownloader.filters.installedOnly"),
				},
				{
					value: "not_installed",
					label: t("versionDownloader.filters.notInstalledOnly"),
				},
			]}
			bind:value={installStatusFilter}
		/>

		<Select
			label={t("versionDownloader.filters.majorVersion")}
			options={majorVersionOptions}
			bind:value={majorVersionFilter}
		/>

		{#if filter === "fabric"}
			<Select
				label={t("versionDownloader.filters.fabricStability")}
				options={[
					{ value: "all", label: t("versionDownloader.filters.all") },
					{
						value: "stable",
						label: t("versionDownloader.filters.stableOnly"),
					},
					{
						value: "unstable",
						label: t("versionDownloader.filters.unstableOnly"),
					},
				]}
				bind:value={fabricStabilityFilter}
			/>
		{/if}
	</div>
</div>