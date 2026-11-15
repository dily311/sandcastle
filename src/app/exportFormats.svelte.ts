// 내보내기 형식: 버튼(ExportSplit)과 메뉴의 '내보내기'(형식 고르기 팝업)가 같이 씀
// 고른 형식은 종류별로 이 컴퓨터에 기억해서 다음에 기본값으로.
export interface ExportFormat<T extends string = string> {
	id: T;
	label: string;
	desc?: string;
}

export function rememberedFormat<T extends string>(key: string, formats: ExportFormat<T>[]): T {
	try {
		const v = localStorage.getItem(key);
		if (formats.some((f) => f.id === v)) return v as T;
	} catch {
		/* 기억 못 하면 첫 형식 */
	}
	return formats[0].id;
}
export function rememberFormat(key: string, id: string) {
	try {
		localStorage.setItem(key, id);
	} catch {
		/* 그만 */
	}
}

interface Pick {
	at: { x: number; y: number };
	title: string;
	formats: ExportFormat[];
	key: string;
	run: (id: string) => void;
}

class ExportPicker {
	current = $state<Pick | null>(null);
	/** 마지막으로 누른 자리 (메뉴 항목을 누른 뒤 그 자리에 형식 목록을 띄움) */
	private last = { x: 200, y: 200 };

	constructor() {
		if (typeof window !== "undefined") window.addEventListener("pointerdown", (e) => (this.last = { x: e.clientX, y: e.clientY }), true);
	}
	/** 메뉴의 '내보내기…'에서: 누른 자리에 형식 목록 */
	open<T extends string>(title: string, key: string, formats: ExportFormat<T>[], run: (id: T) => void, at?: { x: number; y: number }) {
		this.current = { at: at ?? this.last, title, key, formats, run: run as (id: string) => void };
	}
	close() {
		this.current = null;
	}
}

export const exportPicker = new ExportPicker();
