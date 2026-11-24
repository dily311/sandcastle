<script lang="ts">
	import { _ } from "svelte-i18n";
	import Icon from "$shared/ui/Icon.svelte";
	import { contextActionRegistry } from "$/lib/contextmenu/actions";
	import { contextMenu } from "$/lib/contextmenu/contextMenu.svelte";
	import { ArrowRight, Check, ChevronRight } from "@lucide/svelte";
	import type { ContextMenuContext, ContextMenuControl, ContextMenuItem } from "../types";

	async function execute(item: Pick<ContextMenuItem, "action" | "onselect">, context?: ContextMenuContext, keepOpen = false) {
		if (!item.action && !item.onselect) return;
		if (!keepOpen) contextMenu.close();

		if (item.onselect) await item.onselect(context);
		if (item.action) await contextActionRegistry.execute(item.action, context);
	}

	function resolveIcon(icon: ContextMenuControl["icon"]): string {
		return typeof icon === "function" ? icon() : icon;
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
		const children = Array.isArray(item.children) ? item.children : undefined;
		if (children && children.length != 0) {
			const rect = e.currentTarget.getBoundingClientRect();
			contextMenu.openSubmenu(rect.right, rect.top, children, context);

			return;
		}
		contextMenu.closeSubmenu();
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
				{:else if item.controls?.length}
					<li class="contextmenu-controls" onmouseenter={() => contextMenu.closeSubmenu()}>
						{#each item.controls as ctrl (ctrl.id)}
							<button
								type="button"
								class="control-btn"
								aria-label={$_(ctrl.label)}
								title={$_(ctrl.label)}
								disabled={ctrl.disabled?.(contextMenu.context) ?? false}
								onclick={(e) => {
									e.stopPropagation();
									void execute(ctrl, contextMenu.context, ctrl.keepOpen ?? false);
								}}
							>
								<Icon name={resolveIcon(ctrl.icon)} size={14} />
							</button>
						{/each}
					</li>
				{:else}
					{@const children = Array.isArray(item.children) ? item.children : undefined}
					<li class="contextmenu-item">
						<button
							type="button"
							class:danger={item.danger}
							disabled={item.disabled?.(contextMenu.context) ?? false}
							onclick={() => execute(item, contextMenu.context)}
							onmouseenter={(e) => handleMouseOn(e, item, contextMenu.context)}
						>
							<div class="menu-item-icon">
								{#if item.icon}
									<Icon name={item.icon} />
								{/if}
							</div>
							<div class="menu-item-title">
								{item.rawLabel ? item.label : $_(item.label)}
							</div>
							{#if children?.length}
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
							disabled={item.disabled?.(contextMenu.context) ?? false}
							onclick={() => execute(item, contextMenu.context)}
						>
							<div class="menu-item-icon">
								{#if item.icon}
									<Icon name={item.icon} />
								{/if}
							</div>
							<div class="menu-item-title">
								{item.rawLabel ? item.label : $_(item.label)}
							</div>
							{#if item.isChecked?.(contextMenu.context) ?? false}
								<div class="menu-item-icon mod-checked">
									<Check />
								</div>
							{/if}
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
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 0.4rem;
		padding: 0.2rem;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		white-space: nowrap;
	}

	.contextmenu hr {
		border-top: 1px solid var(--border-strong);
		margin: 0.2rem 0;
	}

	.contextmenu.submenu {
		position: fixed;
		z-index: 901;
	}

	.contextmenu-controls {
		display: flex;
		align-items: center;
		justify-content: space-around;
		gap: 0.2rem;
		padding: 0.15rem 0.2rem;
	}
	.control-btn {
		flex: 1 1 0;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0.35rem 0.4rem;
		border-radius: 0.3rem;
		color: var(--text);
		cursor: pointer;
	}
	.control-btn:hover:not(:disabled) {
		background-color: var(--surface-hover);
	}
	.control-btn:disabled {
		opacity: 0.4;
		cursor: default;
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
		background-color: var(--surface-hover);
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
