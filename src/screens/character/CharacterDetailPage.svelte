<script lang="ts">
	import { _ } from "svelte-i18n";
	import Avatar from "$/components/avatar.svelte";
	import Button from "$/components/ui/button/Button.svelte";
	import Icon from "$/components/ui/Icon.svelte";
	import { characters } from "$/lib/chat/characterStore.svelte";
	import type { Character } from "$/lib/type/conv";
	import { pop, push, router } from "svelte-spa-router";
	import WarningPage from "../WarningPage.svelte";

	// props로 편집할 characterId를 받습니다. (null일 경우 새로 만들기)
	let { params }: { params?: { characterId?: string } } = $props();

	const characterId = $derived(params?.characterId ?? null);
	// 초기 캐릭터 데이터 탐색
	const existingChar = $derived(characterId ? characters.value.find((c) => c.id === characterId) : null);

	// 채팅 바로 시작
	function handleStartChat() {
		if (!characterId) return;
	}

	const tabs = [
		{
			id: "info",
			label: "기본정보",
		},
		{},
	];
</script>

{#if !existingChar}
	<WarningPage />
{:else}
	<div class="coverImg">커버 이미지</div>
	<div class="page narrow">
		<div>
			<div class="title">
				<Avatar avatar={existingChar.avatar ?? ""} />
				<h2>{existingChar.name}</h2>
			</div>

			<div class="tabs">
				{#each tabs as item}
					<div>
						{item.label}
					</div>
				{/each}
			</div>
			<div class="content">
				<div>
					{existingChar.alternateDescriptions}
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.coverImg {
		width: 100%;
		height: 200px;
		background: var(--gray-2);
	}

	.title {
		transform: translateY(-2rem);
	}

	.tabs {
		display: flex;
	}
</style>
