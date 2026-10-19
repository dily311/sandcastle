<script lang="ts">
	import { _ } from "svelte-i18n";
	import Icon from "$/components/Icon.svelte";
	import { contextActionRegistry } from "$/lib/contextmenu/actions";
	import { contextMenu } from "$/lib/contextmenu/contextMenu.svelte";
	import { ArrowRight, Check, ChevronRight } from "@lucide/svelte";
	import type { ContextMenuContext, ContextMenuItem } from "../types";

	async function execute(action?: string, context?: ContextMenuContext) {
		if (!action) return;
		contextMenu.close();

		await contextActionRegistry.execute(action, context);
	}

	function handleWindowClick() {
		contextMenu.close();
	}

	function handleContextMenu(event: MouseEvent) {
		event.preventDefault();
	}

	function handleMouseOn(
		e: MouseEvent & { currentTarget: EventTarget & HTMLButtonElement },
		item: ContextMenuItem,
		context?: ContextMenuContext,
	) {
		if (item.children && item.children?.length != 0) {
			const rect = e.currentTarget.getBoundingClientRect();
			contextMenu.openSubmenu(rect.right, rect.top, item.children, context);

			return;
		}
	}
</script>

<svelte:window onclick={() => handleWindowClick()} oncontextmenu={(e) => handleContextMenu(e)} />

{#if contextMenu.isOpen}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		bind:this={contextMenu.menuEl}
		class="contextmenu-wrap"
		style="top: {contextMenu.pos.y}px; left: {contextMenu.pos.x}px;"
		onmouseleave={() => contextMenu.closeSubmenu()}
	>
		<ul class="contextmenu">
			{#each contextMenu.items as item}
				{#if item.separator}
					<hr />
				{:else}
					<li class="contextmenu-item">
						<button
							type="button"
							class:danger={item.danger}
							disabled={item.disabled?.() ?? false}
							onclick={() => execute(item.action, contextMenu.context)}
							onmouseenter={(e) => handleMouseOn(e, item, contextMenu.context)}
						>
							<div class="menu-item-icon">
								{#if item.icon}
									<Icon name={item.icon} />
								{/if}
							</div>
							<div class="menu-item-title">
								{$_(item.label)}
							</div>
							{#if item.children?.length}
								<div class="menu-item-icon mod-children">
									<span class="arrow">
										<ChevronRight />
									</span>
								</div>
							{/if}
							{#if item.isChecked?.(contextMenu.context) ?? false}
								<div class="menu-item-icon mod-checked">
									<Check />
								</div>
							{/if}
						</button>
					</li>
				{/if}
			{/each}
		</ul>
		{#if contextMenu.submenu}
			<ul
				bind:this={contextMenu.submenuEl}
				class="contextmenu submenu"
				style="left: {contextMenu.submenu.x}px; top: {contextMenu.submenu.y}px;"
			>
				{#each contextMenu.submenu.items ?? [] as item}
					<li class="contextmenu-item">
						<button
							type="button"
							class="menu-item"
							class:danger={item.danger}
							disabled={item.disabled?.() ?? false}
							onclick={() => execute(item.action, contextMenu.context)}
						>
							<div class="menu-item-icon">
								{#if item.icon}
									<Icon name={item.icon} />
								{/if}
							</div>
							<div class="menu-item-title">
								{$_(item.label)}
							</div>
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
{/if}

<style>
	.contextmenu-wrap {
		position: fixed;
		top: 0;
		left: 0;
		z-index: 900;

		--icon-size: 12px;
	}

	.contextmenu {
		width: auto;
		max-height: 100%;
		background: var(--background-primary);
		border: 1px solid var(--card-border-color);
		border-radius: 0.4rem;
		padding: 0.2rem;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		white-space: nowrap;
	}

	.contextmenu hr {
		border-top: 1px solid var(--hr-color);
		margin: 0.2rem 0;
	}

	.contextmenu.submenu {
		position: fixed;
		z-index: 901;
	}

	.contextmenu-item button {
		display: flex;
		align-items: center;
		width: 100%;
		padding: 0.2rem 0.4rem;
		gap: 0.6rem;
		font-size: 0.8rem;
		cursor: pointer;
		text-align: start;
	}
	.contextmenu-item button:hover {
		background-color: var(--button-hover-bg);
	}
	.menu-item-icon {
		display: flex;
		justify-content: center;
		align-items: center;
	}
	.menu-item-title {
		flex: 1 1 auto;
	}
</style>
