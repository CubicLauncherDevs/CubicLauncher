<script lang="ts">
	import { fly } from "svelte/transition";
	import { onMount } from "svelte";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";

	interface Option {
		value: string;
		label: string;
		badge?: string;
		icon?: string;
	}

	let {
		value = $bindable(),
		options = [],
		placeholder = "Seleccionar...",
		loadingPlaceholder = "Cargando...",
		disabled = false,
		loading = false,
		label,
		id,
		onchange,
	} = $props<{
		value: string;
		options: Option[];
		placeholder?: string;
		loadingPlaceholder?: string;
		disabled?: boolean;
		loading?: boolean;
		label?: string;
		id?: string;
		onchange?: (value: string) => void;
	}>();

	let isOpen = $state(false);
	let container: HTMLDivElement;
	let triggerEl: HTMLButtonElement;
	let dropdownEl = $state<HTMLDivElement>();
	let dropdownStyles = $state("");

	const triggerDisabled = $derived(disabled || loading);
	const triggerOpen = $derived(isOpen && !loading);

	$effect(() => {
		if (loading && isOpen) {
			isOpen = false;
			dropdownStyles = "";
		}
	});

	function portal(el: HTMLElement) {
		document.body.appendChild(el);
		return {
			destroy() {
				el.remove();
			},
		};
	}

	function updateDropdownPosition() {
		const rect = triggerEl!.getBoundingClientRect();
		dropdownStyles = `top:${rect.bottom + 8}px;left:${rect.left}px;width:${rect.width}px`;
	}

	function toggle() {
		if (triggerDisabled) return;
		isOpen = !isOpen;
		if (isOpen) {
			updateDropdownPosition();
		} else {
			dropdownStyles = "";
		}
	}

	function selectOption(option: Option) {
		if (loading) return;
		value = option.value;
		isOpen = false;
		dropdownStyles = "";
		onchange?.(value);
	}

	function handleClickOutside(event: MouseEvent) {
		if (container && !container.contains(event.target as Node)) {
			if (!dropdownEl || !dropdownEl.contains(event.target as Node)) {
				isOpen = false;
				dropdownStyles = "";
			}
		}
	}

	$effect(() => {
		if (isOpen) {
			const onScroll = (e: Event) => {
				if (dropdownEl && dropdownEl.contains(e.target as Node)) return;
				isOpen = false;
				dropdownStyles = "";
			};
			window.addEventListener("scroll", onScroll, true);
			return () => window.removeEventListener("scroll", onScroll, true);
		}
	});

	onMount(() => {
		window.addEventListener("click", handleClickOutside, true);
		return () =>
			window.removeEventListener("click", handleClickOutside, true);
	});

	const selectedLabel = $derived(
		options.find((o: Option) => o.value === value)?.label || placeholder,
	);

	const selectedIcon = $derived(
		options.find((o: Option) => o.value === value)?.icon,
	);
</script>

<div class="custom-select-container" bind:this={container} {id}>
	{#if label}
		<span class="input-label">{label}</span>
	{/if}

	<button
		type="button"
		class="select-trigger"
		class:disabled={triggerDisabled}
		class:open={triggerOpen}
		onclick={toggle}
		aria-expanded={triggerOpen}
		aria-haspopup="listbox"
		aria-busy={loading}
		bind:this={triggerEl}
	>
		{#if loading}
			<span class="select-spinner" aria-hidden="true"></span>
		{/if}
		<span class="selected-value">
			{#if !loading && selectedIcon}
				<span class="option-icon">
					{#if selectedIcon.startsWith("/")}
						<img src={selectedIcon} alt="" class="option-img" />
					{:else}
						{selectedIcon}
					{/if}
				</span>
			{/if}
			{loading ? loadingPlaceholder : selectedLabel}
		</span>
		{#if !loading}
			<ChevronDownIcon size={16} class="chevron-icon" />
		{/if}
	</button>

	{#if triggerOpen}
		<div
			use:portal
			bind:this={dropdownEl}
			class="select-dropdown"
			style={dropdownStyles}
			transition:fly={{ y: 8, duration: 200 }}
			role="listbox"
		>
			{#each options as option (option.value)}
				<div
					class="select-option"
					class:selected={option.value === value}
					onclick={() => selectOption(option)}
					onkeydown={(e) => e.key === "Enter" && selectOption(option)}
					role="option"
					aria-selected={option.value === value}
					tabindex="0"
				>
					{#if option.icon}
						<span class="option-icon">
							{#if option.icon.startsWith("/")}
								<img
									src={option.icon}
									alt=""
									class="option-img"
								/>
							{:else}
								{option.icon}
							{/if}
						</span>
					{/if}
					<span class="select-option-label">{option.label}</span>
					{#if option.badge}
						<span class="select-option-badge">{option.badge}</span>
					{/if}
					{#if option.value === value}
						<CheckIcon size={14} class="check-icon" />
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.select-spinner {
		width: 14px;
		height: 14px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: select-spin 0.8s linear infinite;
		flex-shrink: 0;
	}

	@keyframes select-spin {
		to {
			transform: rotate(360deg);
		}
	}

	.option-icon {
		font-size: 1.2em;
		line-height: 1;
		margin-right: 6px;
		flex-shrink: 0;
		display: inline-flex;
		align-items: center;
	}

	.option-img {
		width: 1.2em;
		height: 1.2em;
		display: block;
		filter: var(--icon-filter);
	}

	.selected-value .option-icon {
		margin-right: 4px;
	}
</style>
