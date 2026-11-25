<script lang="ts">
	// 대화 옵션 자물쇠 (5-E-29, ADR-028): 잠김 = 이 대화의 값을 씀. 풀림 = 캐릭터 기본 → 전역을 따름.
	// 누르면 지금 보이는 값을 이 대화에 고정하거나, 풀고 다시 따르게 한다
	import { Lock, LockOpen } from "@lucide/svelte";
	import { _ } from "svelte-i18n";
	import { IconButton } from "$shared/ui";

	let {
		locked,
		from,
		onclick,
		disabled = false,
	}: {
		locked: boolean;
		/** 풀렸을 때 따르는 곳: "캐릭터 기본" · "전역" */
		from: string;
		onclick: () => void;
		disabled?: boolean;
	} = $props();
</script>

<IconButton
	class="sc-option-lock"
	size="sm"
	active={locked}
	aria-pressed={locked}
	label={locked ? $_("uiPolish.lockPinned", { values: { from } }) : $_("uiPolish.lockFollowing", { values: { from } })}
	{disabled}
	{onclick}
>
	{#if locked}<Lock />{:else}<LockOpen />{/if}
</IconButton>
