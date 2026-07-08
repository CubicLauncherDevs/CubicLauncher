<script lang="ts">
	import { t } from "$lib/i18n";

	let {
		currentStep,
		totalSteps,
		onprev,
		onnext,
		ongoToStep,
		onfinish,
	}: {
		currentStep: number;
		totalSteps: number;
		onprev: () => void;
		onnext: () => void;
		ongoToStep: (i: number) => void;
		onfinish: () => void;
	} = $props();
</script>

<div class="tut-footer">
	<div class="tut-dots">
		{#each Array(totalSteps) as _, i (i)}
			<button
				type="button"
				class="tut-dot"
				class:active={i === currentStep}
				onclick={() => ongoToStep(i)}
				aria-label="Step {i + 1}"
			></button>
		{/each}
	</div>
	<div class="tut-nav">
		{#if currentStep > 0}
			<button type="button" class="btn-secondary tut-btn" onclick={onprev}
				>{t("tutorial.prev")}</button
			>
		{/if}
		{#if currentStep < totalSteps - 1}
			<button type="button" class="btn-primary tut-btn" onclick={onnext}
				>{t("tutorial.next")}</button
			>
		{:else}
			<button type="button" class="btn-primary tut-btn" onclick={onfinish}
				>{t("tutorial.finish")}</button
			>
		{/if}
	</div>
</div>
