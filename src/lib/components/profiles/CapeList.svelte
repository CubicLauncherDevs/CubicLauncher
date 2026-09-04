<script lang="ts">
	import { t } from "$lib/i18n";
	import type { MinecraftProfileCape } from "$lib/types/types";
	import { fly } from "svelte/transition";

	interface Props {
		capes: MinecraftProfileCape[];
		selectedCapeId: string | null;
		activeCapeId: string | null;
		processing: boolean;
		onSelect: (capeId: string | null) => void;
	}

	let { capes, selectedCapeId, activeCapeId, processing, onSelect }: Props =
		$props();

	function handleClick(capeId: string | null) {
		if (processing) return;
		onSelect(capeId);
	}
</script>

{#if capes.length > 0}
	<div
		class="cape-strip"
		role="radiogroup"
		aria-label={t("userMenu.skinCape.capes")}
		in:fly={{ y: 8, duration: 200 }}
	>
		<span class="strip-label">{t("userMenu.skinCape.capes")}</span>

		<div class="strip-scroll">
			{#each capes as cape (cape.id)}
				<button
					type="button"
					class="cape-btn"
					class:selected={selectedCapeId === cape.id}
					class:active={activeCapeId === cape.id}
					onclick={() => handleClick(cape.id)}
					disabled={processing}
					aria-label={cape.alias || t("userMenu.skinCape.cape")}
					title={cape.alias || t("userMenu.skinCape.cape")}
				>
					{#if cape.url}
						<div class="cape-thumb">
							<img src={cape.url} alt="" />
						</div>
					{:else}
						<div class="cape-thumb cape-thumb-fallback"></div>
					{/if}
				</button>
			{/each}

			<button
				type="button"
				class="cape-btn unequip-btn"
				class:selected={selectedCapeId === null}
				class:active={activeCapeId === null}
				onclick={() => handleClick(null)}
				disabled={processing}
				aria-label={t("userMenu.skinCape.unequip")}
				title={t("userMenu.skinCape.unequip")}
			>
				<span class="unequip-label">{t("userMenu.skinCape.none")}</span>
			</button>
		</div>
	</div>
{/if}

<style>
	.cape-strip {
		display: flex;
		align-items: center;
		gap: 12px;
		width: 100%;
	}

	.strip-label {
		font-size: 0.7rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		white-space: nowrap;
		flex-shrink: 0;
	}

	.strip-scroll {
		display: flex;
		align-items: center;
		gap: 10px;
		overflow-x: auto;
		overflow-y: hidden;
		scrollbar-width: thin;
		padding-bottom: 4px;
	}

	.cape-btn {
		position: relative;
		flex-shrink: 0;
		width: 48px;
		height: 96px;
		padding: 3px;
		background: var(--bg-input);
		border: 2px solid var(--border);
		border-radius: var(--border-radius);
		cursor: pointer;
		transition:
			border-color 0.15s ease,
			background 0.15s ease,
			transform 0.1s ease;
		overflow: hidden;
	}

	.cape-btn:hover:not(:disabled, .selected) {
		background: var(--surface-hover);
	}

	.cape-btn.selected {
		border-color: var(--accent);
		background: rgba(var(--accent-rgb), 0.1);
		box-shadow: 0 0 0 2px rgba(var(--accent-rgb), 0.2);
	}

	.cape-btn.active::after {
		content: "";
		position: absolute;
		top: 5px;
		left: 5px;
		width: 7px;
		height: 7px;
		background: var(--accent);
		border-radius: 50%;
		box-shadow: 0 0 0 2px var(--bg-input);
	}

	.cape-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.cape-btn:active:not(:disabled) {
		transform: scale(0.96);
	}

	.cape-thumb {
		width: 100%;
		height: 100%;
		position: relative;
		overflow: hidden;
		background: var(--bg-card);
		border-radius: var(--border-radius-sm);
	}

	.cape-thumb img {
		position: absolute;
		left: -8px;
		top: -8px;
		width: 320px;
		height: 160px;
		max-width: none;
		image-rendering: pixelated;
	}

	.cape-thumb-fallback {
		background: var(--cubic-logo) center/30% no-repeat;
	}

	.unequip-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-secondary);
	}

	.unequip-label {
		font-size: 0.65rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.3px;
	}

	.unequip-btn:hover:not(:disabled, .selected) {
		color: var(--text-primary);
		background: var(--surface-hover);
	}

	@media (max-width: 520px) {
		.cape-btn {
			width: 40px;
			height: 80px;
		}

		.strip-label {
			font-size: 0.65rem;
		}
	}
</style>
