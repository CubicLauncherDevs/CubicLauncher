<script lang="ts">
	let {
		loaderTab = $bindable("vanilla"),
		LOADERS = [],
		onswitch,
	}: {
		loaderTab: string;
		LOADERS: Array<{ value: string; label: string; icon: string }>;
		onswitch: (tab: string) => void;
	} = $props();

	function handleClick(value: string) {
		loaderTab = value;
		onswitch(value);
	}
</script>

<div class="loader-unified">
	{#each LOADERS as loader (loader.value)}
		<button
			type="button"
			class="loader-btn"
			class:active={loaderTab === loader.value}
			title={loader.label}
			onclick={() => handleClick(loader.value)}
		>
			<span class="loader-icon-wrap">
				<img src={loader.icon} alt="" />
			</span>
			<span class="loader-label">{loader.label}</span>
		</button>
	{/each}
</div>

<style>
	.loader-unified {
		display: flex;
		flex-direction: column;
		width: 100%;
	}

	.loader-btn {
		flex: none;
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 6px;
		padding: 12px 6px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		margin-top: -1px;
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
		position: relative;
		z-index: 0;
		transition:
			background-color 0.15s,
			color 0.15s,
			border-color 0.15s,
			box-shadow 0.15s;
	}

	.loader-btn:first-child {
		margin-top: 0;
		border-radius: var(--border-radius-sm) var(--border-radius-sm) 0 0;
	}

	.loader-btn:last-child {
		border-radius: 0 0 var(--border-radius-sm) var(--border-radius-sm);
	}

	.loader-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
		z-index: 1;
	}

	.loader-btn.active {
		background: var(--surface-active);
		border-color: var(--accent);
		color: var(--text-primary);
		z-index: 2;
	}

	.loader-icon-wrap {
		width: 34px;
		height: 34px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--surface-input);
		border-radius: var(--border-radius-sm);
		padding: 4px;
		flex-shrink: 0;
		transition: background-color 0.15s;
	}

	.loader-btn:hover .loader-icon-wrap,
	.loader-btn.active .loader-icon-wrap {
		background: var(--bg-main);
	}

	.loader-btn img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		display: block;
	}

	.loader-label {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 100%;
	}

	@media (max-width: 500px) {
		.loader-btn {
			padding: 10px 4px;
			gap: 0;
		}

		.loader-label {
			display: none;
		}

		.loader-icon-wrap {
			width: 28px;
			height: 28px;
		}
	}
</style>
