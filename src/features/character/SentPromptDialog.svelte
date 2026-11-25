<script lang="ts">
	// 보낸 메시지 컨텍스트 확인 창 (ChatView와 PhoneRoom이 동일하게 사용)
	import { Dialog } from "$shared/ui";
	import type { ChatSession, Message } from "./session.svelte";

	interface Props {
		prompt: { msg: Message; data: Awaited<ReturnType<ChatSession["sentPrompt"]>> | null } | null;
		messageIndex?: number;
		onclose: () => void;
	}
	let { prompt, messageIndex = 0, onclose }: Props = $props();

	const roleName: Record<string, string> = { system: "시스템", user: "사용자", assistant: "AI" };
	const sourceName: Record<string, string> = {
		preset: "프리셋",
		world_info: "월드인포",
		system_rules: "시스템 규칙",
		character: "캐릭터",
		persona: "페르소나",
		examples: "예시 대화",
		chat: "대화 기록",
		user_input: "이번 입력",
		qr: "QR 지시",
		author_note: "작가 노트",
		memory: "메모",
		summary: "요약",
		recall: "회상",
		inject: "주입",
		plugin: "플러그인",
		regen_extra: "추가 지시",
	};
</script>

<Dialog
	open={!!prompt}
	title="보낸 컨텍스트"
	description={prompt ? `#${messageIndex + 1} 응답을 만들 때 보낸 프롬프트` : ""}
	size="lg"
	{onclose}
>
	{#if prompt?.data}
		{@const total = prompt.data.messages.reduce((s, m) => s + m.token, 0)}
		<p class="ptotal">메시지 {prompt.data.messages.length}개 · 약 {total.toLocaleString()} 토큰</p>
		<ol class="plist">
			{#each prompt.data.messages as pm, i (i)}
				<li>
					<details class="prole">
						<summary>
							<b>{roleName[pm.role] ?? pm.role}</b>{#if pm.source}<span class="psrc" data-kind={pm.source.kind}>{sourceName[pm.source.kind] ?? pm.source.kind}</span>{#if pm.source.name}<span class="pname">{pm.source.name}</span>{/if}{/if}<span class="ptok">{pm.token} 토큰</span>
						</summary>
						<pre>{pm.content}</pre>
					</details>
				</li>
			{/each}
		</ol>
	{:else if prompt}
		<p class="ptotal">기록 없음 (가져온 메시지, 멈춘 응답, 이 기능 이전에 만든 응답)</p>
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
