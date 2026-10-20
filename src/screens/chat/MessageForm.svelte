<script lang="ts">
	import { conversations, messages, sendToModel } from "$/lib/chat/chatStore.svelte";
	import { appSetting } from "$/lib/stores/settingStore.svelte";
	import type { Character, Conversation } from "$type/conv";
	import { Plus, Send } from "@lucide/svelte";
	import { personas } from "$/lib/chat/personaStore.svelte";
	import TextArea from "$/components/ui/TextArea.svelte";
	import ButtonGroup from "$/components/ui/button/ButtonGroup.svelte";
	import Button from "$/components/ui/button/Button.svelte";

	let { conversation: conv, character }: { conversation: Conversation; character: Character | null } = $props();
	let inputMessage = $state("");

	async function handleSend() {
		if (conversations.isChattingLoading) return;

		const persona = personas.activePersona;
		if (conversations.isChattingLoading || !character || !persona) return;

		if (inputMessage !== "") {
			const userText = inputMessage ?? "";
			await messages.append({
				conversationId: conv.id,
				role: "user",
				name: persona.name,
				content: userText,
				meta: {
					sender: {
						type: "user",
						name: persona.name,
						avatar: persona.avatar,
						color: persona.color,
					},
				},
				characterId: null,
			});
			inputMessage = "";
		}

		await messages.startAssistantStream(conv.id, character);
	}

	async function handleKeyDown(e: KeyboardEvent) {
		if (conversations.isChattingLoading) return;
		if (
			(appSetting.sendOnEnter && e.key === "Enter" && !e.shiftKey) ||
			(!appSetting.sendOnEnter && e.key === "Enter" && e.shiftKey)
		) {
			e.preventDefault();
			handleSend();
		}
	}
</script>

<div class="message-form">
	<Button variant="ghost" size="icon" class="clickable-icon add-btn">
		<Plus />
	</Button>
	<div class="input-area">
		<TextArea bind:value={inputMessage} rows={1} placeholder="메시지를 입력하세요..." onkeydown={handleKeyDown} />
	</div>
	<Button variant="primary" size="lg" class="send-btn" onclick={() => handleSend()}>
		<Send />
	</Button>
</div>

<style>
	.message-form {
		padding: 12px 16px;
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.input-area {
		flex: 1;
	}

	.send-btn {
		background: var(--accent-color);
		color: white;
		border: none;
		border-radius: 12px;
		padding: 0 16px;
		height: 38px;
		font-weight: 600;
		display: flex;
		align-items: center;
		gap: 6px;
		cursor: pointer;
	}

	.send-btn:hover {
		background: var(--accent-color-hover);
	}
</style>
