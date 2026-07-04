<script lang="ts">
	let {
		username,
		avatarSvg,
		isPremium,
		userTypeLabel,
		onclick,
	}: {
		username: string;
		avatarSvg: string;
		isPremium: boolean;
		userTypeLabel: string;
		onclick: () => void;
	} = $props();
</script>

<div
	class="user-profile"
	onclick={onclick}
	role="button"
	tabindex="0"
	onkeydown={(e) =>
		(e.key === "Enter" || e.key === " ") && onclick()}
	style="cursor: pointer;"
>
	<div class="user-avatar-wrapper">
		{#if avatarSvg}
			{@html avatarSvg}
		{/if}
	</div>
	<div class="user-info">
		<div class="user-name-wrapper">
			<span class="user-name">{username}</span>
		</div>
		<span class="user-status" class:premium={isPremium}>
			{userTypeLabel}
		</span>
	</div>
</div>

<style>
	.user-avatar-wrapper {
		width: 28px;
		height: 28px;
		border-radius: var(--border-radius-sm);
		border: 1px solid var(--border);
		flex-shrink: 0;
		background: url("/images/cubic.svg") center/60% no-repeat;
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.user-avatar-wrapper :global(svg) {
		width: 100%;
		height: 100%;
		display: block;
		border-radius: inherit;
	}

	.user-profile {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px;
		margin-top: auto;
		background: var(--bg-item-active);
	}

	.user-info {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		gap: 2px;
	}

	.user-name {
		font-size: 0.82rem;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.user-name-wrapper {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.user-status {
		font-size: 0.68rem;
		color: var(--text-secondary);
		letter-spacing: 0.3px;
		transition: color 0.2s ease;
	}

	.user-status.premium {
		color: var(--accent);
		font-weight: 600;
	}

	@media (max-width: 650px) {
		.user-profile {
			justify-content: center;
		}

		.user-info {
			display: none;
		}
	}
</style>