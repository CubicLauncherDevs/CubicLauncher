<script lang="ts">
	import { t } from "$lib/i18n";

	let {
		addingOffline = $bindable(false),
		offlineName = $bindable(""),
		onAddOffline,
		onOpenAuth,
		onOpenYggdrasil,
		showYggdrasilModal = false,
	}: {
		addingOffline?: boolean;
		offlineName?: string;
		onAddOffline: () => void;
		onOpenAuth: () => void;
		onOpenYggdrasil: () => void;
		showYggdrasilModal?: boolean;
	} = $props();

	function handleCancel() {
		addingOffline = false;
		offlineName = "";
	}
</script>

<div class="card add-card">
	<div class="add-toggle">
		<button
			type="button"
			class="add-toggle-btn"
			class:active={addingOffline}
			onclick={() => (addingOffline = true)}
		>
			{t("userMenu.addOffline")}
		</button>
		<button
			type="button"
			class="add-toggle-btn"
			class:active={!addingOffline && !showYggdrasilModal}
			onclick={onOpenAuth}
		>
			{t("userMenu.loginMicrosoft")}
		</button>
		<button
			type="button"
			class="add-toggle-btn ygg"
			onclick={onOpenYggdrasil}
		>
			{t("userMenu.authInjector")}
		</button>
	</div>
	{#if addingOffline}
		<div class="add-form">
			<input
				type="text"
				bind:value={offlineName}
				placeholder={t("userMenu.usernamePlaceholder")}
				maxlength="16"
				class="env-input"
				onkeydown={(e) => e.key === "Enter" && onAddOffline()}
			/>
			<div class="add-form-actions">
				<button
					type="button"
					class="btn-primary"
					onclick={onAddOffline}
				>
					{t("userMenu.add")}
				</button>
				<button
					type="button"
					class="btn-secondary"
					onclick={handleCancel}
				>
					{t("userMenu.cancel")}
				</button>
			</div>
		</div>
	{/if}
</div>

<style>
	.card {
		background: var(--bg-card);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 var(--surface-selected);
		overflow: hidden;
	}

	.add-card {
		display: flex;
		flex-direction: column;
	}

	.add-toggle {
		display: flex;
		gap: 0;
	}

	.add-toggle-btn {
		flex: 1;
		padding: 10px;
		font-size: 0.78rem;
		font-weight: 600;
		cursor: pointer;
		text-align: center;
		font-family: inherit;
		background: var(--bg-input);
		border: none;
		color: var(--text-secondary);
		transition: all 0.15s;
	}

	.add-toggle-btn:first-child {
		border-right: 1px solid var(--border-color);
	}

	.add-toggle-btn:last-child {
		border-left: 1px solid var(--border-color);
	}

	.add-toggle-btn.active {
		background: var(--accent);
		color: var(--accent-text);
	}

	.add-toggle-btn:hover:not(.active) {
		background: var(--surface-selected);
		color: var(--text-primary);
	}

	.add-form {
		display: flex;
		gap: 8px;
		padding: 12px 14px;
		border-top: 1px solid var(--border-color);
		align-items: center;
	}

	.add-form-actions {
		display: flex;
		gap: 6px;
		flex-shrink: 0;
	}

	.env-input {
		flex: 1;
		min-width: 0;
		width: 0;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 4px 8px;
		border-radius: var(--border-radius-sm);
		font-size: 0.8rem;
		height: 28px;
		box-sizing: border-box;
	}

	.env-input:focus {
		outline: none;
		border-color: var(--text-muted);
	}

	.btn-primary {
		background: var(--accent);
		color: var(--accent-text);
		border: none;
		cursor: pointer;
		padding: 5px 14px;
		border-radius: var(--border-radius-sm);
		font-size: 0.75rem;
		font-weight: 600;
		transition: opacity 0.15s;
	}

	.btn-primary:hover {
		opacity: 0.85;
	}

	.btn-secondary {
		background: transparent;
		border: 1px solid var(--border-color);
		color: var(--text-secondary);
		cursor: pointer;
		padding: 5px 14px;
		border-radius: var(--border-radius-sm);
		font-size: 0.75rem;
		font-weight: 600;
		transition: all 0.15s;
	}

	.btn-secondary:hover {
		background: var(--surface-selected);
		color: var(--text-primary);
	}
</style>
