<script lang="ts">
	// 보낸 메시지 컨텍스트 확인 창 (ChatView와 PhoneRoom이 동일하게 사용)
	import { _ } from "svelte-i18n";
	import { Dialog } from "$shared/ui";
	import type { ChatSession, Message } from "./session.svelte";

	interface Props {
		prompt: { msg: Message; data: Awaited<ReturnType<ChatSession["sentPrompt"]>> | null } | null;
		messageIndex?: number;
		onclose: () => void;
	}
	let { prompt, messageIndex = 0, onclose }: Props = $props();

	const roleName = $derived<Record<string, string>>({ system: $_("chatExtra.roles.system"), user: $_("chatExtra.roles.user"), assistant: $_("chatExtra.roles.assistant") });
	const sourceName = $derived<Record<string, string>>({
		preset: $_("chatExtra.sources.preset"),
		world_info: $_("chatExtra.sources.world_info"),
		system_rules: $_("chatExtra.sources.system_rules"),
		character: $_("chatExtra.sources.character"),
		persona: $_("chatExtra.sources.persona"),
		examples: $_("chatExtra.sources.examples"),
		chat: $_("chatExtra.sources.chat"),
		user_input: $_("chatExtra.sources.user_input"),
		qr: $_("chatExtra.sources.qr"),
		author_note: $_("chatExtra.sources.author_note"),
		memory: $_("chatExtra.sources.memory"),
		summary: $_("chatExtra.sources.summary"),
		recall: $_("chatExtra.sources.recall"),
		inject: $_("chatExtra.sources.inject"),
		plugin: $_("chatExtra.sources.plugin"),
		regen_extra: $_("chatExtra.sources.regen_extra"),
	});
</script>

<Dialog
	open={!!prompt}
	title={$_("chatExtra.sentPrompt.title")}
	description={prompt ? $_("chatExtra.sentPrompt.desc", { values: { index: messageIndex + 1 } }) : ""}
	size="lg"
	{onclose}
>
	{#if prompt?.data}
		{@const total = prompt.data.messages.reduce((s, m) => s + m.token, 0)}
		<p class="ptotal">{$_("chatExtra.sentPrompt.summary", { values: { count: prompt.data.messages.length, tokens: total.toLocaleString() } })}</p>
		<ol class="plist">
			{#each prompt.data.messages as pm, i (i)}
				<li>
					<details class="prole">
						<summary>
							<b>{roleName[pm.role] ?? pm.role}</b>{#if pm.source}<span class="psrc" data-kind={pm.source.kind}>{sourceName[pm.source.kind] ?? pm.source.kind}</span>{#if pm.source.name}<span class="pname">{pm.source.name}</span>{/if}{/if}<span class="ptok">{$_("chatExtra.sentPrompt.tokens", { values: { count: pm.token } })}</span>
						</summary>
						<pre>{pm.content}</pre>
					</details>
				</li>
			{/each}
		</ol>
	{:else if prompt}
		<p class="ptotal">{$_("chatExtra.sentPrompt.empty")}</p>
	{/if}
</Dialog>

<style>
	.ptotal {
		font-size: var(--text-sm);
		color: var(--text-muted);
		margin-bottom: var(--space-3);
	}
	.plist {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.prole {
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.prole summary {
		cursor: pointer;
	}
	.prole summary > b {
		margin-right: var(--space-2);
	}
	.psrc {
		display: inline-block;
		padding: 1px 6px;
		border-radius: var(--radius-sm);
		background: var(--bg-sunken);
		border: 1px solid var(--border);
		color: var(--text-muted);
		font-size: 11px;
		line-height: 1.4;
	}
	.psrc[data-kind="world_info"] {
		background: var(--accent-soft);
		border-color: transparent;
		color: var(--accent);
	}
	.psrc[data-kind="preset"] {
		color: var(--text);
	}
	.pname {
		margin-left: var(--space-1);
		color: var(--text);
		font-weight: 500;
	}
	.ptok {
		margin-left: var(--space-2);
		color: var(--text-faint);
	}
	.plist pre {
		margin: 0;
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-sm);
		background: var(--bg-sunken);
		font-family: var(--font-mono);
		font-size: 12px;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		max-height: 320px;
		overflow: auto;
	}
</style>
