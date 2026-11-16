//! 데이터 관리
//!
//! - overview: 캐릭터 → 채팅 트리, 개수, 저장 공간 (관리 화면)
//! - pack:     고른 캐릭터와 채팅만 옮기는 캐릭터 팩 (.scpack, 가져오면 병합)
//! - backup:   앱 데이터 전체 백업 (.scbackup) / 재시작 후 복원
//! - check:    DB 무결성·고아 파일 점검 (보여주기만)
pub mod archive;
pub mod backup;
pub mod card;
pub mod check;
pub mod overview;
pub mod pack;

#[cfg(test)]
mod tests;
