<script lang="ts">
	import { t } from "$lib/i18n";
	import type { MinecraftProfileCape } from "$lib/types/types";

	interface Props {
		capes: MinecraftProfileCape[];
		activeCapeId: string | null;
		showUnequipPending: boolean;
		processing: boolean;
		onEquip: (capeId: string) => void;
		onUnequip: () => void;
	}

	let {
		capes,
		activeCapeId,
		showUnequipPending,
		processing,
		onEquip,
		onUnequip,
	}: Props = $props();

	const activeCape = $derived(
		capes.find((c: MinecraftProfileCape) => c.id === activeCapeId) ?? null,
	);
	const inactiveCapes = $derived(
		capes.filter((c: MinecraftProfileCape) => c.id !== activeCapeId),
	);
</script>

<div class="capes-section">
	<h5 class="subsection-title">
		{t("userMenu.skinCape.capes")}
		{#if capes.length > 0}
			<span class="cape-count">{capes.length}</span>
		{/if}
	</h5>

	{#if capes.length === 0}
		<p class="empty-text">{t("userMenu.skinCape.noCapes")}</p>
	{:else}
		{#if activeCape}
			<div class="active-cape-card">
				{#if activeCape.url}
					<div class="active-cape-img">
						<img src={activeCape.url} alt={activeCape.alias} />
					</div>
				{:else}
					<div class="active-cape-img active-cape-img-fallback"></div>
				{/if}
				<div class="active-cape-meta">
					<span class="active-cape-name">
						{activeCape.alias || t("userMenu.skinCape.cape")}
					</span>
					<span class="active-cape-status">
						{t("userMenu.skinCape.active")}
					</span>
				</div>
				<button
					type="button"
					class="btn-secondary unequip-btn"
					onclick={onUnequip}
					disabled={processing}
				>
					{t("userMenu.skinCape.unequip")}
				</button>
			</div>
		{:else if showUnequipPending}
			<div class="active-cape-card pending-unequip">
				<div class="active-cape-meta">
					<span class="active-cape-name">
						{t("userMenu.skinCape.unequipPending")}
					</span>
				</div>
			</div>
		{/if}

		{#if inactiveCapes.length > 0}
			<div class="cape-list">
				{#each inactiveCapes as cape (cape.id)}
					<div class="cape-row">
						{#if cape.url}
							<div class="cape-thumb">
								<img src={cape.url} alt={cape.alias} />
							</div>
						{:else}
							<div class="cape-thumb cape-thumb-fallback"></div>
						{/if}
						<span class="cape-name">
							{cape.alias || t("userMenu.skinCape.cape")}
						</span>
						<button
							type="button"
							class="btn-primary cape-action"
							onclick={() => onEquip(cape.id)}
							disabled={processing}
						>
							{t("userMenu.skinCape.equip")}
						</button>
					</div>
				{/each}
			</div>
		{/if}
	{/if}
</div>

<style>
	.capes-section {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 14px;
	}

	.subsection-title {
		margin: 0;
		font-size: 0.75rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.6px;
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.cape-count {
		background: var(--accent);
		color: var(--accent-text);
		padding: 2px 8px;
		border-radius: 999px;
		font-size: 0.65rem;
	}

	.empty-text {
		margin: 0;
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	.active-cape-card {
		display: flex;
		align-items: center;
		gap: 14px;
		background: var(--bg-input);
		border: 1px solid var(--accent);
		border-left: 4px solid var(--accent);
		border-radius: var(--border-radius);
		padding: 12px;
	}

	.active-cape-img {
		width: 48px;
		height: 80px;
		position: relative;
		overflow: hidden;
		border-radius: var(--border-radius-sm);
		background: var(--bg-card);
		border: 1px solid var(--border);
		flex-shrink: 0;
	}

	.active-cape-img img {
		position: absolute;
		left: -4px;
		top: -4px;
		width: 256px;
		height: 128px;
		max-width: none;
		image-rendering: pixelated;
	}

	.active-cape-img-fallback {
		background: var(--cubic-logo) center/30% no-repeat;
	}

	.active-cape-meta {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
		flex: 1;
	}

	.active-cape-name {
		font-size: 0.9rem;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.active-cape-status {
		font-size: 0.65rem;
		font-weight: 700;
		color: var(--accent);
		text-transform: uppercase;
		letter-spacing: 0.4px;
	}

	.pending-unequip {
		justify-content: center;
		border-left: 4px solid var(--text-muted);
		border-color: var(--border);
		opacity: 0.8;
	}

	.cape-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.cape-row {
		display: flex;
		align-items: center;
		gap: 12px;
		background: var(--bg-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		padding: 10px 12px;
		transition: background 0.15s ease;
	}

	.cape-row:hover {
		background: var(--surface-selected);
	}

	.cape-thumb {
		width: 40px;
		height: 68px;
		position: relative;
		overflow: hidden;
		background: var(--bg-card);
		border-radius: var(--border-radius-sm);
		border: 1px solid var(--border);
		flex-shrink: 0;
	}

	.cape-thumb img {
		position: absolute;
		left: -4px;
		top: -4px;
		width: 256px;
		height: 128px;
		max-width: none;
		image-rendering: pixelated;
	}

	.cape-thumb-fallback {
		background: var(--cubic-logo) center/30% no-repeat;
	}

	.cape-name {
		flex: 1;
		min-width: 0;
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.cape-action,
	.unequip-btn {
		font-family: inherit;
		font-size: 0.75rem;
		font-weight: 600;
		padding: 5px 12px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition:
			background 0.15s ease,
			color 0.15s ease,
			opacity 0.15s ease;
	}

	.cape-action:disabled,
	.unequip-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	@media (max-width: 520px) {
		.active-cape-card {
			gap: 10px;
			padding: 10px;
		}

		.active-cape-img {
			width: 38px;
			height: 64px;
		}

		.cape-row {
			gap: 10px;
			padding: 8px;
		}

		.cape-thumb {
			width: 34px;
			height: 56px;
		}
	}
</style>
