<script lang="ts">
	import type {
		CrashAnalysis,
		CrashFinding,
		CrashSource,
		DiagnosticAction,
	} from "$lib/api/diagnostics";
	import { formatAnalysis, sortFindings } from "$lib/api/diagnostics";
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import CopyIcon from "$lib/icons/CopyIcon.svelte";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";
	import { showError, showSuccess } from "$lib/state/state.svelte";

	let {
		analysis,
		loading = false,
		collapsible = false,
		onanalyze,
		onevidence,
		onaction,
	}: {
		analysis: CrashAnalysis | null;
		loading?: boolean;
		collapsible?: boolean;
		onanalyze?: () => void;
		onevidence?: (evidence: { line: number; text: string }) => void;
		onaction?: (action: DiagnosticAction) => void;
	} = $props();

	let collapsed = $state(false);
	const findings = $derived(analysis ? sortFindings(analysis.findings) : []);
	// La causa más probable va en el titular; el resto, como hallazgos debajo.
	const lead = $derived(
		findings.find((finding) => finding.code !== "unknown") ??
			findings[0] ??
			null,
	);
	const rest = $derived(findings.filter((finding) => finding !== lead));
	const leadAction = $derived(
		lead?.action && onaction && lead.code !== "unknown"
			? lead.action
			: null,
	);
	const exitCode = $derived(
		analysis ? (analysis.exitCode ?? analysis.context.exitCode) : null,
	);
	const hasMeta = $derived(
		!!analysis &&
			(!!analysis.context.minecraftVersion ||
				!!analysis.context.javaVersion ||
				exitCode !== null),
	);

	function findingTitle(finding: CrashFinding): string {
		const key = `diagnostics.crash.findings.${finding.code}`;
		const label = t(key, finding.params);
		return label === key
			? t("diagnostics.crash.unknownCode", { code: finding.code })
			: label;
	}

	function sourceLabel(source: CrashSource): string {
		return t(`diagnostics.crash.source.${source}`);
	}

	function actionLabel(action: DiagnosticAction): string {
		switch (action.kind) {
			case "reinstallVersion":
				return t("diagnostics.actions.reinstallVersion");
			case "installJava":
				return t("diagnostics.actions.installJava", {
					version: action.version,
				});
			case "openMods":
				return t("diagnostics.actions.openMods");
			case "openWorlds":
				return t("diagnostics.actions.openWorlds");
		}
	}

	async function copyDiagnosis() {
		if (!analysis) return;
		try {
			await navigator.clipboard.writeText(formatAnalysis(analysis));
			showSuccess(t("diagnostics.crash.copied"), "");
		} catch (error) {
			showError(t("errors.title"), String(error));
		}
	}
</script>

<section
	class="crash-analysis"
	class:error={lead?.severity === "error"}
	class:warning={lead?.severity === "warning"}
	aria-label={t("diagnostics.crash.title")}
