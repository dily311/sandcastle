import { tick } from "svelte";
import type { ContextMenuContext, ContextMenuItem, ContextSubMenu } from "./types";
import { interactiveManager } from "$/lib/interaction/interactive-registry";
import { contextMenuRegistry } from "./registry";

const REGION_ID_CONTEXTMENU = "contextmenu";

class ContextMenuState {
	regionId = REGION_ID_CONTEXTMENU;
	isOpen = $state(false);
	pos = $state({ x: 0, y: 0 });
	menuEl = $state<HTMLElement | undefined>(undefined);
	submenuEl = $state<HTMLElement | undefined>(undefined);
	items = $state<ContextMenuItem[]>([]);
	context = $state<ContextMenuContext | undefined>(undefined);
	interactive = $state(false);
	submenu = $state<ContextSubMenu | undefined>(undefined);

	async openAt(x: number, y: number, items: ContextMenuItem[], interactive = false, context?: ContextMenuContext) {
		this.pos = { x: x, y: y };
		this.items = filterMenuItems(items, context);
		this.interactive = interactive;
		this.isOpen = true;
		this.context = context;
		this.submenu = undefined;

		await tick(); // DOM이 그려질 때까지 대기

		if (this.menuEl) {
			const { offsetWidth, offsetHeight } = this.menuEl;
			const { innerWidth, innerHeight } = window;

			// 오른쪽 끝 처리
			if (x + offsetWidth > innerWidth) x -= offsetWidth;
			// 아래쪽 끝 처리
			if (y + offsetHeight > innerHeight) y -= offsetHeight;
			if (x < 0) x = 0;
			this.pos = { x, y };
		}

		await tick(); // DOM이 그려질 때까지 대기

		// 클릭스루가 되지 않아야 한다면
		if (this.interactive && this.menuEl) {
			interactiveManager.registerInteractiveElement(this.regionId, this.menuEl);
		}
	}

	close() {
		this.isOpen = false;
		this.interactive = false;
		this.items = [];

		interactiveManager.unregisterInteractiveElement(this.regionId);
	}

	toggle(x: number, y: number, items: ContextMenuItem[], interactive = false) {
		if (this.isOpen) {
			this.close();
			return;
		}

		this.openAt(x, y, items, interactive);
	}

	async openSubmenu(x: number, y: number, items: ContextMenuItem[], context?: ContextMenuContext) {
		this.submenu = {
			x,
			y,
			items: filterMenuItems(items, context),
		};

		await tick(); // DOM이 그려질 때까지 대기

		if (this.menuEl && this.submenuEl) {
			const { offsetWidth, offsetHeight } = this.menuEl;
			const { offsetWidth: subOffsetWidth, offsetHeight: subOffsetHeight } = this.submenuEl;
			const { innerWidth, innerHeight } = window;

			// 오른쪽 끝 처리
			if (x + subOffsetWidth > innerWidth) x -= offsetWidth + subOffsetWidth -5;
			// 아래쪽 끝 처리
			if (y + subOffsetHeight > innerHeight) y -= offsetHeight + subOffsetHeight;
			if (x < 0) x = 0;
			this.submenu.x = x;
			this.submenu.y = y;
		}

		await tick(); // DOM이 그려질 때까지 대기

		// 클릭스루가 되지 않아야 한다면
		if (this.interactive && this.submenuEl) {
			interactiveManager.registerInteractiveElement(this.regionId + "submenu", this.submenuEl);
		}
	}
	closeSubmenu() {
		this.submenu = undefined;
		interactiveManager.unregisterInteractiveElement(this.regionId + "submenu");
	}
}

export const contextMenu = new ContextMenuState();

/** 컨텍스트 메뉴를 쉽게 열게 하기 위한 헬퍼 함수 */
export function handleContextMenu(event: any, id: string, context?: ContextMenuContext) {
	event.preventDefault();
	event.stopPropagation();
	const items = contextMenuRegistry.getFeatureItems(id);
	contextMenu.openAt(event.x, event.y, items, false, context);
}
/** 컨텍스트 메뉴를 쉽게 열게 하기 위한 헬퍼 함수 */
export function handleContextMenuForWidget(event: any, id: string, context?: ContextMenuContext) {
	event.preventDefault();
	event.stopPropagation();
	const items = contextMenuRegistry.getFeatureItems(id);
	contextMenu.openAt(event.x, event.y, items, true, context);
}
/** 컨텍스트 메뉴를 쉽게 열게 하기 위한 헬퍼 함수 */
export function handleContextMenuForMascot(event: any, id: string, context?: ContextMenuContext) {
	const items = contextMenuRegistry.getFeatureItems(id);
	contextMenu.openAt(event.x, event.y, items, true, context);
}

function filterMenuItems(items: ContextMenuItem[], context?: ContextMenuContext): ContextMenuItem[] {
	return items
		.filter((item) => {
			return item.visible ? item.visible(context) : true;
		})
		.map((item) => {
			if (!item.children) {
				return item;
			}
			const rawChildren = typeof item.children === "function" ? item.children(context) : item.children;
			return {
				...item,
				children: filterMenuItems(rawChildren, context),
			};
		})
		.filter((item) => {
			// children이 있었는데 전부 숨겨졌다면 부모도 숨길지 결정
			if (Array.isArray(item.children)) {
				return item.children.length > 0;
			}

			return true;
		});
}

/** 컨텍스트 메뉴에 쉽게 type을 검사할 수 있는 헬퍼 함수 */
export function visibleFor(...types: string[]) {
	return (context?: ContextMenuContext) => context?.type !== undefined && types.includes(context.type);
}
