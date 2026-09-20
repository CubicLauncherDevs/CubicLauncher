<script lang="ts">
	import { fly } from "svelte/transition";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";
	import Icon from "$lib/icons/Icon.svelte";
	import { getIconPath } from "$lib/icons/registry";
	import { animDuration } from "$lib/utils/animations";

	interface Option {
		value: string;
		label: string;
		badge?: string;
		icon?: string;
		subtitle?: string;
		status?: "loading" | "download";
	}

	let {
		value = $bindable(),
		options = [],
		placeholder = "Seleccionar...",
		loadingPlaceholder = "Cargando...",
		disabled = false,
		loading = false,
		compact = false,
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
		compact?: boolean;
		label?: string;
		id?: string;
		onchange?: (value: string) => void;
	}>();

	let isOpen = $state(false);
	let container: HTMLDivElement;
	let triggerEl: HTMLButtonElement;
	let dropdownEl = $state<HTMLDivElement>();
	let dropdownStyles = $state("");
	let activeIndex = $state(-1);
	let typed = "";
	let lastTypedAt = 0;
	const uid = $props.id();
	const listId = `${uid}-list`;
	const labelId = `${uid}-label`;

	const triggerDisabled = $derived(disabled || loading);
	const triggerOpen = $derived(isOpen && !triggerDisabled);
	const flyDuration = $derived(animDuration(200));

	$effect(() => {
		if (triggerDisabled && isOpen) close();
	});

	$effect(() => {
		if (!triggerOpen || !dropdownEl) return;
		const option = dropdownEl.children.item(activeIndex);
		if (option instanceof HTMLElement) {
			// Scroll only the menu; scrolling an ancestor would close the popup.
			if (option.offsetTop < dropdownEl.scrollTop)
				dropdownEl.scrollTop = option.offsetTop;
			else if (
				option.offsetTop + option.offsetHeight >
				dropdownEl.scrollTop + dropdownEl.clientHeight
			) {
				dropdownEl.scrollTop =
					option.offsetTop +
					option.offsetHeight -
					dropdownEl.clientHeight;
			}
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
		const rect = triggerEl.getBoundingClientRect();
		const margin = 8;
		const width = Math.min(
			Math.max(rect.width, compact ? 240 : rect.width),
			window.innerWidth - margin * 2,
		);
		const left = Math.max(
			margin,
			Math.min(rect.left, window.innerWidth - width - margin),
		);
		const below = window.innerHeight - rect.bottom - margin * 2;
		const above = rect.top - margin * 2;
		const openAbove = below < 240 && above > below;
		const maxHeight = Math.max(0, Math.min(280, openAbove ? above : below));
		const vertical = openAbove
			? `bottom:${window.innerHeight - rect.top + margin}px`
			: `top:${rect.bottom + margin}px`;
		dropdownStyles = `${vertical};left:${left}px;width:${width}px;max-height:${maxHeight}px`;
	}

	function open() {
		if (triggerDisabled || options.length === 0) return;
		activeIndex = Math.max(
			0,
			options.findIndex((option: Option) => option.value === value),
		);
		updateDropdownPosition();
		isOpen = true;
	}

	function close() {
		isOpen = false;
		dropdownStyles = "";
		typed = "";
	}

	function toggle() {
		if (triggerOpen) close();
		else open();
	}

	function selectOption(option: Option) {
		if (triggerDisabled) return;
		value = option.value;
		close();
		triggerEl.focus({ preventScroll: true });
		onchange?.(value);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (triggerDisabled || event.isComposing) return;
		if (event.key === "Tab") {
			close();
			return;
		}
		if (event.key === "Escape") {
			if (!triggerOpen) return;
			event.preventDefault();
			event.stopPropagation();
			close();
			triggerEl.focus({ preventScroll: true });
			return;
		}
		if (event.ctrlKey || event.metaKey || event.altKey) return;
		if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
			event.preventDefault();
			if (!triggerOpen) open();
			else if (event.key === "ArrowDown")
				activeIndex = Math.min(options.length - 1, activeIndex + 1);
			else if (event.key === "ArrowUp")
				activeIndex = Math.max(0, activeIndex - 1);
			if (event.key === "Home") activeIndex = 0;
			if (event.key === "End") activeIndex = options.length - 1;
		} else if (event.key === "Enter" || event.key === " ") {
			event.preventDefault();
			if (triggerOpen && options[activeIndex])
				selectOption(options[activeIndex]);
			else open();
		} else if (event.key.length === 1) {
			event.preventDefault();
			if (!triggerOpen) open();
			const now = performance.now();
			typed = now - lastTypedAt > 700 ? event.key : typed + event.key;
			lastTypedAt = now;
			const index = options.findIndex((option: Option) =>
				[option.label, option.subtitle].some((text) =>
					text
						?.toLocaleLowerCase()
						.startsWith(typed.toLocaleLowerCase()),
				),
			);
			if (index >= 0) activeIndex = index;
		}
	}

	function handleOutside(event: Event) {
		if (container && !container.contains(event.target as Node)) {
			if (!dropdownEl || !dropdownEl.contains(event.target as Node)) {
				close();
			}
		}
	}

	$effect(() => {
		if (triggerOpen) {
			const onScroll = (e: Event) => {
				if (dropdownEl && dropdownEl.contains(e.target as Node)) return;
				close();
			};
			window.addEventListener("scroll", onScroll, true);
			window.addEventListener("resize", close);
			window.addEventListener("pointerdown", handleOutside, true);
			window.addEventListener("focusin", handleOutside, true);
			return () => {
				window.removeEventListener("scroll", onScroll, true);
				window.removeEventListener("resize", close);
				window.removeEventListener("pointerdown", handleOutside, true);
				window.removeEventListener("focusin", handleOutside, true);
			};
		}
	});

	const selectedLabel = $derived(
		options.find((o: Option) => o.value === value)?.label || placeholder,
	);

	const selectedIcon = $derived(
		options.find((o: Option) => o.value === value)?.icon,
	);
	const selectedOption = $derived(
		options.find((o: Option) => o.value === value),
	);
</script>

<div class="custom-select-container" class:compact bind:this={container} {id}>
	{#if label}
		<span class="input-label" id={labelId}>{label}</span>
	{/if}

	<button
		type="button"
		class="select-trigger"
		class:disabled={triggerDisabled}
		class:open={triggerOpen}
		disabled={triggerDisabled}
		onclick={toggle}
		onkeydown={handleKeydown}
		role="combobox"
		aria-labelledby={label ? labelId : undefined}
		aria-label={label ? undefined : placeholder}
		aria-expanded={triggerOpen}
		aria-haspopup="listbox"
		aria-controls={triggerOpen ? listId : undefined}
		aria-activedescendant={triggerOpen && options[activeIndex]
			? `${uid}-option-${activeIndex}`
			: undefined}
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
					{:else if getIconPath(selectedIcon)}
						<Icon name={selectedIcon} class="option-img" />
					{:else}
						{selectedIcon}
					{/if}
				</span>
			{/if}
			<span class="option-copy">
				<span class="select-option-label"
					>{loading ? loadingPlaceholder : selectedLabel}</span
				>
				{#if !loading && selectedOption?.subtitle}
					<span class="option-subtitle"
						>{selectedOption.subtitle}</span
					>
				{/if}
			</span>
		</span>
		{#if !loading && selectedOption?.status === "loading"}
			<span class="select-spinner" aria-hidden="true"></span>
		{/if}
		{#if !loading}
			<ChevronDownIcon size={16} class="chevron-icon" />
		{/if}
	</button>

	{#if triggerOpen}
		<div
			use:portal
			bind:this={dropdownEl}
			class="select-dropdown"
			class:compact
			id={listId}
			style={dropdownStyles}
			transition:fly={{ y: 8, duration: flyDuration }}
			role="listbox"
			aria-labelledby={label ? labelId : undefined}
			aria-label={label ? undefined : placeholder}
		>
			{#each options as option, index (option.value)}
				<div
					class="select-option"
					id={`${uid}-option-${index}`}
					class:highlighted={index === activeIndex}
					class:selected={option.value === value}
					onclick={() => selectOption(option)}
					onkeydown={handleKeydown}
					role="option"
					aria-selected={option.value === value}
					tabindex="-1"
				>
					{#if option.icon}
						<span class="option-icon">
							{#if option.icon.startsWith("/")}
								<img
									src={option.icon}
									alt=""
									class="option-img"
								/>
							{:else if getIconPath(option.icon)}
								<Icon name={option.icon} class="option-img" />
							{:else}
								{option.icon}
							{/if}
						</span>
					{/if}
					<span class="option-copy">
						<span class="select-option-label">{option.label}</span>
						{#if option.subtitle}
							<span class="option-subtitle"
								>{option.subtitle}</span
							>
						{/if}
					</span>
					{#if option.badge}
						<span class="select-option-badge">{option.badge}</span>
					{/if}
					{#if option.status}
						<span class="option-status" aria-hidden="true">
							{#if option.status === "loading"}
								<span class="select-spinner"></span>
							{:else}
								<Icon name="ui:download" size={14} />
							{/if}
						</span>
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
	.selected-value {
		display: flex;
		align-items: center;
		min-width: 0;
		flex: 1;
	}
	.option-copy {
		display: flex;
		flex-direction: column;
		gap: 3px;
		min-width: 0;
		flex: 1;
		text-align: start;
	}
	.option-subtitle {
		font-size: var(--font-size-label);
		font-weight: normal;
		color: var(--text-secondary);
		line-height: 1.4;
		white-space: normal;
		overflow-wrap: anywhere;
	}
	.option-status {
		display: inline-flex;
		color: var(--text-muted);
		flex-shrink: 0;
	}
	.select-trigger:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.select-dropdown {
		box-sizing: border-box;
		overscroll-behavior: contain;
		scrollbar-width: none;
	}
	.select-dropdown::-webkit-scrollbar {
		display: none;
	}
	.select-option.highlighted {
		background: var(--surface-hover);
		color: var(--text-primary);
		outline: 1px solid var(--accent);
		outline-offset: -1px;
	}
	.compact .input-label {
		font-size: var(--font-size-label);
		font-weight: var(--font-weight-bold);
		letter-spacing: 0.06em;
		color: var(--text-secondary);
		text-transform: uppercase;
		margin: 0;
	}
	.compact .select-trigger {
		min-height: var(--control-min-height, 38px);
		padding: var(--control-compact-padding, 8px 12px);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-medium);
		border-radius: var(--border-radius);
		background: var(--bg-card-gradient), var(--surface-input);
		box-shadow: var(--shadow-sm);
	}
	.compact .select-trigger:hover:not(:disabled),
	.compact .select-trigger.open {
		border-color: var(--accent);
		background: var(--surface-hover);
	}
	.compact .select-trigger.open {
		box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 15%, transparent);
	}
	.select-dropdown.compact {
		padding: var(--select-menu-padding, 6px);
		border-radius: var(--border-radius);
		border-color: color-mix(in srgb, var(--accent) 25%, var(--border));
		box-shadow: var(--shadow-lg, 0 12px 32px rgba(0, 0, 0, 0.3));
	}
	.compact .select-option {
		min-height: var(--select-option-min-height, 36px);
		box-sizing: border-box;
		padding: var(--control-compact-padding, 8px 10px);
		gap: var(--space-sm);
		font-size: var(--font-size-sm);
		border: var(--border-width) solid transparent;
	}
	.compact .select-option.selected {
		background: color-mix(
			in srgb,
			var(--accent) 12%,
			var(--surface-dropdown)
		);
		border-color: color-mix(in srgb, var(--accent) 25%, transparent);
		color: var(--text-primary);
	}
	.compact .select-option-label {
		white-space: normal;
		overflow-wrap: anywhere;
	}
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

	@media (prefers-reduced-motion: reduce) {
		.select-spinner {
			animation: none;
		}
	}
</style>
