<script lang="ts">
	import { t } from "#lib/i18n/index.js";
	import Icon from "#lib/icons/Icon.svelte";
	import Select from "#lib/components/layout/Select.svelte";
	import {
		OPTION_BY_KEY,
		OPTION_CATEGORIES,
		OPTIONS_BY_CATEGORY,
		formatRangeValue,
		parseOptions,
		rangeDisplayValue,
		rangeRawValue,
		removeOption,
		setOption,
		type OptionDefinition,
	} from "#lib/utils/minecraftOptions.js";

	let {
		content = $bindable(""),
		disabled = false,
	}: {
		content?: string;
		disabled?: boolean;
	} = $props();

	let query = $state("");

	const values = $derived(parseOptions(content));
	const advancedCount = $derived(
		[...values.keys()].filter((key) => !OPTION_BY_KEY.has(key)).length,
	);
	const normalizedQuery = $derived(query.trim().toLowerCase());
	const filteredCategories = $derived(
		OPTION_CATEGORIES.map((category) => {
			const all = OPTIONS_BY_CATEGORY[category];
			const categoryMatches =
				normalizedQuery.length > 0 &&
				t(`optionsProfiles.categories.${category}`)
					.toLowerCase()
					.includes(normalizedQuery);
			return {
				category,
				options: categoryMatches
					? all
					: all.filter((option) => {
							if (!normalizedQuery) return true;
							return (
								option.key
									.toLowerCase()
									.includes(normalizedQuery) ||
								labelFor(option)
									.toLowerCase()
									.includes(normalizedQuery)
							);
						}),
			};
		}).filter((group) => group.options.length > 0),
	);

	function rawFor(option: OptionDefinition): string {
		return values.get(option.key) ?? option.defaultRaw;
	}

	function isPresent(option: OptionDefinition): boolean {
		return values.has(option.key);
	}

	function canReset(option: OptionDefinition): boolean {
		return isPresent(option) && rawFor(option) !== option.defaultRaw;
	}

	function update(option: OptionDefinition, value: string): void {
		content = setOption(content, option.key, value);
	}

	function reset(option: OptionDefinition): void {
		content = removeOption(content, option.key);
	}

	function toggle(option: OptionDefinition, checked: boolean): void {
		update(option, checked ? "true" : "false");
	}

	function labelFor(option: OptionDefinition): string {
		return t(`optionsProfiles.fields.${option.key}`);
	}

	function choicesFor(
		option: Extract<OptionDefinition, { kind: "select" }>,
	): { value: string; label: string }[] {
		const choices = option.choices.map((choice) => ({
			value: choice.value,
			label: t(
				`optionsProfiles.fields.${option.key}_${choice.value.replace(
					/[^a-z0-9]/gi,
					"",
				)}`,
			),
		}));
		const current = rawFor(option);
		if (!choices.some((choice) => choice.value === current)) {
			choices.unshift({ value: current, label: current });
		}
		return choices;
	}

	function rangeFill(
		option: Extract<OptionDefinition, { kind: "range" }>,
	): string {
		const value = rangeDisplayValue(option, values.get(option.key));
		const percent =
			((value - option.min) / (option.max - option.min)) * 100;
		return `${Math.min(100, Math.max(0, percent))}%`;
	}
</script>

