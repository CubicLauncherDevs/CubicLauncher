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
			onclick={() => handleClick(loader.value)}
		>
			<img src={loader.icon} alt={loader.label} />
			<span>{loader.label}</span>
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
		--btn-bg: rgba(var(--accent-rgb, 255, 255, 255), 0.03);
		flex: none;
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 4px;
		padding: 10px 4px;
		background: var(--btn-bg);
		border: 1px solid var(--border);
		border-top: none;
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
		border-top: 1px solid var(--border);
		border-radius: var(--border-radius-sm) var(--border-radius-sm) 0 0;
	}

	.loader-btn:last-child {
		border-radius: 0 0 var(--border-radius-sm) var(--border-radius-sm);
	}

	.loader-btn:hover {
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.06);
		color: var(--text-primary);
		z-index: 1;
	}

	.loader-btn.active {
		background: rgba(var(--accent-rgb, 255, 255, 255), 0.1);
		border-color: var(--accent);
		color: var(--text-primary);
		z-index: 2;
	}

	.loader-btn img {
		width: 22px;
		height: 22px;
		object-fit: contain;
	}

	.loader-btn span {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 100%;
	}

	@media (max-width: 500px) {
		.loader-btn span {
			display: none;
		}
	}
</style>
