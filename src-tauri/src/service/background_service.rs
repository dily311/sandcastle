use async_trait::async_trait;
use std::{sync::Arc, time::Duration};
use tauri::{Manager, Runtime};
use tauri_plugin_background_service::{BackgroundService, ServiceContext, ServiceError};

use crate::task::TaskManager;

pub struct AppBackgroundService;

impl AppBackgroundService {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl<R: Runtime> BackgroundService<R> for AppBackgroundService {
    async fn init(&mut self, ctx: &ServiceContext<R>) -> Result<(), ServiceError> {
        // One-time setup goes here.
        // DB 및 서비스 초기화
        // interrupted 작업 복구 대상 조회
        let manager = ctx.app.state::<Arc<TaskManager>>().inner().clone();
        let _ = manager
            .recover_on_startup()
            .await
            .map_err(|e| ServiceError::Init(e.to_string()));
        Ok(())
    }

    async fn run(&mut self, ctx: &ServiceContext<R>) -> Result<(), ServiceError> {
        let manager = ctx.app.state::<Arc<TaskManager>>().inner().clone();
        let _ = manager.run_worker(ctx.shutdown.clone()).await;
        Ok(())
    }
}
