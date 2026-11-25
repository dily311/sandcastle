<script lang="ts">
	import { _ } from "svelte-i18n";
	// 펜 버튼: 초안을 적어 AI에게 보내면 내 말투의 대사가 입력칸에 채워진다 (보내지는 않음). 설정에서 "초안 없이 바로 요청"을 켜면 누르는 즉시 요청.
	import { onMount } from "svelte";
	import { PenLine, Square, Undo2 } from "@lucide/svelte";
	import { Button, Dialog, IconButton, Textarea } from "$shared/ui";
	import { ghost } from "./ghost.svelte";
	import { pluginSetting } from "$shared/stores/aiTools.svelte";

	let { convId, getDraft, setDraft, disabled = false }: { convId: string; getDraft: () => string; setDraft: (v: string) => void; disabled?: boolean } = $props();

	// 설정 값(초안 없이 바로 요청)을 쓰므로 한 번 불러 둔다
	onMount(() => {
		if (!pluginSetting.value) void pluginSetting.load();
	});

	let open = $state(false);
	let brief = $state("");
	const running = $derived(ghost.isRunning(convId));
	const draft = $derived(getDraft());
	const direct = $derived(!!pluginSetting.value?.ghost?.sendWithoutDraft);

	// 입력을 비우면 대필 표시를 거둔다
	$effect(() => {
		if (!running && !draft.trim()) ghost.cleared(convId);
	});

	function press() {
		if (direct) return void ghost.run(convId, null, getDraft, setDraft);
		brief = getDraft(); // 이미 쓴 글이 있으면 그것이 초안
		open = true;
	}
	async function submit() {
		open = false;
		await ghost.run(convId, brief.trim() || null, getDraft, setDraft);
	}
</script>

{#if running}
	<IconButton label={$_("extraTools.ghostStopAria")} onclick={() => ghost.cancel(convId)}><Square /></IconButton>
{:else}
	<IconButton label={$_("extraTools.ghostRunAria")} onclick={press} {disabled}><PenLine /></IconButton>
	{#if ghost.canUndo(convId, draft)}
		<IconButton label={$_("extraTools.ghostUndoAria")} onclick={() => ghost.undo(convId, setDraft)}><Undo2 /></IconButton>
	{/if}
{/if}

<Dialog bind:open title={$_("extraTools.ghostTitle")}>
	<Textarea bind:value={brief} rows={4} autosize maxRows={10} placeholder={$_("extraTools.ghostPlaceholder")} aria-label={$_("extraTools.ghostAria")} />
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (open = false)}>{$_("common.cancel")}</Button>
		<Button variant="primary" onclick={submit}>{$_("extraTools.ghostSendBtn")}</Button>
	{/snippet}
</Dialog>
