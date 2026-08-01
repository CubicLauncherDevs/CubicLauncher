<script lang="ts">
	import { t } from "$lib/i18n";
	import CloseIcon from "$lib/icons/CloseIcon.svelte";
	import Icon from "$lib/icons/Icon.svelte";

	let {
		user,
		isActive,
		avatarSvg,
		isEditing,
		editingName = $bindable(""),
		isConfirmingRemove,
		onswitch,
		onstartedit,
		onsavename,
		oncancelname,
		onlogout,
		onstartremove,
		onconfirmremove,
		oncancelremove,
	}: {
		user: {
			username: string;
			user_type: string;
			yggdrasil_server_url?: string | null;
		};
		isActive: boolean;
		avatarSvg: string;
		isEditing: boolean;
		editingName?: string;
		isConfirmingRemove: boolean;
		onswitch: () => void;
		onstartedit: () => void;
		onsavename: () => void;
		oncancelname: () => void;
		onlogout: () => void;
		onstartremove: () => void;
		onconfirmremove: () => void;
		oncancelremove: () => void;
	} = $props();
</script>

<div
	class="card user-card"
	class:active={isActive}
	onclick={onswitch}
	role="button"
	tabindex="0"
	onkeydown={(e) => e.key === "Enter" && onswitch()}
>
	<div class="user-card-row">
		<div class="user-avatar-wrapper">
			{#if avatarSvg}
				{@html avatarSvg}
			{/if}
		</div>
		<div class="user-info">
			{#if isEditing}
				<!-- svelte-ignore a11y_autofocus -->
				<input
					type="text"
					bind:value={editingName}
					onkeydown={(e) => {
						if (e.key === "Enter") onsavename();
						if (e.key === "Escape") oncancelname();
					}}
					onblur={onsavename}
					maxlength="16"
					class="user-name-input"
					autofocus
				/>
			{:else}
				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<span
					class="user-name"
					class:clickable={user.user_type === "Cracked"}
					onclick={(e) => {
						if (user.user_type === "Cracked") {
							e.stopPropagation();
							onstartedit();
						}
					}}
					role={user.user_type === "Cracked" ? "button" : undefined}
					tabindex={user.user_type === "Cracked" ? 0 : undefined}
					onkeydown={(e) => {
						if (
							user.user_type === "Cracked" &&
							(e.key === "Enter" || e.key === " ")
						) {
							e.stopPropagation();
							onstartedit();
						}
					}}>{user.username}</span
				>
			{/if}
			<span class="user-type">
				{#if user.user_type === "Yggdrasil"}
					{t("userMenu.authInjector")} - {user.yggdrasil_server_url
						?.split("//")[1]
						?.split("/")[0] ?? "Servidor"}
				{:else if user.user_type === "Microsoft"}
					{t("userMenu.premium")}
				{:else}
					{t("userMenu.offline")}
				{/if}
			</span>
		</div>
		<div class="user-badges">
			{#if isActive}
				<span class="active-badge">{t("userMenu.active")}</span>
			{/if}
		</div>
		<div class="user-actions">
			{#if isActive && user.user_type === "Microsoft"}
				<button
					type="button"
					class="icon-btn"
					title={t("userMenu.logout")}
					onclick={(e) => {
						e.stopPropagation();
						onlogout();
					}}
				>
					<Icon src="/images/icons/ui/logout.svg" size={14} />
				</button>
			{/if}
			{#if isConfirmingRemove}
				<div class="confirm-group">
					<button
						type="button"
						class="icon-btn confirm-yes"
						onclick={(e) => {
							e.stopPropagation();
							onconfirmremove();
						}}
					>
						<Icon src="/images/icons/ui/check.svg" size={12} />
					</button>
					<button
						type="button"
						class="icon-btn confirm-no"
						onclick={(e) => {
							e.stopPropagation();
							oncancelremove();
						}}
					>
						<CloseIcon size={12} />
					</button>
				</div>
			{:else}
				<button
					type="button"
					class="icon-btn remove"
					title={t("userMenu.removeUser")}
					onclick={(e) => {
						e.stopPropagation();
						onstartremove();
					}}
				>
					<Icon src="/images/icons/ui/trash.svg" size={12} />
				</button>
			{/if}
		</div>
	</div>
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

	.user-card {
		cursor: pointer;
		transition: border-color 0.15s;
	}

	.user-card:hover {
		border-color: var(--text-muted);
	}

	.user-card.active {
		border-color: var(--accent);
	}

	.user-card-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 12px;
	}

	.user-avatar-wrapper {
		width: 32px;
		height: 32px;
		border-radius: var(--border-radius-sm);
		border: 1px solid var(--border-color);
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

	.user-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
		flex: 1;
	}

	.user-name {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.user-name.clickable {
		cursor: text;
		border-bottom: 1px dashed var(--text-muted);
	}

	.user-name.clickable:hover {
		border-bottom-color: var(--accent);
	}

	.user-name-input {
		font-size: 0.85rem;
		font-weight: 600;
		padding: 2px 4px;
		background: var(--bg-input);
		border: 1px solid var(--accent);
		border-radius: 3px;
		color: var(--text-primary);
		outline: none;
		width: 100%;
		box-sizing: border-box;
	}

	.user-type {
		font-size: 0.6rem;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.3px;
	}

	.user-badges {
		display: flex;
		gap: 4px;
		flex-shrink: 0;
	}

	.active-badge {
		font-size: 0.55rem;
		background: var(--accent);
		color: var(--bg-main);
		padding: 2px 6px;
		border-radius: 4px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.3px;
	}

	.user-actions {
		display: flex;
		gap: 2px;
		flex-shrink: 0;
	}

	.icon-btn {
		background: none;
		border: 1px solid transparent;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 4px;
		border-radius: 4px;
		display: flex;
		transition: all 0.15s;
	}

	.icon-btn:hover {
		color: var(--text-primary);
		border-color: var(--border-color);
		background: var(--surface-selected);
	}

	.icon-btn.remove:hover {
		color: var(--color-error);
		border-color: rgba(var(--color-error-rgb), 0.2);
		background: rgba(var(--color-error-rgb), 0.08);
	}

	.confirm-group {
		display: flex;
		gap: 2px;
	}

	.confirm-yes:hover {
		color: var(--color-success);
		border-color: rgba(var(--color-success-rgb), 0.2);
		background: rgba(var(--color-success-rgb), 0.08);
	}

	.confirm-no:hover {
		color: var(--color-error);
		border-color: rgba(var(--color-error-rgb), 0.2);
		background: rgba(var(--color-error-rgb), 0.08);
	}
</style>
