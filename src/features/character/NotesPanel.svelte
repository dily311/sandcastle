<script lang="ts">
	import { _ } from "svelte-i18n";
	// 메모(내 것, AI에게 보내지 않음)와 작가 노트(보낼 때 프롬프트에 들어감) — 대화 meta에 저장. 대화·폰 패널이 같이 씀
	import { Input, Textarea } from "$shared/ui";
	import { library, type Conversation } from "./library.svelte";

	/** bare: 감싸는 panel-body 없이 섹션만 (대화 설정 탭 안에 넣을 때) */
	let { conv, bare = false }: { conv: Conversation; bare?: boolean } = $props();

	type Notes = { memory?: string; authorNote?: { text?: string; depth?: number } };
	const notes = $derived((conv.meta ?? {}) as Notes);
	let memory = $state("");
	let noteText = $state("");
	let noteDepth = $state(4);
	$effect(() => {
		// 대화가 바뀌거나 저장된 값이 바뀌면 다시 채움
		memory = memoOf(notes.memory);
		noteText = notes.authorNote?.text ?? "";
		noteDepth = notes.authorNote?.depth ?? 4;
	});
	// 예전 버전이 메모 칸에 "off"를 메모리 검색 스위치로 썼다 — 메모가 아니다
	const memoOf = (m: string | undefined) => (m === "off" ? "" : (m ?? ""));
	function saveMemory() {
		if (memory === memoOf(notes.memory)) return;
		void library.patchMeta(conv.id, { memory: memory.trim() ? memory : null });
	}
	function saveNote() {
		const depth = Math.max(0, Math.min(50, Math.round(Number(noteDepth) || 0)));
		noteDepth = depth;
		if (noteText === (notes.authorNote?.text ?? "") && depth === (notes.authorNote?.depth ?? 4)) return;
		void library.patchMeta(conv.id, { authorNote: noteText.trim() ? { text: noteText, depth } : null });
	}
</script>

{#snippet fields()}
	<section>
		<h4>{$_("notesPanel.memoHeading")}</h4>
		<Textarea bind:value={memory} autosize rows={4} maxRows={14} placeholder={$_("notesPanel.memoPlaceholder")} aria-label={$_("notesPanel.memoAria")} onblur={saveMemory} />
	</section>
	<section>
		<h4>{$_("notesPanel.anHeading")}</h4>
		<Textarea bind:value={noteText} autosize rows={4} maxRows={14} placeholder={$_("notesPanel.anPlaceholder")} aria-label={$_("notesPanel.anHeading")} onblur={saveNote} />
		<label class="depth">
			<span>{$_("notesPanel.depthLabel")}</span>
			<Input type="number" min="0" max="50" bind:value={noteDepth} aria-label={$_("notesPanel.depthAria")} onblur={saveNote} />
		</label>
		<p class="hint">{$_("notesPanel.depthHint")}</p>
	</section>
{/snippet}

{#if bare}
	{@render fields()}
{:else}
	<div class="panel-body">{@render fields()}</div>
{/if}

<style>
	.depth {
		display: grid;
		grid-template-columns: 64px 96px;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.hint {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-muted);
		line-height: 1.5;
	}
	.hint code {
		font-family: var(--font-mono);
		font-size: 11px;
	}
</style>
