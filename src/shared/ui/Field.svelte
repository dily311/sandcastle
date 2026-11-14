<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		label: string;
		/** 입력 요소의 id (label for) */
		for?: string;
		hint?: string;
		error?: string | null;
		required?: boolean;
		/** 오른쪽 끝 보조 영역 (글자 수 등) */
		aside?: Snippet;
		children: Snippet;
	}
	let { label, for: htmlFor, hint, error, required = false, aside, children }: Props = $props();
</script>

<div class="field" class:invalid={!!error}>
	<div class="head">
		<label for={htmlFor}>
			{label}
			{#if required}<span class="req" aria-hidden="true">*</span>{/if}
		</label>
		{#if aside}<span class="aside">{@render aside()}</span>{/if}
	</div>
	{@render children()}
	{#if error}
		<p class="msg error" role="alert">{error}</p>
	{:else if hint}
		<p class="msg">{hint}</p>
	{/if}
</div>

<style>
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		min-width: 0;
	}
	.head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--space-2);
	}
	label {
		font-weight: 600;
		font-size: var(--text-sm);
		color: var(--text);
	}
	.req {
		color: var(--danger);
		margin-left: 2px;
	}
	.aside {
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.msg {
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.msg.error {
		color: var(--danger);
	}
</style>
