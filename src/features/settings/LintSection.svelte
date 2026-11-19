<script lang="ts">
	// 메시지 linter 규칙 (3-B, ADR-014). 요금 없이 앱 안에서 도는 검사. 응답이 저장될 때 한 번, 플러그인 후처리보다 먼저
	import { onMount } from "svelte";
	import type { LintMode } from "$ipc/bindings";
	import { Input, Select, TagInput } from "$shared/ui";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import { current, ensureLoaded, save } from "./pluginSettingForm.svelte";

	onMount(ensureLoaded);
	const cur = $derived(current());

	const MODES: { value: LintMode; label: string }[] = [
		{ value: "off", label: "끄기" },
		{ value: "report", label: "표시만" },
		{ value: "fix", label: "자동 고침" },
	];
	const REPORT_ONLY = MODES.filter((m) => m.value !== "fix");
	const num = (v: string) => Math.max(0, Math.min(100000, Math.floor(Number(v)) || 0));
	// 금지어 목록은 태그로 (월드인포 키워드처럼). 태그가 바뀌면 저장한다
	let bannedWords = $state<string[]>([]);
	$effect(() => {
		bannedWords = [...(cur?.lint.bannedWords ?? [])];
	});
	$effect(() => {
		const next = [...new Set(bannedWords.map((w) => w.trim()).filter(Boolean))].slice(0, 100);
		if (cur && JSON.stringify(next) !== JSON.stringify(cur.lint.bannedWords)) void save("lint", { bannedWords: next });
	});
</script>

<Section title="메시지 linter" lead="응답이 저장될 때 글을 검사함. 요금은 들지 않음. 고치면 원문이 남고 메시지 아래에서 되돌릴 수 있음. 스트리밍 중에는 돌지 않음">
	{#if cur}
		<Group>
			<Row label="닫히지 않은 것" desc="태그(<b> 등)·코드 블록(```)·굵게(**)·기울임(*)·따옴표. 고치면 글 끝에 닫는 표시를 붙임">
				<Select value={cur.lint.unclosed} options={MODES} onchange={(e) => save("lint", { unclosed: e.currentTarget.value as LintMode })} />
			</Row>
			<Row label="반복된 문단" desc="같은 문단이 한 응답에 또 나옴. 고치면 두 번째부터 지움">
				<Select value={cur.lint.repeat} options={MODES} onchange={(e) => save("lint", { repeat: e.currentTarget.value as LintMode })} />
			</Row>
			<Row label="단어 반복 오류" desc="같은 말을 끝없이 되풀이하는 고장. 고치면 두 번까지만 남김">
				<Select value={cur.lint.stuck} options={MODES} onchange={(e) => save("lint", { stuck: e.currentTarget.value as LintMode })} />
			</Row>
			<Row label="내 대사 대신 쓰기" desc="'내 이름: …'으로 시작하는 줄. 고치면 그 줄부터 잘라 냄">
				<Select value={cur.lint.impersonate} options={MODES} onchange={(e) => save("lint", { impersonate: e.currentTarget.value as LintMode })} />
			</Row>
			<Row label="금지어" desc="대소문자 구분 없음. 고치면 그 단어를 지움">
				<Select value={cur.lint.banned} options={MODES} onchange={(e) => save("lint", { banned: e.currentTarget.value as LintMode })} />
			</Row>
			{#if cur.lint.banned !== "off"}
				<Row label="금지어 목록" desc="Enter나 쉼표로 추가 (100개까지). 자동 재생성의 금지어와는 따로임" stacked>
					<TagInput bind:value={bannedWords} placeholder="금지어 입력" label="금지어 추가" />
				</Row>
			{/if}
			<Row label="글자 수" desc="범위를 벗어나면 알림만 (고치지 않음)">
				<Select value={cur.lint.length} options={REPORT_ONLY} onchange={(e) => save("lint", { length: e.currentTarget.value as LintMode })} />
			</Row>
			{#if cur.lint.length !== "off"}
				<Row label="최소 / 최대 글자" desc="0이면 제한 없음">
					<div class="pair">
						<Input type="number" min="0" value={String(cur.lint.minChars)} onchange={(e) => save("lint", { minChars: num(e.currentTarget.value) })} />
						<Input type="number" min="0" value={String(cur.lint.maxChars)} onchange={(e) => save("lint", { maxChars: num(e.currentTarget.value) })} />
					</div>
				</Row>
			{/if}
		</Group>
	{/if}
</Section>

<style>
	.pair {
		display: flex;
		gap: var(--space-2);
	}
</style>
