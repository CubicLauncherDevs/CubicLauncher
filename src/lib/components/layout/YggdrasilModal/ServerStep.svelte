<script lang="ts">
	let {
		serverUrl = $bindable(""),
		onconnect,
	}: {
		serverUrl?: string;
		onconnect: () => void;
	} = $props();

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === "Enter") onconnect();
	}
</script>

<div class="form-step">
	<p class="instruction-text">
		Ingresa la URL del servidor de autenticación Yggdrasil.
	</p>
	<div class="form-group">
		<label class="form-label" for="ygg-server-url">Servidor</label>
		<input
			id="ygg-server-url"
			type="text"
			class="form-input"
			placeholder="ej: littlesk.in"
			bind:value={serverUrl}
			onkeydown={handleKeydown}
		/>
	</div>
	<button
		type="button"
		class="action-btn primary"
		onclick={onconnect}
		disabled={!serverUrl.trim()}
	>
		Conectar
	</button>
</div>

<style>
	.form-step {
		display: flex;
		flex-direction: column;
		align-items: center;
		width: 100%;
		animation: fadeIn 0.4s ease;
	}

	.instruction-text {
		font-size: 0.85rem;
		color: var(--text-secondary);
		margin-bottom: 1.25rem;
		line-height: 1.6;
		padding: 0 0.5rem;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		width: 100%;
		max-width: 340px;
		gap: 0.4rem;
		margin-bottom: 1rem;
	}

	.form-label {
		font-size: 0.7rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.form-input {
		width: 100%;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 0.55rem 0.75rem;
		border-radius: var(--border-radius-sm);
		font-size: 0.85rem;
		font-family: inherit;
		box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.2);
		box-sizing: border-box;
		transition: border-color 0.15s;
	}

	.form-input:focus {
		outline: none;
		border-color: var(--accent);
	}

	.form-input::placeholder {
		color: var(--text-muted);
	}

	.action-btn {
		padding: 0.6rem 1.5rem;
		border-radius: var(--border-radius-sm);
		font-size: 0.8rem;
		font-weight: 600;
		font-family: inherit;
		cursor: pointer;
		transition: all 0.15s;
		border: none;
		flex: 1;
	}

	.action-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.action-btn.primary {
		background: var(--accent);
		color: var(--accent-text);
	}

	.action-btn.primary:hover:not(:disabled) {
		opacity: 0.85;
	}

	@keyframes fadeIn {
		from { opacity: 0; transform: translateY(8px); }
		to { opacity: 1; transform: translateY(0); }
	}
</style>