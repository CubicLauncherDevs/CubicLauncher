<script lang="ts">
	import { fly } from "svelte/transition";
	import { t } from "$lib/i18n";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";
	import { animDuration } from "$lib/utils/animations";
	import type { CatalogTag } from "$lib/api/mosaicarySkins";

	interface Props {
		/** Las etiquetas que se ofrecen, las más usadas primero. */
		tags: CatalogTag[];
		/** Las elegidas. */
		chosen: string[];
		/** Cuántas se pueden elegir a la vez. */
		most: number;
		/** Si las skins tienen que llevar todas las elegidas, no solo alguna. */
		all: boolean;
		onToggle: (tag: string) => void;
		onClear: () => void;
		onAll: (all: boolean) => void;
	}

	let { tags, chosen, most, all, onToggle, onClear, onAll }: Props = $props();

	let isOpen = $state(false);
	let query = $state("");
	let trigger: HTMLButtonElement;
	let panel = $state<HTMLDivElement>();
	let place = $state("");
	const flyDuration = $derived(animDuration(150));

	// La lista se lleva al body, como la de `Select`: dentro de la ventana quedaría
	// recortada por su área con desplazamiento.
	function portal(el: HTMLElement) {
		document.body.appendChild(el);
		return {
			destroy() {
				el.remove();
			},
		};
	}

	function position() {
		const box = trigger.getBoundingClientRect();
		const below = window.innerHeight - box.bottom;
		const height = Math.min(320, Math.max(below, box.top) - 16);
		const top = below >= 200 ? box.bottom + 4 : box.top - 4 - height;
		place = `top:${top}px;left:${box.left}px;width:${Math.max(box.width, 240)}px;max-height:${height}px`;
	}

	function toggleOpen() {
		if (isOpen) return close();
		position();
		query = "";
		isOpen = true;
	}

	function close() {
		isOpen = false;
	}

	function onOutside(event: PointerEvent) {
		const target = event.target as Node;
		if (!trigger.contains(target) && !panel?.contains(target)) close();
	}

	// La lista está fija en la pantalla: si lo que hay debajo se desplaza, se cierra en vez
	// de quedarse flotando lejos de su campo. Su propio desplazamiento no cuenta.
	function onScroll(event: Event) {
		if (!panel?.contains(event.target as Node)) close();
	}

	$effect(() => {
		if (!isOpen) return;
		document.addEventListener("pointerdown", onOutside, true);
		document.addEventListener("scroll", onScroll, true);
		window.addEventListener("resize", close);
		return () => {
			document.removeEventListener("pointerdown", onOutside, true);
			document.removeEventListener("scroll", onScroll, true);
			window.removeEventListener("resize", close);
		};
	});

	const shown = $derived.by(() => {
		const words = query.trim().toLowerCase();
		return words
			? tags.filter((one) => one.tag.toLowerCase().includes(words))
			: tags;
	});

	const summary = $derived(
		chosen.length === 0
			? t("userMenu.skinCape.catalog.tagsNone")
			: chosen.length <= 2
				? chosen.join(", ")
				: t("userMenu.skinCape.catalog.tagsChosen", {
						count: chosen.length,
					}),
	);
</script>

<button
	type="button"
	class="tag-trigger"
	class:open={isOpen}
	bind:this={trigger}
	onclick={toggleOpen}
	onkeydown={(e) => e.key === "Escape" && close()}
	aria-haspopup="listbox"
	aria-expanded={isOpen}
	title={chosen.join(", ")}
>
	<span class="tag-summary" class:muted={chosen.length === 0}>{summary}</span>
	<ChevronDownIcon size={16} />
</button>

