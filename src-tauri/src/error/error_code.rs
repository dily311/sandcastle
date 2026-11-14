use serde::Serialize;

// 에러 코드 정의
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCategory {
    Archive,
    NotFound,
    Validation,
    Database,
    AiProvider,
    Io,
    Window,
    Game,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // 공통
    InternalError,
    ResourceNotFound,
    InvalidInput,

    // Database
    DatabaseError,
    DatabaseLocked,

    // AI Provider
    AiProviderError,
    AiProviderRateLimit,
    AiProviderAuthFailed,
    AiProviderModelNotFound,
    AiProviderTimeout,
    AiProviderConnectionFailed,

    // Game
    GameInvalidState,
    GameSceneNotFound,
    GameEventFailed,
    GameDataInvalid, // 진단 에러로 시작 불가(details에 진단 목록)
    GameNotOpened, // open_game을 안 부름
    GameNoSession, // 진행 중인 세션이 없음
    GameSessionChanged, // AI를 기다리는 동안 세션이 바뀜
    GameActionBlocked, // 이동/대화/아이템 사용이 규칙상 불가(이유는 details.reason: conditionNotMet, sceneActive, notPresent 등)

    // Archive
    ArchiveInvalid,
    ArchiveCorrupted,
    ArchiveUnsafePath,

    // 기타
    IoError,
    WindowError,
}
