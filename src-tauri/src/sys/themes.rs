use anyhow::{anyhow, Context, Ok, Result};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::models::settings::{CssTheme, CssThemeMeta};

pub struct ThemeManager {
    themes_dir: PathBuf,
}

impl ThemeManager {
    // 초기화 시 themes 폴더 경로 저장 및 생성
    pub fn new(app_dir: PathBuf) -> Self {
        let themes_dir = app_dir.join("themes");
        if !themes_dir.exists() {
            let _ = fs::create_dir_all(&themes_dir);
        }
        Self { themes_dir }
    }

    // 기본 테마 파일저장
    pub fn init_default_themes(&self) -> Result<()> {
        let default_theme = [
            (
                "pastel.json",
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/theme/pastel.json")),
            ),
            (
                "catppuccin.json",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/resources/theme/catppuccin.json"
                )),
            ),
            (
                "mono.json",
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/theme/mono.json")),
            ),
        ];

        for (filename, contents) in default_theme {
            let theme_path = self.themes_dir.join(filename);
            if !theme_path.exists() {
                fs::write(theme_path, contents).context("테마 파일 복사중 실패")?;
            }
        }

        Ok(())
    }

    // 1. 목록 가져오기
    pub fn get_list(&self) -> Result<Vec<CssThemeMeta>> {
        if !self.themes_dir.exists() {
            return Err(anyhow!("테마가 저장됨 폴더 경로를 가져오는 데 실패함"));
        }

        let mut filenames: Vec<CssThemeMeta> = Vec::new();

        let entries = fs::read_dir(&self.themes_dir).context("파일 경로를 읽는 데 실패함")?;
        for entry in entries.flatten() {
            let path = entry.path();

            // 1. 조건 검사 (파일이면서 json 확장자인지)
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    let file = fs::File::open(&path)?;
                    let mut data: CssThemeMeta = serde_json::from_reader(file)?;
                    data.filename = filename.to_string();

                    filenames.push(data);
                }
            }
        }

        Ok(filenames)
    }

    // 2. 단일 테마 내용 읽기
    pub fn load_content(&self, filename: &str) -> Result<String> {
        let safe_filename = Path::new(filename)
            .file_name()
            .context("유효하지 않은 파일명입니다.")?;
        let theme_path = self.themes_dir.join(safe_filename);
        // theme_path.set_extension("json");

        let data = fs::read_to_string(theme_path).context("파일을 읽는 중 실패했습니다.")?;
        let json_var: Value = serde_json::from_str(&data)?;

        let css_content = json_var["cssContent"]
            .as_str()
            .map(|s| s.to_string())
            .context("CSS field not found in theme file")?;

        Ok(css_content)
    }

    // 3. 테마 저장
    pub fn save(&self, filename: &str, content: &CssTheme) -> Result<()> {
        let safe_filename = Path::new(filename)
            .file_name()
            .context("유효하지 않은 파일명입니다.")?;
        let save_path = self.themes_dir.join(safe_filename);
        let data = serde_json::to_string(content)?;
        fs::write(save_path, data).context("파일을 저장하는 중 실패했습니다.")?;

        Ok(())
    }

    // 4. 테마 삭제
    pub fn delete(&self, filename: &str) -> Result<()> {
        let safe_filename = Path::new(filename)
            .file_name()
            .context("유효하지 않은 파일명입니다.")?;
        let path = self.themes_dir.join(safe_filename);

        fs::remove_file(path).context("파일을 삭제하는 중 실패했습니다.")?;

        Ok(())
    }
}