{#if isOpen}
	<div use:portal>
		<div
			class="tag-panel"
			bind:this={panel}
			style={place}
			role="listbox"
			aria-multiselectable="true"
			tabindex="-1"
			onkeydown={(e) => e.key === "Escape" && close()}
			transition:fly={{ y: -6, duration: flyDuration }}
		>
			<div class="tag-panel-head">
				<input
					type="search"
					class="text-input tag-search"
					placeholder={t("userMenu.skinCape.catalog.tagsSearch")}
					bind:value={query}
				/>
				{#if chosen.length}
					<button type="button" class="tag-clear" onclick={onClear}
						>{t("userMenu.skinCape.catalog.tagsClear")}</button
					>
				{/if}
			</div>
			{#if chosen.length > 1}
				<label class="tag-all">
					<input
						type="checkbox"
						checked={all}
						onchange={(e) => onAll(e.currentTarget.checked)}
					/>
					{t("userMenu.skinCape.catalog.tagsAll")}
				</label>
			{/if}
			<div class="tag-options">
				{#each shown as one (one.tag)}
					{@const on = chosen.includes(one.tag)}
					<button
						type="button"
						class="tag-option"
						class:selected={on}
						role="option"
						aria-selected={on}
						disabled={!on && chosen.length >= most}
						onclick={() => onToggle(one.tag)}
					>
						<span class="tag-check">
							{#if on}<CheckIcon size={12} />{/if}
						</span>
						<span class="tag-name">{one.tag}</span>
						<span class="tag-count">{one.count}</span>
					</button>
				{:else}
					<p class="tag-empty">
						{t("userMenu.skinCape.catalog.tagsNoMatch")}
					</p>
				{/each}
			</div>
		</div>
	</div>
{/if}

<style>
	.tag-trigger {
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		min-height: var(--control-min-height, 38px);
		padding: var(--control-compact-padding, 8px 12px);
		font-family: inherit;
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-medium);
		color: var(--text-primary);
		border: var(--border-width) solid var(--border);
		border-radius: var(--border-radius);
		background: var(--bg-card-gradient), var(--surface-input);
		box-shadow: var(--shadow-sm);
		cursor: pointer;
		text-align: start;
	}

	.tag-trigger:hover,
	.tag-trigger.open {
		border-color: var(--accent);
		background: var(--surface-hover);
	}

	.tag-trigger.open {
		box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 15%, transparent);
	}

	.tag-summary {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.tag-summary.muted {
		color: var(--text-secondary);
	}

	.tag-panel {
		position: fixed;
		z-index: 2000;
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		padding: 6px;
		background: var(--surface-dropdown, var(--bg-sidebar));
		border: var(--border-width) solid
			color-mix(in srgb, var(--accent) 25%, var(--border));
		border-radius: var(--border-radius);
		box-shadow: var(--shadow-lg, 0 12px 32px rgba(0, 0, 0, 0.3));
	}

	.tag-panel-head {
		display: flex;
		gap: 6px;
		padding-bottom: 6px;
	}

	.tag-search {
		flex: 1;
		min-width: 0;
	}

	.tag-clear {
		background: none;
		border: none;
		color: var(--accent);
		font-family: inherit;
		font-size: 0.72rem;
		cursor: pointer;
		white-space: nowrap;
	}

	.tag-all {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 2px 8px 8px;
		font-size: 0.75rem;
		color: var(--text-secondary);
		cursor: pointer;
	}

	.tag-all input {
		accent-color: var(--accent);
	}

	.tag-options {
		overflow-y: auto;
		overscroll-behavior: contain;
		min-height: 0;
	}

	.tag-option {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 32px;
		padding: 6px 8px;
		background: none;
		border: var(--border-width) solid transparent;
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font-family: inherit;
		font-size: var(--font-size-sm);
		cursor: pointer;
		text-align: start;
	}

	.tag-option:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.tag-option.selected {
		background: color-mix(in srgb, var(--accent) 12%, transparent);
		border-color: color-mix(in srgb, var(--accent) 25%, transparent);
		color: var(--text-primary);
	}

	.tag-option:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.tag-check {
		width: 14px;
		height: 14px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border: 1px solid var(--border);
		border-radius: 3px;
		color: var(--accent);
		flex-shrink: 0;
	}

	.tag-name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.tag-count {
		font-size: 0.7rem;
		color: var(--text-muted);
	}

	.tag-empty {
		margin: 0;
		padding: 10px 8px;
		font-size: 0.75rem;
		color: var(--text-muted);
	}
</style>
