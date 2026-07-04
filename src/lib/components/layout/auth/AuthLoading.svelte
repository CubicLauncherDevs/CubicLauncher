<script lang="ts">
	let {
		title = "Conectando...",
		subtitle = undefined as string | undefined,
	} = $props<{
		title?: string;
		subtitle?: string;
	}>();
</script>

<div class="state-container">
	<div class="minimal-spinner"></div>
	<h3 class="state-title">{title}</h3>
	{#if subtitle}
		<p class="state-subtitle">{subtitle}</p>
	{/if}
</div>

<style>
	.state-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		width: 100%;
		animation: fadeIn 0.4s ease;
	}

	.state-title {
		font-size: 1.2rem;
		font-weight: 700;
		margin: 0 0 0.5rem 0;
		color: var(--text-primary);
	}

	.state-subtitle {
		font-size: 0.85rem;
		color: var(--text-secondary);
		margin: 0;
		max-width: 80%;
		line-height: 1.5;
	}

	.minimal-spinner {
		width: 32px;
		height: 32px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
		will-change: transform;
		margin-bottom: 1.5rem;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
</style>