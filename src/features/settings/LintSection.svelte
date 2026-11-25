<script lang="ts">
	// 메시지 linter 규칙 (3-B, ADR-014). 요금 없이 앱 안에서 도는 검사. 응답이 저장될 때 한 번, 플러그인 후처리보다 먼저
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import type { LintMode } from "$ipc/bindings";
	import { Input, Select, TagInput } from "$shared/ui";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import { current, ensureLoaded, save } from "./pluginSettingForm.svelte";

	onMount(ensureLoaded);
	const cur = $derived(current());

	const MODES = $derived<{ value: LintMode; label: string }[]>([
		{ value: "off", label: $_("settings.lintSec.modeOff") },
		{ value: "report", label: $_("settings.lintSec.modeReport") },
		{ value: "fix", label: $_("settings.lintSec.modeFix") },
	]);
	const REPORT_ONLY = $derived(MODES.filter((m) => m.value !== "fix"));
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

<Section title={$_("settings.lintSec.title")} lead={$_("settings.lintSec.lead")}>
	{#if cur}
		<Group>
			<Row label={$_("settings.lintSec.unclosed")} desc={$_("settings.lintSec.unclosedDesc")}>
				<Select value={cur.lint.unclosed} options={MODES} onchange={(e) => save("lint", { unclosed: e.currentTarget.value as LintMode })} />
			</Row>
			<Row label={$_("settings.lintSec.repeat")} desc={$_("settings.lintSec.repeatDesc")}>
				<Select value={cur.lint.repeat} options={MODES} onchange={(e) => save("lint", { repeat: e.currentTarget.value as LintMode })} />
			</Row>
			<Row label={$_("settings.lintSec.stuck")} desc={$_("settings.lintSec.stuckDesc")}>
				<Select value={cur.lint.stuck} options={MODES} onchange={(e) => save("lint", { stuck: e.currentTarget.value as LintMode })} />
			</Row>
			<Row label={$_("settings.lintSec.impersonate")} desc={$_("settings.lintSec.impersonateDesc")}>
				<Select value={cur.lint.impersonate} options={MODES} onchange={(e) => save("lint", { impersonate: e.currentTarget.value as LintMode })} />
			</Row>
			<Row label={$_("settings.lintSec.banned")} desc={$_("settings.lintSec.bannedDesc")}>
				<Select value={cur.lint.banned} options={MODES} onchange={(e) => save("lint", { banned: e.currentTarget.value as LintMode })} />
			</Row>
			{#if cur.lint.banned !== "off"}
				<Row label={$_("settings.lintSec.bannedList")} desc={$_("settings.lintSec.bannedListDesc")} stacked>
					<TagInput bind:value={bannedWords} placeholder={$_("settings.lintSec.bannedPlaceholder")} label={$_("settings.lintSec.bannedList")} />
				</Row>
			{/if}
			<Row label={$_("settings.lintSec.length")} desc={$_("settings.lintSec.lengthDesc")}>
				<Select value={cur.lint.length} options={REPORT_ONLY} onchange={(e) => save("lint", { length: e.currentTarget.value as LintMode })} />
			</Row>
			{#if cur.lint.length !== "off"}
				<Row label={$_("settings.lintSec.minMaxChars")} desc={$_("settings.lintSec.minMaxDesc")}>
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
