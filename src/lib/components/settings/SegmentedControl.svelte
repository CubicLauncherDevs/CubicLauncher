<script lang="ts">
	interface Option {
		value: string;
		label: string;
	}

	let {
		id,
		label,
		value,
		options,
		disabled = false,
		onchange,
	}: {
		id?: string;
		label?: string;
		value: string;
		options: Option[];
		disabled?: boolean;
		onchange: (value: string) => void;
	} = $props();
</script>

<div class="segmented-control" {id} role="group" aria-label={label}>
	{#each options as option (option.value)}
		<button
			type="button"
			aria-pressed={value === option.value}
			{disabled}
			onclick={() => onchange(option.value)}
		>
			{option.label}
		</button>
	{/each}
</div>

<style>
	.segmented-control {
		display: flex;
		gap: 3px;
		padding: 3px;
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		box-shadow: var(--shadow-inset);
	}
	.segmented-control button {
		flex: 1;
		min-width: 0;
		appearance: none;
		font-family: inherit;
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text-secondary);
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--border-radius-sm);
		padding: 7px 6px;
		min-height: 32px;
		cursor: pointer;
		white-space: nowrap;
		transition:
			background 0.15s,
			border-color 0.15s,
			color 0.15s;
	}
	.segmented-control button:hover:not(:disabled) {
		color: var(--text-primary);
		background: var(--surface-hover);
	}
	.segmented-control button[aria-pressed="true"] {
		background: var(--surface-selected);
		border-color: var(--border-hover);
		color: var(--text-primary);
		box-shadow: var(--shadow-sm);
	}
	.segmented-control button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.segmented-control button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
</style>
