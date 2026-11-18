<script lang="ts">
	// 권한 목록 (설치 미리보기·켜기 동의·상세가 같이 쓴다). 위험한 것부터, 이번에 새로 늘어난 것은 표시
	import { Badge } from "$shared/ui";
	import { permissionInfo, sortPermissions, type Risk } from "./permissions";

	let { perms, added = [] }: { perms: readonly string[]; added?: readonly string[] } = $props();

	const TONE: Record<Risk, "danger" | "warning" | "neutral"> = { high: "danger", mid: "warning", low: "neutral" };
	const RISK: Record<Risk, string> = { high: "주의", mid: "보통", low: "낮음" };
</script>

{#if perms.length === 0}
	<p class="none">권한을 요청하지 않습니다. 데이터를 읽거나 바꾸지 않고, 화면에 아무것도 더하지 않습니다.</p>
{:else}
	<ul class="perms">
		{#each sortPermissions(perms) as p (p)}
			{@const i = permissionInfo(p)}
			<li>
				<div class="head">
					<Badge tone={TONE[i.risk]}>{RISK[i.risk]}</Badge>
					<b>{i.label}</b>
					{#if added.includes(p)}<Badge tone="accent">새로 늘어남</Badge>{/if}
					<code>{p}</code>
				</div>
				<p>{i.desc}</p>
			</li>
		{/each}
	</ul>
{/if}

<style>
	.perms {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-2);
	}
	li {
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		padding: var(--space-2) var(--space-3);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
	code {
		margin-left: auto;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	li p {
		margin: var(--space-1) 0 0;
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.none {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
</style>
