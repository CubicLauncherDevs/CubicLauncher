<script lang="ts">
	import { fly } from "svelte/transition";
	import { onMount } from "svelte";
	import CheckIcon from "#lib/icons/CheckIcon.svelte";
	import ChevronDownIcon from "#lib/icons/ChevronDownIcon.svelte";
	import { animDuration } from "#lib/utils/animations.js";

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
	}: {
		value: string;
		options: Option[];
		placeholder?: string;
		disabled?: boolean;
		label?: string;
		id?: string;
		onchange?: (value: string) => void;
	} = $props();

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

	const flyDuration = $derived(animDuration(200));
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
			transition:fly={{ y: 8, duration: flyDuration }}
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
		gap: var(--input-group-gap, 6px);
		width: 100%;
	}

	.dd-label {
		font-size: var(--font-size-label);
		font-weight: var(--font-weight-bold);
		text-transform: uppercase;
		letter-spacing: 1px;
		color: var(--text-secondary);
	}

	.dd-trigger {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background: var(--surface-selected);
		border: var(--border-width) solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: var(--select-padding, var(--control-padding));
		color: var(--text-primary);
		font-family: inherit;
		font-size: var(--font-size-control);
		cursor: pointer;
		transition: all var(--transition-normal) ease;
		text-align: left;
		width: 100%;
		outline: none;
	}

	.dd-trigger:hover:not(.dd-disabled) {
		background: var(--surface-active);
		border-color: var(--border-hover);
	}

	.dd-trigger.dd-open {
		border-color: var(--border-focus);
		background: var(--surface-active);
		box-shadow: var(
			--control-focus-shadow,
			0 0 0 2px var(--surface-subtle)
		);
	}

	.dd-trigger.dd-disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}

	.dd-selected {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.dd-dropdown {
		position: absolute;
		top: calc(100% + var(--space-sm));
		left: 0;
		right: 0;
		background: var(--surface-dropdown);
		border: var(--border-width) solid var(--border);
		border-radius: var(--border-radius-sm);
		box-shadow: var(--shadow-floating);
		z-index: 1000;
		max-height: var(--select-max-height, 240px);
		overflow-y: auto;
		padding: var(--select-menu-padding, 6px);
		backdrop-filter: blur(var(--backdrop-blur-dropdown, 4px));
	}

	.dd-option {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--control-padding);
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font-size: var(--font-size-control);
		cursor: pointer;
		transition: all var(--transition-fast) ease;
		margin-bottom: 2px;
	}

	.dd-option:last-child {
		margin-bottom: 0;
	}

	.dd-option:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.dd-option.dd-selected {
		background: var(--surface-selected);
		color: var(--text-primary);
		font-weight: var(--font-weight-medium);
		border: var(--border-width) solid var(--border);
	}

	.dd-option-content {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.dd-option-label {
		font-size: var(--font-size-control);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.dd-option-subtitle {
		font-size: var(--font-size-label);
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	:global(.dd-dropdown::-webkit-scrollbar) {
		width: var(--scrollbar-size);
	}

	:global(.dd-dropdown::-webkit-scrollbar-track) {
		background: transparent;
	}

	:global(.dd-dropdown::-webkit-scrollbar-thumb) {
		background: var(--scrollbar-thumb);
		border-radius: var(--scrollbar-radius);
	}
</style>
