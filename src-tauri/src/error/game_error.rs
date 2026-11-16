//! 게임 오류 코드 만들기. 화면은 code로 종류를, details.reason으로 막힌 이유를 구분한다.
//!
//! | 코드                 | 언제                                                   |
//! |----------------------|--------------------------------------------------------|
//! | GAME_NOT_OPENED      | open_game을 부르지 않았다                               |
//! | GAME_NO_SESSION      | 진행 중인 세션(런타임)이 없다                            |
//! | GAME_SESSION_CHANGED | AI를 기다리는 동안 세션이 바뀌었다                       |
//! | GAME_DATA_INVALID    | 게임 데이터 문제로 시작할 수 없다 (details.diagnostics)  |
//! | GAME_ACTION_BLOCKED  | 규칙상 지금 할 수 없는 행동 (details.reason)             |
//! | GAME_INVALID_STATE   | 지금 상태에서는 맞지 않는 요청 (지난 플레이의 메시지 고치기 등) |
use serde::Serialize;
use serde_json::json;

use crate::error::{AppError, ErrorCode};

/// GAME_ACTION_BLOCKED의 details.reason
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BlockReason {
    /// 이동·아이템 사용 조건이 맞지 않음
    ConditionNotMet,
    /// 씬이 진행 중이라 할 수 없음 (이동, 1:1 대화)
    SceneActive,
    /// 상대가 이 자리에 없음
    NotPresent,
    /// 지금 위치에서 그곳으로 가는 길이 없음 (또는 현재 위치가 없음)
    NoRoute,
    /// 열린 대화가 없거나 다른 캐릭터와의 대화가 열려 있음
    NoActiveDialogue,
    /// 열린 씬이 없음
    NoOpenScene,
    /// 엔딩 씬은 끝낼 수 없음
    EndingScene,
    /// 지금 열린 씬의 선택지가 아님
    ChoiceNotInScene,
    /// 선택지의 표시 조건이 맞지 않음
    ChoiceUnavailable,
    /// 씬 입력을 쓸 수 없는 씬 (엔딩이거나 AI 이어가기가 꺼짐)
    SceneChatDisabled,
    /// 가지고 있지 않은 아이템
    ItemNotOwned,
    /// 사용 효과가 없는 아이템
    ItemNotUsable,
    /// 핵심 아이템은 버릴 수 없음
    KeyItem,
}

impl AppError {
    fn game_with(code: ErrorCode, message: impl Into<String>, details: Option<serde_json::Value>) -> Self {
        Self::Game {
            code,
            message: message.into(),
            details,
        }
    }

    /// 규칙상 막힌 행동
    pub fn game_blocked(reason: BlockReason, message: impl Into<String>) -> Self {
        Self::game_with(ErrorCode::GameActionBlocked, message, Some(json!({ "reason": reason })))
    }

    pub fn game_not_opened() -> Self {
        Self::game_with(ErrorCode::GameNotOpened, "열려 있는 게임이 없습니다 (open_game을 먼저 부르세요)", None)
    }

    pub fn game_no_session() -> Self {
        Self::game_with(ErrorCode::GameNoSession, "진행 중인 세션이 없습니다", None)
    }

    pub fn game_session_changed() -> Self {
        Self::game_with(ErrorCode::GameSessionChanged, "응답을 기다리는 동안 세션이 바뀌었습니다", None)
    }

    /// 게임 데이터 문제. diagnostics는 validate의 진단 목록(있으면)
    pub fn game_data_invalid(message: impl Into<String>, diagnostics: Option<serde_json::Value>) -> Self {
        Self::game_with(ErrorCode::GameDataInvalid, message, diagnostics.map(|d| json!({ "diagnostics": d })))
    }

    pub fn game_invalid_state(message: impl Into<String>) -> Self {
        Self::game_with(ErrorCode::GameInvalidState, message, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_error_carries_reason() {
        let res = AppError::game_blocked(BlockReason::NotPresent, "없음").to_response();
        assert_eq!(res.code, ErrorCode::GameActionBlocked);
        assert_eq!(res.details, Some(json!({ "reason": "notPresent" })));
    }
}
