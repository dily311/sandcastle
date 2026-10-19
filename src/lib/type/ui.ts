
// 1. 파일의 메타데이터나 내용이 단순 문자열일 경우
export type FileContent = string; 

// 2. 재귀적 Map 타입 정의 (Value에 자기 자신인 FolderMap이 포함됨)
export type FolderMap = Map<string, FolderMap | FileContent>;