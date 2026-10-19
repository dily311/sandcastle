// ! rust로 넘어오는 데이터를 정의
// ! 혼동이 없도록 인터페이스와 구조체명을 동일하게 할 것

export interface ApiProfile {
    id: string,
    name: string,
    provider: string,
    formatType: string,
    baseUrl: string,
    model: string,
    selectedPresetId: string | null,
    createdAt: number,
    updatedAt: number,
}

export interface ApiProfileInput {
    name: string,
    provider: string,
    formatType: string,
    baseUrl: string,
    model: string,
    selectedPresetId: string | null,
}

// =======================================
// AI 연결
// =======================================
export interface APIList {
	label: string;
	provider: string;
	formatType: string;
	baseUrl: string;
	modelList: { label: string; model: string[] }[];
}