{#snippet resetButton(option: OptionDefinition)}
	{#if canReset(option)}
		<button
			type="button"
			class="reset-btn"
			{disabled}
			title={t("optionsProfiles.editor.resetField", {
				name: labelFor(option),
			})}
			aria-label={t("optionsProfiles.editor.resetField", {
				name: labelFor(option),
			})}
			onclick={() => reset(option)}
		>
			<Icon name="ui:refresh" size={13} />
		</button>
	{/if}
{/snippet}

<div class="visual-editor">
	<div class="search-box">
		<Icon name="ui:search" size={15} />
		<input
			type="text"
			bind:value={query}
			placeholder={t("optionsProfiles.editor.search")}
			aria-label={t("optionsProfiles.editor.search")}
			spellcheck="false"
		/>
		{#if query}
			<button
				type="button"
				class="search-clear"
				title={t("optionsProfiles.editor.clearSearch")}
				aria-label={t("optionsProfiles.editor.clearSearch")}
				onclick={() => (query = "")}
			>
				<Icon name="ui:close" size={13} />
			</button>
		{/if}
	</div>

	{#if advancedCount > 0}
		<p class="notice">
			{t("optionsProfiles.editor.preserved", { count: advancedCount })}
		</p>
	{/if}

	{#each filteredCategories as { category, options } (category)}
		<section class="category" aria-labelledby="options-cat-{category}">
			<header class="category-head">
				<h3 id="options-cat-{category}">
					{t(`optionsProfiles.categories.${category}`)}
				</h3>
				<span class="category-count">{options.length}</span>
			</header>
			<div class="fields">
				{#each options as option (option.key)}
					{#if option.kind === "toggle"}
						<div class="field toggle-field">
							<div class="toggle-row">
								<label
									class="toggle-label"
									for="opt-{option.key}"
									>{labelFor(option)}</label
								>
								<div class="field-tools">
									{#if !isPresent(option)}
										<span class="default-pill"
											>{t(
												"optionsProfiles.editor.default",
											)}</span
										>
									{/if}
									{@render resetButton(option)}
									<input
										class="switch"
										type="checkbox"
										id="opt-{option.key}"
										checked={rawFor(option) === "true"}
										{disabled}
										onchange={(event) =>
											toggle(
												option,
												event.currentTarget.checked,
											)}
									/>
								</div>
							</div>
						</div>
					{:else}
						<div class="field">
							<div class="field-head">
								<label
									class="field-label"
									for="opt-{option.key}"
									>{labelFor(option)}</label
								>
								<div class="field-tools">
									{#if !isPresent(option)}
										<span class="default-pill"
											>{t(
												"optionsProfiles.editor.default",
											)}</span
										>
									{/if}
									{@render resetButton(option)}
									{#if option.kind === "range"}
										<output
											class="value-pill"
											for="opt-{option.key}"
										>
											{formatRangeValue(
												option,
												rangeDisplayValue(
													option,
													values.get(option.key),
												),
											)}
										</output>
									{/if}
								</div>
							</div>
							{#if option.kind === "range"}
								<input
									id="opt-{option.key}"
									class="qm-range-input"
									style:--range-fill={rangeFill(option)}
									type="range"
									min={option.min}
									max={option.max}
									step={option.step}
									value={rangeDisplayValue(
										option,
										values.get(option.key),
									)}
									{disabled}
									oninput={(event) =>
										update(
											option,
											rangeRawValue(
												option,
												event.currentTarget
													.valueAsNumber,
											),
										)}
								/>
							{:else}
								<Select
									id="opt-{option.key}"
									value={rawFor(option)}
									options={choicesFor(option)}
									{disabled}
									onchange={(value) => update(option, value)}
								/>
							{/if}
						</div>
					{/if}
				{/each}
			</div>
		</section>
	{:else}
		<p class="empty">
			{t("optionsProfiles.editor.noResults", { query: query.trim() })}
		</p>
	{/each}
</div>

<style>
	.visual-editor {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}
	.search-box {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 36px;
		padding: 0 10px;
		background: var(--surface-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-muted);
		transition: border-color 0.15s ease;
	}
	.search-box:focus-within {
		border-color: var(--accent);
	}
	.search-box input {
		flex: 1;
		min-width: 0;
		height: 100%;
		background: transparent;
		border: none;
		outline: none;
		color: var(--text-primary);
		font: inherit;
		font-size: 0.8rem;
	}
	.search-box input::placeholder {
		color: var(--text-muted);
	}
	.search-clear {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 22px;
		height: 22px;
		padding: 0;
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--border-radius-sm);
		color: var(--text-muted);
		cursor: pointer;
	}
	.search-clear:hover {
		color: var(--text-primary);
		border-color: var(--border);
		background: var(--surface-hover);
	}
	.search-clear:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}
	.notice,
	.empty {
		margin: 0;
		padding: 8px 10px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-input);
		color: var(--text-muted);
		font-size: var(--font-size-label, 0.72rem);
		line-height: 1.5;
	}
	.category {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 12px 14px 14px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-input);
	}
	.category-head {
		display: flex;
		align-items: center;
		gap: 8px;
		padding-bottom: 8px;
		border-bottom: 1px solid var(--border);
	}
	.category-head::before {
		content: "";
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--accent);
	}
	.category h3 {
		margin: 0;
		font-size: var(--font-size-label, 0.72rem);
		font-weight: 700;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--accent);
	}
	.category-count {
		margin-left: auto;
		font-size: var(--font-size-label, 0.68rem);
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}
	.fields {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
		gap: 14px 20px;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 8px;
		min-width: 0;
	}
	.field-head,
	.toggle-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		min-height: 22px;
	}
	.field-label,
	.toggle-label {
		font-size: 0.8rem;
		color: var(--text-secondary);
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.toggle-label {
		cursor: pointer;
		transition: color 0.15s ease;
	}
	.toggle-row:hover .toggle-label {
		color: var(--text-primary);
	}
	.field-tools {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-shrink: 0;
	}
	.default-pill {
		padding: 1px 7px;
		border: 1px dashed var(--border);
		border-radius: 999px;
		color: var(--text-muted);
		font-size: var(--font-size-label, 0.68rem);
		white-space: nowrap;
	}
	.value-pill {
		min-width: 40px;
		padding: 2px 8px;
		border: 1px solid var(--border);
		border-radius: 999px;
		background: var(--surface-selected);
		color: var(--text-primary);
		font-size: 0.75rem;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
		text-align: center;
	}
	.reset-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 22px;
		height: 22px;
		padding: 0;
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--border-radius-sm);
		color: var(--text-muted);
		cursor: pointer;
	}
	.reset-btn:hover:not(:disabled) {
		color: var(--text-primary);
		border-color: var(--border);
		background: var(--surface-hover);
	}
	.reset-btn:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}
	.switch {
		appearance: none;
		-webkit-appearance: none;
		position: relative;
		flex-shrink: 0;
		width: 40px;
		height: 22px;
		margin: 0;
		box-sizing: border-box;
		border: 1px solid var(--border-color);
		border-radius: 999px;
		background: var(--bg-input);
		cursor: pointer;
		transition:
			background 0.18s ease,
			border-color 0.18s ease;
	}
	.switch::after {
		content: "";
		position: absolute;
		top: 2px;
		left: 2px;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--text-muted);
		transition:
			transform 0.18s ease,
			background 0.18s ease;
	}
	.switch:checked {
		background: var(--accent);
		border-color: var(--accent);
	}
	.switch:checked::after {
		transform: translateX(18px);
		background: var(--accent-text);
	}
	.switch:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.switch:disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}
</style>
