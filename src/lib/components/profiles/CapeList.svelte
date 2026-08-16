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
			<div class="cape-grid">
				{#each inactiveCapes as cape (cape.id)}
					<div class="cape-card">
						{#if cape.url}
							<div class="cape-thumb">
								<img src={cape.url} alt={cape.alias} />
							</div>
						{:else}
							<div class="cape-thumb cape-thumb-fallback"></div>
						{/if}
						<div class="cape-info">
							<span class="cape-name">
								{cape.alias || t("userMenu.skinCape.cape")}
							</span>
						</div>
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
		padding-top: 8px;
		border-top: 1px solid var(--border);
	}

	.subsection-title {
		margin: 0;
		font-size: 0.75rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.cape-count {
		background: var(--surface-selected);
		color: var(--text-secondary);
		padding: 1px 6px;
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
		gap: 12px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-left: 3px solid var(--accent);
		border-radius: var(--border-radius);
		padding: 12px;
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 var(--surface-selected);
	}

	.active-cape-img {
		width: 40px;
		height: 64px;
		position: relative;
		overflow: hidden;
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
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
		gap: 2px;
		min-width: 0;
		flex: 1;
	}

	.active-cape-name {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.active-cape-status {
		font-size: 0.62rem;
		font-weight: 700;
		color: var(--accent);
		text-transform: uppercase;
		letter-spacing: 0.3px;
	}

	.unequip-btn {
		flex-shrink: 0;
		font-size: 0.75rem;
		padding: 6px 12px;
	}

	.pending-unequip {
		justify-content: center;
		border-left: 3px solid var(--text-muted);
		opacity: 0.75;
	}

	.cape-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 10px;
	}

	.cape-card {
		display: flex;
		flex-direction: column;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		overflow: hidden;
		transition: background 0.15s ease;
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 var(--surface-selected);
	}

	.cape-card:hover {
		background: var(--surface-selected);
	}

	.cape-thumb {
		width: 40px;
		height: 64px;
		position: relative;
		overflow: hidden;
		margin: 6px auto 0;
		background: var(--bg-input);
		border-radius: var(--border-radius-sm);
		border: 1px solid var(--border);
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

	.cape-info {
		padding: 10px;
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.cape-name {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.cape-action {
		margin: 0 10px 10px;
		font-size: 0.75rem;
		padding: 6px 10px;
	}

	.btn-primary,
	.btn-secondary {
		font-family: inherit;
		font-weight: 600;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition: all 0.15s ease;
		border: 1px solid transparent;
		white-space: nowrap;
	}

	.btn-primary {
		background: var(--accent);
		color: var(--accent-text);
	}

	.btn-primary:hover:not(:disabled) {
		opacity: 0.85;
	}

	.btn-secondary {
		background: transparent;
		border-color: var(--border);
		color: var(--text-secondary);
	}

	.btn-secondary:hover:not(:disabled) {
		background: var(--surface-selected);
		color: var(--text-primary);
	}

	.btn-primary:disabled,
	.btn-secondary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	@media (max-width: 520px) {
		.cape-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
