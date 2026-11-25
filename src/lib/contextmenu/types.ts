export type ContextMenuSourceWindow = "app" | "widget";

export type ContextMenuSource = {
	// 메뉴를 제공하는 기능 ID
	featureId: string;
	// 메뉴의 priority
	// 숫자가 작을 수록 먼저 표시
	priority?: number;

	items: ContextMenuItem[];

	// 클릭스루 기능을 사용할 것인지 여부
	interactive?: boolean;
};

export type ContextMenuAction = string;

export type ContextMenuControl = {
	id: string;
	icon: string | (() => string);
	label: string;
	action?: ContextMenuAction;
	onselect?: (context?: ContextMenuContext) => void | Promise<void>;
	disabled?: (context?: ContextMenuContext) => boolean;
	keepOpen?: boolean;
};

export type ContextMenuItem = {
	id: string;
	label: string;
	/** true면 번역 키 대신 label 원문 그대로 표시 */
	rawLabel?: boolean;
	icon?: string;
	// 실행할 액션 ID
	action?: ContextMenuAction;
	/** 동적 메뉴 항목용 직접 실행 콜백 */
	onselect?: (context?: ContextMenuContext) => void | Promise<void>;
	/** 가로 컨트롤 버튼 묶음 (예: 음악 플레이어 << > >>) */
	controls?: ContextMenuControl[];
	// 하위 메뉴 (정적 목록 또는 메뉴 열 때마다 계산하는 함수)
	children?: ContextMenuItem[] | ((context?: ContextMenuContext) => ContextMenuItem[]);
	// 구분선
	separator?: boolean;
	// 위험한 작업 여부
	danger?: boolean;
	// 메뉴 표시 여부
	visible?: (context?: ContextMenuContext) => boolean;
	// 체크 여부
	isChecked?: (context?: ContextMenuContext) => boolean;
	// 비활성화 여부
	disabled?: (context?: ContextMenuContext) => boolean;
};

export type ContextMenuContext = {
	target?: HTMLElement;
	type?: string;
	data?: Record<string, unknown>;
};


export type ContextSubMenu = {
	x: number;
	y: number;
	items: ContextMenuItem[];
}