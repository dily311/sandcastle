//! 데이터 관리
//!
//! - overview: 캐릭터 → 채팅 트리, 개수, 저장 공간 (관리 화면)
//! - pack:     고른 캐릭터와 채팅만 옮기는 캐릭터 팩 (.scpack, 가져오면 병합)
//! - backup:   앱 데이터 전체 백업 (.scbackup) / 재시작 후 복원
//! - check:    DB 무결성·고아 파일 점검 (보여주기만)
//! - fonts:    사용자가 올린 글꼴 파일 (fonts/)
//! - trash:    휴지통 (지운 항목 되살리기·완전히 지우기, 2-A-3)
//! - settings_scope: 설정 칸마다 같이 쓸지 / 기기 전용인지 (2-A-4)
//! - sync_bundle: 기기 사이에 주고받는 동기화 묶음 (.scsync, 2-B-1)
//! - sync_merge: 받은 묶음을 행 단위로 합치기 (2-B-2)
//! - sync_exchange: 파일로 주고받기 — 만들기·미리보기·적용 (2-B-3)
//! - sync_target: 동기화 저장소(폴더·드라이브)의 모양 (2-C-1)
//! - sync_run: 저장소 동기화 한 번 — 받기·올리기·파일·지운 기록 정리 (2-C-2)
//! - sync_conflicts: 둘 다 남긴 것 고르기, 합치기 되돌리기 (2-E)
pub mod archive;
pub mod backup;
pub mod card;
pub mod check;
pub mod fonts;
pub mod overview;
pub mod pack;
pub mod settings_scope;
pub mod sync_bundle;
pub mod sync_conflicts;
pub mod sync_exchange;
pub mod sync_merge;
#[cfg(test)]
mod sync_merge_tests;
pub mod sync_run;
pub mod sync_target;
pub mod trash;

#[cfg(test)]
mod tests;
