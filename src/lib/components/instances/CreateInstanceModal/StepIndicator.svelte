<script lang="ts">
	let {
		currentStep = $bindable(0),
		totalSteps = 2,
	}: {
		currentStep: number;
		totalSteps?: number;
	} = $props();
</script>

<div class="step-indicator">
	{#each { length: totalSteps } as _, i (i)}
		{@const active = i === currentStep}
		{@const done = i < currentStep}
		<div class="step-dot" class:active class:done>
			{#if done}
				<svg
					width="12"
					height="12"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="3"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<polyline points="20 6 9 17 4 12"></polyline>
				</svg>
			{:else}
				<span>{i + 1}</span>
			{/if}
		</div>
		{#if i < totalSteps - 1}
			<div class="step-line" class:done></div>
		{/if}
	{/each}
</div>

<style>
	.step-indicator {
		display: flex;
		align-items: center;
		gap: 0;
		padding: 4px 0 12px;
	}

	.step-dot {
		width: 28px;
		height: 28px;
		border-radius: 50%;
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.06);
		border: 2px solid var(--border);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.7rem;
		font-weight: 700;
		color: var(--text-secondary);
		transition: all 0.2s;
		flex-shrink: 0;
	}

	.step-dot.active {
		border-color: var(--accent);
		color: var(--accent);
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.12);
	}

	.step-dot.done {
		border-color: var(--accent);
		background: var(--accent);
		color: var(--accent-text, #0c0c0c);
	}

	.step-line {
		flex: 1;
		height: 2px;
		background: var(--border);
		margin: 0 8px;
		transition: background 0.2s;
	}

	.step-line.done {
		background: var(--accent);
	}
</style>
