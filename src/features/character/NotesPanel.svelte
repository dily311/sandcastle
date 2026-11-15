<script lang="ts">
	// 작가 노트·메모 (대화 meta에 저장, 보낼 때 프롬프트에 들어감). 대화·폰 패널이 같이 씀
	import { Input, Textarea } from "$shared/ui";
	import { library, type Conversation } from "./library.svelte";

	let { conv }: { conv: Conversation } = $props();

	type Notes = { memory?: string; authorNote?: { text?: string; depth?: number } };
	const notes = $derived((conv.meta ?? {}) as Notes);
	let memory = $state("");
	let noteText = $state("");
	let noteDepth = $state(4);
	$effect(() => {
		// 대화가 바뀌거나 저장된 값이 바뀌면 다시 채움
		memory = notes.memory ?? "";
		noteText = notes.authorNote?.text ?? "";
		noteDepth = notes.authorNote?.depth ?? 4;
	});
	function saveMemory() {
		if (memory === (notes.memory ?? "")) return;
		void library.patchMeta(conv.id, { memory: memory.trim() ? memory : null });
	}
	function saveNote() {
		const depth = Math.max(0, Math.min(50, Math.round(Number(noteDepth) || 0)));
		noteDepth = depth;
		if (noteText === (notes.authorNote?.text ?? "") && depth === (notes.authorNote?.depth ?? 4)) return;
		void library.patchMeta(conv.id, { authorNote: noteText.trim() ? { text: noteText, depth } : null });
	}
</script>

<div class="panel-body">
	<section>
		<h4>메모 <span class="faint">시스템 규칙 끝에 붙음</span></h4>
		<Textarea bind:value={memory} autosize rows={4} maxRows={14} placeholder={"긴 대화에서 잊으면 안 되는 사실\n예) {{user}}는 3장에서 열쇠를 잃어버림"} aria-label="메모" onblur={saveMemory} />
	</section>
	<section>
		<h4>작가 노트 <span class="faint">지금 장면의 방향</span></h4>
		<Textarea bind:value={noteText} autosize rows={4} maxRows={14} placeholder={"예) {{char}}는 아직 비밀을 털어놓지 않는다. 묘사는 짧게."} aria-label="작가 노트" onblur={saveNote} />
		<label class="depth">
			<span>깊이</span>
			<Input type="number" min="0" max="50" bind:value={noteDepth} aria-label="작가 노트 깊이" onblur={saveNote} />
		</label>
		<p class="hint">최근 메시지 몇 개 앞에 끼울지. 0이면 입력 바로 앞 (가장 세게 반영). <code>{"{{char}}"}</code> <code>{"{{user}}"}</code> 사용 가능</p>
	</section>
</div>

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
