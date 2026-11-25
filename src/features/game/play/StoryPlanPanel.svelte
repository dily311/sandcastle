<script lang="ts">
	// 각본가(StoryPlan): 지금까지의 줄거리 계획을 보고, 각본가가 제안한 이벤트·씬을 무대에 올린다. 플랜은 조언일 뿐 상태를 바꾸지 않는다.
	// 켜진 게임에서만 보인다. 새 플랜은 누르거나(사용자) 자동 갱신 스위치를 켠 때(자동)만 AI를 부른다.
	import { Sparkles } from "@lucide/svelte";
	import { _ } from "svelte-i18n";
	import { Badge, Button, Switch } from "$shared/ui";
	import { gameStore } from "../runtime/gameStore.svelte";
	import { playBridge } from "../runtime/playBridge.svelte";
	import { runtimeStore } from "../runtime/runtimeStore.svelte";
	import { storyPlan } from "../runtime/storyPlan.svelte";

	const plan = $derived(storyPlan.record?.plan);
	const sceneName = (id: string) => gameStore.scenes?.find((s) => s.id === id)?.name ?? null;
	const busy = $derived(storyPlan.busy || playBridge.loading);
	const BEAT = $derived<Record<string, string>>({
		pending: $_("finalPolish.planBeatPending"),
		active: $_("finalPolish.planBeatActive"),
		done: $_("finalPolish.planBeatDone"),
		dropped: $_("finalPolish.planBeatDropped"),
	});
	let staging = $state<string | null>(null);

	// 이 세션의 마지막 플랜을 읽는다 (이어하기 때)
	$effect(() => {
		const sid = runtimeStore.state?.sessionId;
		if (storyPlan.enabled && sid) {
			storyPlan.reset();
			void storyPlan.load();
		}
	});
	// 자동 갱신: 스위치를 켠 때만, 체크포인트에 닿으면
	$effect(() => {
		if (storyPlan.autoRefresh && storyPlan.due && !storyPlan.busy) void storyPlan.refresh(true);
	});

	async function stage(kind: "event" | "scene", id: string) {
		staging = `${kind}:${id}`;
		try {
			await runtimeStore.stageSuggestion(kind, id);
			await storyPlan.load();
		} catch {
			/* 막힌 이유는 reportGameError가 알렸다 */
		} finally {
			staging = null;
		}
	}
</script>

{#if storyPlan.enabled}
	<section class="plan">
		<h4>
			{$_("finalPolish.planHeading")}
			{#if storyPlan.due}<Badge tone="accent">{$_("finalPolish.planDueBadge")}</Badge>{/if}
		</h4>
		{#if plan}
			<p class="premise">{plan.premise}</p>
			{#if plan.beats.length}
				<ol class="beats">
					{#each plan.beats as b (b.title)}
						<li class={b.status}><b>{b.title}</b><small>{BEAT[b.status] ?? b.status} · {b.goal}</small></li>
					{/each}
				</ol>
			{/if}
			{#if plan.openThreads.length}
				<h5>{$_("finalPolish.planOpenThreads")}</h5>
				<ul>{#each plan.openThreads as t (t)}<li>{t}</li>{/each}</ul>
			{/if}
			{#if plan.suggestions.length}
				<h5>{$_("finalPolish.planSuggestions")}</h5>
				<div class="sugs">
					{#each plan.suggestions as s (`${s.kind}:${s.id}`)}
						<div class="sug">
							<span class="t"><b>{s.kind === "scene" ? (sceneName(s.id) ?? $_("finalPolish.planSceneFallback")) : $_("finalPolish.planEventFallback")}</b><small>{s.reason}</small></span>
							<Button size="sm" variant="ghost" loading={staging === `${s.kind}:${s.id}`} disabled={busy || staging !== null} onclick={() => stage(s.kind, s.id)}>{$_("finalPolish.planStageBtn")}</Button>
						</div>
					{/each}
				</div>
			{/if}
		{:else}
			<p class="note">{$_("finalPolish.planEmpty")}</p>
		{/if}
		<div class="acts">
			<Button size="sm" loading={storyPlan.busy} disabled={busy} onclick={() => storyPlan.refresh(false)}>{#snippet icon()}<Sparkles />{/snippet}{$_("finalPolish.planRefreshBtn")}</Button>
		</div>
		<Switch bind:checked={storyPlan.autoRefresh} label={$_("finalPolish.planAutoRefresh")} description={$_("finalPolish.planAutoRefreshDesc")} />
	</section>
{/if}

<style>
	.plan {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	h5 {
		margin: var(--space-1) 0 0;
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.premise {
		margin: 0;
		font-size: var(--text-sm);
		line-height: 1.5;
	}
	ul,
	ol {
		margin: 0;
		padding-left: var(--space-4);
		font-size: var(--text-sm);
	}
	.beats {
		list-style: none;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.beats li {
		display: flex;
		flex-direction: column;
	}
	.beats li.done,
	.beats li.dropped {
		color: var(--text-muted);
	}
	.beats li.active b {
		color: var(--accent-text);
	}
	small {
		color: var(--text-muted);
		font-size: var(--text-xs);
	}
	.sug {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1) 0;
	}
	.sug .t {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		font-size: var(--text-sm);
	}
	.note {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.acts {
		display: flex;
	}
</style>