>
	<header class="head">
		<button
			type="button"
			class="heading"
			class:static={!collapsible}
			onclick={() => {
				if (collapsible) collapsed = !collapsed;
			}}
			aria-expanded={collapsible ? !collapsed : true}
		>
			<span class="badge" aria-hidden="true">
				<Icon
					name="instance:terminal"
					fallbackName="instance:check-square"
					size={15}
				/>
			</span>
			<span class="heading-text">
				<span class="eyebrow">{t("diagnostics.crash.title")}</span>
				<span class="headline">
					{#if loading && !analysis}
						{t("diagnostics.crash.analyzing")}
					{:else if !analysis}
						{t("diagnostics.crash.idle")}
					{:else if lead && lead.code !== "unknown"}
						{findingTitle(lead)}
					{:else}
						{t("diagnostics.crash.noKnownCause")}
					{/if}
				</span>
			</span>
			{#if collapsible}
				<span class="chevron" class:open={!collapsed}>
					<ChevronDownIcon size={16} />
				</span>
			{/if}
		</button>

		<div class="actions">
			{#if leadAction}
				<button
					type="button"
					class="fix"
					onclick={() => onaction?.(leadAction)}
					>{actionLabel(leadAction)}</button
				>
			{/if}
			{#if onanalyze}
				<button
					type="button"
					class="ghost"
					disabled={loading}
					onclick={onanalyze}
				>
					{t("diagnostics.crash.analyzeAgain")}
				</button>
			{/if}
			<button
				type="button"
				class="ghost icon-only"
				disabled={!analysis}
				onclick={copyDiagnosis}
				title={t("diagnostics.crash.copy")}
				aria-label={t("diagnostics.crash.copy")}
			>
				<CopyIcon size={13} />
			</button>
		</div>
	</header>

	{#if !collapsed}
		{#if analysis}
			{#if hasMeta}
				<div class="meta">
					<span class="source" class:report={analysis.reportName}>
						<span class="source-dot" aria-hidden="true"></span>
						{sourceLabel(analysis.source)}
					</span>
					{#if analysis.context.minecraftVersion}
						<span class="meta-item">
							{t("diagnostics.crash.minecraft")}
							<strong>{analysis.context.minecraftVersion}</strong>
						</span>
					{/if}
					{#if analysis.context.javaVersion}
						<span class="meta-item">
							{t("diagnostics.crash.java")}
							<strong>{analysis.context.javaVersion}</strong>
						</span>
					{/if}
					{#if exitCode !== null}
						<span class="meta-item">
							{t("diagnostics.crash.exitCode")}
							<strong class="mono">{exitCode}</strong>
						</span>
					{/if}
				</div>
			{/if}

			{#if analysis.context.description}
				<p class="description">
					<span>{t("diagnostics.crash.description")}</span>
					{analysis.context.description}
				</p>
			{/if}

			{#if rest.length > 0}
				<ul class="findings">
					{#each rest as finding (finding.code)}
						<li class="finding" data-severity={finding.severity}>
							<div class="finding-head">
								<span class="severity-dot" aria-hidden="true"
								></span>
								<span class="finding-title"
									>{findingTitle(finding)}</span
								>
								{#if finding.action && onaction}
									<button
										type="button"
										class="fix"
										onclick={() =>
											onaction?.(finding.action!)}
										>{actionLabel(finding.action)}</button
									>
								{/if}
							</div>
							{#if finding.evidence.length > 0}
								<ul class="evidence">
									{#each finding.evidence as item (`${finding.code}:${item.line}`)}
										<li>
											<button
												type="button"
												class="evidence-line"
												title={t(
													"diagnostics.crash.jumpToLine",
												)}
												onclick={() =>
													onevidence?.(item)}
											>
												<span class="line"
													>L{item.line}</span
												>
												<code>{item.text}</code>
											</button>
										</li>
									{/each}
								</ul>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}

			<footer class="foot">
				{#if analysis.truncated}
					<span class="hint">{t("diagnostics.crash.truncated")}</span>
				{/if}
				{#if analysis.reportName}
					<span class="hint mono">{analysis.reportName}</span>
				{/if}
			</footer>
		{/if}
	{/if}
</section>

<style>
	.crash-analysis {
		--tone: var(--color-info);
		--tone-rgb: var(--color-info-rgb);
		position: relative;
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		background: var(--bg-card);
		color: var(--text-primary);
		font-size: 0.8rem;
		overflow: hidden;
	}
	.crash-analysis.error {
		--tone: var(--color-error);
		--tone-rgb: var(--color-error-rgb);
	}
	.crash-analysis.warning {
		--tone: var(--color-warning);
		--tone-rgb: var(--color-warning-rgb);
	}
	.crash-analysis::before {
		content: "";
		position: absolute;
		inset: 0 auto 0 0;
		width: 2px;
		background: var(--tone);
	}

	.head {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 12px 10px 14px;
		border-bottom: 1px solid var(--border);
	}
	.heading {
		display: flex;
		align-items: center;
		gap: 10px;
		flex: 1;
		min-width: 0;
		padding: 0;
		background: none;
		border: none;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.heading.static {
		cursor: default;
	}
	.badge {
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		flex-shrink: 0;
		border-radius: var(--border-radius-sm);
		background: rgba(var(--tone-rgb), 0.12);
		color: var(--tone);
	}
	.heading-text {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.eyebrow {
		font-size: 0.66rem;
		font-weight: 600;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--text-tertiary);
	}
	.headline {
		font-weight: 600;
		line-height: 1.35;
		overflow-wrap: anywhere;
	}
	.chevron {
		display: inline-flex;
		margin-left: auto;
		flex-shrink: 0;
		color: var(--text-tertiary);
		transition: transform 0.15s ease;
	}
	.chevron.open {
		transform: rotate(180deg);
	}
	.actions {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-shrink: 0;
	}
	.ghost {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 5px;
		height: 26px;
		padding: 0 9px;
		background: transparent;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font: inherit;
		font-size: 0.72rem;
		cursor: pointer;
		transition:
			color 0.15s ease,
			border-color 0.15s ease;
	}
	.ghost.icon-only {
		width: 26px;
		padding: 0;
	}
	.ghost:hover:not(:disabled) {
		color: var(--text-primary);
		border-color: var(--border-hover);
	}
	.ghost:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 4px 14px;
		padding: 9px 14px;
		border-bottom: 1px solid var(--border);
		font-size: 0.72rem;
		color: var(--text-tertiary);
	}
	.meta-item strong {
		font-weight: 500;
		color: var(--text-secondary);
	}
	.source {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.source-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-tertiary);
	}
	.source.report .source-dot {
		background: var(--color-success);
	}
	.mono {
		font-family: var(--font-family-mono);
	}

	.description {
		margin: 0;
		padding: 10px 14px;
		border-bottom: 1px solid var(--border);
		background: var(--surface-subtle);
		line-height: 1.45;
		overflow-wrap: anywhere;
	}
	.description span {
		display: block;
		margin-bottom: 2px;
		font-size: 0.66rem;
		font-weight: 600;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--text-tertiary);
	}

	.findings {
		list-style: none;
		margin: 0;
		padding: 10px 12px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.finding {
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-card);
		padding: 9px 11px;
	}
	.finding-head {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.severity-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
		background: var(--text-tertiary);
	}
	.finding[data-severity="error"] .severity-dot {
		background: var(--color-error);
	}
	.finding[data-severity="warning"] .severity-dot {
		background: var(--color-warning);
	}
	.finding-title {
		flex: 1;
		font-weight: 600;
		line-height: 1.35;
	}
	.fix {
		flex-shrink: 0;
		height: 26px;
		padding: 0 10px;
		border: 1px solid color-mix(in srgb, var(--tone) 60%, transparent);
		border-radius: var(--border-radius-sm);
		background: rgba(var(--tone-rgb), 0.12);
		color: var(--text-primary);
		font: inherit;
		font-size: 0.72rem;
		font-weight: 500;
		white-space: nowrap;
		cursor: pointer;
		transition: background 0.15s ease;
	}
	.fix:hover {
		background: rgba(var(--tone-rgb), 0.22);
	}

	.evidence {
		list-style: none;
		margin: 8px 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.evidence-line {
		display: flex;
		align-items: baseline;
		gap: 10px;
		width: 100%;
		padding: 4px 8px;
		background: var(--bg-input);
		border: 1px solid transparent;
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font: inherit;
		text-align: left;
		cursor: pointer;
		transition:
			border-color 0.15s ease,
			color 0.15s ease;
	}
	.evidence-line:hover,
	.evidence-line:focus-visible {
		border-color: var(--border-hover);
		color: var(--text-primary);
		outline: none;
	}
	.evidence-line .line {
		flex-shrink: 0;
		min-width: 3ch;
		padding-right: 8px;
		border-right: 1px solid var(--border);
		color: var(--text-muted);
		font-family: var(--font-family-mono);
		font-size: 0.68rem;
		user-select: none;
	}
	.evidence-line code {
		min-width: 0;
		font-family: var(--font-family-mono);
		font-size: 0.71rem;
		line-height: 1.45;
		overflow-wrap: anywhere;
		white-space: pre-wrap;
	}

	.foot {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 4px 12px;
		padding: 0 14px 10px;
		font-size: 0.7rem;
		color: var(--text-tertiary);
	}
	.foot .hint {
		margin: 0;
		line-height: 1.45;
	}
</style>
