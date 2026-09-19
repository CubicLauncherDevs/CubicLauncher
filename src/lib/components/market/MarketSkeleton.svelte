<div class="market-skeleton" aria-hidden="true">
	<div class="skeleton-header">
		<span class="skeleton-icon"></span>
		<div class="skeleton-identity">
			<span class="skeleton-line title"></span>
			<span class="skeleton-line author"></span>
		</div>
	</div>
	<div class="skeleton-description">
		<span class="skeleton-line"></span>
		<span class="skeleton-line short"></span>
	</div>
	<div class="skeleton-footer">
		<span class="skeleton-line author"></span>
		<span class="skeleton-button"></span>
	</div>
</div>

<style>
	.market-skeleton {
		display: flex;
		flex-direction: column;
		gap: var(--space-md);
		height: 100%;
		box-sizing: border-box;
		padding: var(--market-card-padding);
		border: var(--border-width) solid var(--border);
		border-radius: var(--market-card-radius);
		background: var(--bg-card-gradient), var(--surface-card);
	}
	.skeleton-header,
	.skeleton-footer {
		display: flex;
		align-items: center;
		gap: var(--market-item-gap);
	}
	.skeleton-icon,
	.skeleton-line,
	.skeleton-button {
		display: block;
		background: color-mix(in srgb, var(--text-secondary) 18%, transparent);
		border-radius: var(--border-radius-sm);
	}
	.skeleton-icon {
		width: var(--market-icon-size);
		height: var(--market-icon-size);
		flex-shrink: 0;
	}
	.skeleton-identity,
	.skeleton-description {
		display: flex;
		flex-direction: column;
		gap: 8px;
		flex: 1;
	}
	.skeleton-line {
		height: 9px;
		width: 100%;
	}
	.skeleton-line.title {
		height: 13px;
		width: 80%;
	}
	.skeleton-line.author {
		width: 40%;
	}
	.skeleton-line.short {
		width: 70%;
	}
	.skeleton-footer {
		justify-content: space-between;
		padding-top: 10px;
		border-top: var(--border-width) solid var(--border);
	}
	.skeleton-button {
		width: 64px;
		height: 28px;
	}
</style>
