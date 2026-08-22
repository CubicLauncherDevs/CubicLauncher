<script lang="ts">
	interface Account {
		username: string;
		user_type: string;
	}

	interface Props {
		user: Account;
		typeLabel: string;
		isActive: boolean;
		isSelected: boolean;
		avatarSvg: string;
		onselect?: () => void;
	}

	let { user, typeLabel, isActive, isSelected, avatarSvg, onselect }: Props =
		$props();

	function handleClick() {
		onselect?.();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === "Enter" || e.key === " ") {
			onselect?.();
		}
	}
</script>

<div
	class="account-item"
	class:active={isActive}
	class:selected={isSelected}
	onclick={handleClick}
	role="button"
	tabindex="0"
	onkeydown={handleKeydown}
>
	<div class="account-avatar">
		{#if avatarSvg}
			{@html avatarSvg}
		{/if}
	</div>
	<div class="account-info">
		<span class="account-name">{user.username}</span>
		<span class="account-type">{typeLabel}</span>
	</div>
	{#if isActive}
		<span class="active-badge">Activo</span>
	{/if}
</div>

<style>
	.account-item {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 12px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition:
			border-color 0.15s ease,
			background-color 0.15s ease;
	}

	.account-item:hover {
		background: var(--surface-hover);
		border-color: var(--border);
	}

	.account-item.selected {
		background: var(--bg-item-active);
		border-color: var(--accent);
	}

	.account-avatar {
		width: 40px;
		height: 40px;
		border-radius: var(--border-radius-sm);
		border: 1px solid var(--border);
		flex-shrink: 0;
		background: var(--cubic-logo) center/60% no-repeat;
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.account-avatar :global(svg) {
		width: 100%;
		height: 100%;
		display: block;
		border-radius: inherit;
	}

	.account-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
		flex: 1;
	}

	.account-name {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.account-type {
		font-size: 0.65rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.active-badge {
		font-size: 0.6rem;
		background: var(--accent);
		color: var(--accent-text);
		padding: 3px 8px;
		border-radius: 999px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.3px;
		flex-shrink: 0;
	}
</style>
