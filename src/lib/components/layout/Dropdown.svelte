<script lang="ts">
	import { fly } from "svelte/transition";
	import { onMount } from "svelte";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";

	interface Option {
		value: string;
		label: string;
		subtitle?: string;
	}

	let {
		value = $bindable(),
		options = [] as Option[],
		placeholder = "Select...",
		disabled = false,
		label,
		id,
		onchange,
	} = $props<{
		value: string;
		options: Option[];
		placeholder?: string;
		disabled?: boolean;
		label?: string;
		id?: string;
		onchange?: (value: string) => void;
	}>();

	let isOpen = $state(false);
	let container: HTMLDivElement;

	function toggle() {
		if (disabled) return;
		isOpen = !isOpen;
	}

	function selectOption(option: Option) {
		value = option.value;
		isOpen = false;
		onchange?.(value);
	}

	function handleClickOutside(event: MouseEvent) {
		if (container && !container.contains(event.target as Node)) {
			isOpen = false;
		}
	}

	onMount(() => {
		window.addEventListener("click", handleClickOutside);
		return () => window.removeEventListener("click", handleClickOutside);
	});

	let selectedLabel = $derived(
		(options as Option[]).find((o) => o.value === value)?.label ??
			placeholder,
	);
</script>

<div class="dd-container" bind:this={container} {id}>
	{#if label}
		<span class="dd-label">{label}</span>
	{/if}

	<button
		type="button"
		class="dd-trigger"
		class:dd-disabled={disabled}
		class:dd-open={isOpen}
		onclick={toggle}
		aria-expanded={isOpen}
		aria-haspopup="listbox"
	>
		<span class="dd-selected">{selectedLabel}</span>
		<ChevronDownIcon size={16} class="dd-chevron" />
	</button>

	{#if isOpen}
		<div
			class="dd-dropdown"
			transition:fly={{ y: 8, duration: 200 }}
			role="listbox"
		>
			{#each options as option (option.value)}
				<div
					class="dd-option"
					class:dd-selected={option.value === value}
					onclick={() => selectOption(option)}
					onkeydown={(e) => e.key === "Enter" && selectOption(option)}
					role="option"
					aria-selected={option.value === value}
					tabindex="0"
				>
					<div class="dd-option-content">
						<span class="dd-option-label">{option.label}</span>
						{#if option.subtitle}
							<span class="dd-option-subtitle"
								>{option.subtitle}</span
							>
						{/if}
					</div>
					{#if option.value === value}
						<CheckIcon size={14} class="dd-check" />
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.dd-container {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: 6px;
		width: 100%;
	}

	.dd-label {
		font-size: 0.68rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 1px;
		color: var(--text-secondary);
	}

	.dd-trigger {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 10px 14px;
		color: var(--text-primary);
		font-family: inherit;
		font-size: 0.85rem;
		cursor: pointer;
		transition: all 0.2s ease;
		text-align: left;
		width: 100%;
		outline: none;
	}

	.dd-trigger:hover:not(.dd-disabled) {
		background: rgba(255, 255, 255, 0.06);
		border-color: rgba(255, 255, 255, 0.2);
	}

	.dd-trigger.dd-open {
		border-color: rgba(255, 255, 255, 0.3);
		background: rgba(255, 255, 255, 0.06);
		box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.02);
	}

	.dd-trigger.dd-disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.dd-selected {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.dd-dropdown {
		position: absolute;
		top: calc(100% + 8px);
		left: 0;
		right: 0;
		background: #121212;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
		z-index: 1000;
		max-height: 240px;
		overflow-y: auto;
		padding: 6px;
		backdrop-filter: blur(var(--backdrop-blur-dropdown, 10px));
	}

	.dd-option {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 12px;
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font-size: 0.85rem;
		cursor: pointer;
		transition: all 0.15s ease;
		margin-bottom: 2px;
	}

	.dd-option:last-child {
		margin-bottom: 0;
	}

	.dd-option:hover {
		background: rgba(255, 255, 255, 0.05);
		color: var(--text-primary);
	}

	.dd-option.dd-selected {
		background: rgba(255, 255, 255, 0.03);
		color: var(--text-primary);
		font-weight: 600;
		border: 1px solid rgba(255, 255, 255, 0.05);
	}

	.dd-option-content {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.dd-option-label {
		font-size: 0.85rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.dd-option-subtitle {
		font-size: 0.65rem;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.dd-dropdown:global(::-webkit-scrollbar) {
		width: 4px;
	}

	.dd-dropdown:global(::-webkit-scrollbar-track) {
		background: transparent;
	}

	.dd-dropdown:global(::-webkit-scrollbar-thumb) {
		background: var(--border);
		border-radius: 10px;
	}
</style>
