<script lang="ts">
	import { INSTANCE_LOGOS } from "$lib/icons/logos";
	import { t } from "$lib/i18n";

	let {
		selectedIcon = $bindable<string | null>(null),
		disabled = false,
	}: {
		selectedIcon: string | null;
		disabled?: boolean;
	} = $props();
</script>

<div class="icon-column">
	<span class="input-label">{t("createInstance.iconLabel")}</span>
	<div class="icon-preview">
		{#if selectedIcon}
			<img src={selectedIcon} alt="Logo" />
			<button
				type="button"
				class="icon-clear"
				onclick={() => (selectedIcon = null)}
				title="Quitar icono"
				{disabled}
			>
				<svg
					width="10"
					height="10"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="3"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<line x1="18" y1="6" x2="6" y2="18"></line>
					<line x1="6" y1="6" x2="18" y2="18"></line>
				</svg>
			</button>
		{:else}
			<svg
				width="28"
				height="28"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="1.5"
				stroke-linecap="round"
				stroke-linejoin="round"
				style="color: var(--text-secondary); opacity: 0.4;"
			>
				<rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
				<circle cx="8.5" cy="8.5" r="1.5"></circle>
				<polyline points="21 15 16 10 5 21"></polyline>
			</svg>
		{/if}
	</div>
	<div class="icon-grid">
		{#each INSTANCE_LOGOS as iconName (iconName)}
			{@const iconPath = `/images/instances/${iconName}`}
			<button
				type="button"
				class="icon-option"
				class:selected={selectedIcon === iconPath}
				onclick={() =>
					(selectedIcon =
						selectedIcon === iconPath ? null : iconPath)}
				title={iconName}
				{disabled}
			>
				<img src={iconPath} alt={iconName} />
			</button>
		{/each}
	</div>
</div>

<style>
	.icon-column {
		display: flex;
		flex-direction: column;
		gap: 10px;
		align-items: center;
		flex-shrink: 0;
	}

	.icon-preview {
		width: 80px;
		height: 80px;
		border-radius: 12px;
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.03);
		border: 2px dashed var(--border);
		display: flex;
		align-items: center;
		justify-content: center;
		position: relative;
		overflow: visible;
	}

	.icon-preview img {
		width: 56px;
		height: 56px;
		object-fit: contain;
		filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.3));
	}

	.icon-clear {
		position: absolute;
		top: -5px;
		right: -5px;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		background: var(--color-error);
		color: white;
		border: 2px solid var(--bg-card);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		box-shadow: var(--shadow-sm);
		transition:
			transform 0.15s,
			background 0.15s;
		opacity: 0;
	}

	.icon-preview:hover .icon-clear {
		opacity: 1;
	}

	.icon-clear:hover {
		transform: scale(1.15);
		filter: brightness(0.8);
	}

	.icon-clear:active {
		transform: scale(0.95);
	}

	.icon-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 6px;
	}

	.icon-option {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.03);
		border: 2px solid var(--border);
		cursor: pointer;
		padding: 6px;
		transition:
			border-color 0.15s,
			background 0.15s;
	}

	.icon-option:hover {
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.06);
		border-color: var(--text-secondary);
	}

	.icon-option.selected {
		border-color: var(--accent);
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.1);
	}

	.icon-option img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}

	.input-label {
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 5px;
		display: block;
	}
</style>
