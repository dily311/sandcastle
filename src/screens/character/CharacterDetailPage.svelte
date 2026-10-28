<script lang="ts">
	import { _ } from "svelte-i18n";
	import Avatar from "$/components/avatar.svelte";
	import Button from "$/components/ui/button/Button.svelte";
	import Icon from "$/components/ui/Icon.svelte";
	import { characters } from "$/lib/chat/characterStore.svelte";
	import type { Character } from "$/lib/type/conv";
	import { pop, push, router } from "svelte-spa-router";

	// props로 편집할 characterId를 받습니다. (null일 경우 새로 만들기)
	let { params }: { params?: { characterId?: string } } = $props();

	const characterId = $derived(params?.characterId ?? null);
	// 초기 캐릭터 데이터 탐색
	const existingChar = $derived(characterId ? characters.value.find((c) => c.id === characterId) : null);

	// 채팅 바로 시작
	function handleStartChat() {
		if (!characterId) return;
	}
</script>

<div class="page narrow">
	<div>
		<div>커버 이미지</div>

		<div>
			타이틀
			<div>
				아바타
				<img src="" alt="" />
			</div>
			<h2>이름</h2>
		</div>
	</div>
</div>

<style>
</style>
