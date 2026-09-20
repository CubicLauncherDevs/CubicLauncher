<script lang="ts">
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import {
		ACCOUNT_PROVIDERS,
		type AccountProvider,
	} from "$lib/utils/accountProviders";

	let {
		onselect,
	}: {
		onselect: (
			provider: AccountProvider,
			trigger: HTMLButtonElement,
		) => void;
	} = $props();
</script>

<section class="add-card" aria-labelledby="add-account-title">
	<header class="add-header">
		<h3 id="add-account-title">{t("userMenu.accountSetup.title")}</h3>
		<p>{t("userMenu.accountSetup.subtitle")}</p>
	</header>
	<div class="provider-grid">
		{#each ACCOUNT_PROVIDERS as provider (provider)}
			<button
				type="button"
				class="provider-option"
				class:custom={provider === "authinject"}
				data-provider={provider}
				onclick={(event) => onselect(provider, event.currentTarget)}
			>
				<span class="provider-icon" aria-hidden="true">
					{#if provider === "premium"}
						<img
							src="/images/icons/brand/microsoft.svg"
							alt=""
							width="22"
							height="22"
						/>
					{:else if provider === "cubicAuth"}
						<Icon src="/images/cubic.svg" size={24} />
					{:else if provider === "ely"}
						<span class="ely-mark">E</span>
					{:else if provider === "offline"}
						<svg
							width="22"
							height="22"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="1.6"
							stroke-linecap="round"
						>
							<circle cx="12" cy="8" r="4" />
							<path d="M4 21v-2a8 8 0 0 1 16 0v2" />
						</svg>
					{:else}
						<Icon name="instance:servers" size={22} />
					{/if}
				</span>
				<span class="provider-name"
					>{t(`userMenu.accountSetup.${provider}`)}</span
				>
				<span class="provider-description"
					>{t(`userMenu.accountSetup.descriptions.${provider}`)}</span
				>
			</button>
		{/each}
	</div>
</section>

<style>
	.add-card {
		container: account-picker / inline-size;
		flex-shrink: 0;
		min-width: 0;
		padding: 16px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
	}

	.add-header {
		margin-bottom: 14px;
	}

	.add-header h3 {
		margin: 0 0 5px;
		font-size: 0.9rem;
		font-weight: 700;
		color: var(--text-primary);
	}

	.add-header p {
		margin: 0;
		font-size: 0.75rem;
		line-height: 1.5;
		color: var(--text-secondary);
	}

	.provider-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 8px;
	}

	.provider-option {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 6px;
		min-width: 0;
		padding: 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		color: var(--text-primary);
		font: inherit;
		text-align: left;
		cursor: pointer;
		transition:
			background-color 0.15s,
			border-color 0.15s;
	}

	.provider-option:hover {
		background: var(--surface-hover);
		border-color: var(--accent);
	}

	.provider-option:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.provider-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border-radius: var(--border-radius-sm);
		background: var(--surface-selected);
		color: var(--accent);
		flex-shrink: 0;
	}

	.ely-mark {
		font-size: 1.25rem;
		font-weight: 800;
	}

	.provider-name {
		font-size: 0.8rem;
		font-weight: 650;
		line-height: 1.4;
		overflow-wrap: anywhere;
	}

	.provider-description {
		font-size: 0.7rem;
		line-height: 1.5;
		color: var(--text-secondary);
		overflow-wrap: anywhere;
	}

	.provider-option.custom {
		grid-column: 1 / -1;
		display: grid;
		grid-template-columns: 32px minmax(0, 1fr);
		column-gap: 10px;
		row-gap: 2px;
	}

	.custom .provider-icon {
		grid-row: span 2;
		align-self: center;
	}

	@container account-picker (max-width: 280px) {
		.provider-grid {
			grid-template-columns: minmax(0, 1fr);
		}

		.provider-option {
			display: grid;
			grid-template-columns: 32px minmax(0, 1fr);
			column-gap: 10px;
			row-gap: 2px;
		}

		.provider-icon {
			grid-row: span 2;
			align-self: center;
		}
	}
</style>
