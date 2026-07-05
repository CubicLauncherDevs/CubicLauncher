<script lang="ts">
	import { t } from "$lib/i18n";
	import ModalBase from "../ModalBase.svelte";

	let {
		open = $bindable(false),
		instanceName,
		onconfirm,
	}: {
		open?: boolean;
		instanceName: string;
		onconfirm: () => void;
	} = $props();
</script>

<ModalBase bind:open title={t("sidebar.modals.deleteTitle")}>
	<p
		style="font-size: 0.9rem; color: var(--text-secondary); line-height: 1.4;"
	>
		{t("sidebar.modals.deleteDesc1")}
		<strong style="color: var(--text-primary);">"{instanceName}"</strong>{t(
			"sidebar.modals.deleteDesc2",
		)}
	</p>
	{#snippet footer()}
		<button
			type="button"
			class="btn-secondary"
			onclick={() => (open = false)}>{t("sidebar.modals.cancel")}</button
		>
		<button
			type="button"
			class="btn-primary"
			style="background: var(--color-error); color: white;"
			onclick={onconfirm}>{t("sidebar.modals.deleteBtn")}</button
		>
	{/snippet}
</ModalBase>
