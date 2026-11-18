//! 올린 글꼴 (앱 데이터 fonts/)
//!
//! 사용자가 고른 글꼴 파일(ttf·otf·woff·woff2)을 앱 데이터 `fonts/`에 복사해 둔다 (원본은 건드리지 않음).
//! 파일은 사용자 데이터라 백업·동기화에 들어간다 (ADR-008 표). 앱에 글꼴을 넣어 두지 않으므로 라이선스는 사용자 몫이다.
//! 확장자만 믿지 않고 파일 머리(매직 바이트)도 본다 — 글꼴이 아닌 파일이 `@font-face`로 불리지 않게.
use std::path::{Path, PathBuf};

use crate::{
    error::{AppError, Result},
    models::settings::UserFont,
};

pub const FONTS_DIR: &str = "fonts";
/// 한 파일 상한. 한글 글꼴은 수 MB~수십 MB라 넉넉히
pub const MAX_FONT_BYTES: u64 = 50 * 1024 * 1024;
/// 한 번에 받을 개수
pub const MAX_PER_CALL: usize = 20;
/// 이름표 길이 상한
const MAX_NAME_CHARS: usize = 60;

/// 확장자와 파일 머리가 맞는 글꼴이면 소문자 확장자를 돌려준다. TTC(글꼴 묶음)는 `@font-face`가 못 읽어 받지 않는다
pub fn check_header(ext: &str, head: &[u8]) -> Result<String> {
    let ext = ext.to_ascii_lowercase();
    let ok = match ext.as_str() {
        // TrueType(00 01 00 00, "true")이거나 CFF 기반 OpenType("OTTO")
        "ttf" | "otf" => matches!(head.get(..4), Some([0, 1, 0, 0]) | Some(b"true") | Some(b"OTTO")),
        "woff" => head.get(..4) == Some(b"wOFF"),
        "woff2" => head.get(..4) == Some(b"wOF2"),
        _ => return Err(AppError::validation("글꼴 파일(ttf·otf·woff·woff2)만 올릴 수 있습니다", None)),
    };
    if ok {
        Ok(ext)
    } else {
        Err(AppError::validation(
            "글꼴 파일이 아니거나 읽을 수 없는 형식입니다 (글꼴 묶음 .ttc는 지원하지 않음)",
            None,
        ))
    }
}

/// 목록에 보일 이름: 파일 이름에서 확장자를 뺀 것
fn display_name(src: &Path) -> String {
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("글꼴");
    let name: String = stem.chars().filter(|c| !c.is_control()).take(MAX_NAME_CHARS).collect();
    if name.trim().is_empty() {
        "글꼴".into()
    } else {
        name
    }
}

/// 고른 파일들을 `dir`(= fonts/)에 복사하고 목록에 넣을 항목을 돌려준다. 하나라도 검사에 걸리면 아무것도 복사하지 않는다
pub fn import_fonts(dir: &Path, srcs: &[PathBuf]) -> Result<Vec<UserFont>> {
    if srcs.len() > MAX_PER_CALL {
        return Err(AppError::validation(format!("한 번에 {MAX_PER_CALL}개까지 올릴 수 있습니다"), None));
    }
    // 먼저 전부 검사 (중간에 실패해도 복사된 파일이 남지 않게)
    let mut checked = Vec::new();
    for src in srcs {
        let meta = std::fs::metadata(src)?;
        if !meta.is_file() {
            return Err(AppError::validation("파일이 아닙니다", None));
        }
        if meta.len() > MAX_FONT_BYTES {
            return Err(AppError::validation(
                format!("글꼴 파일이 너무 큽니다 ({}MB까지)", MAX_FONT_BYTES / 1024 / 1024),
                None,
            ));
        }
        let mut head = [0u8; 4];
        {
            use std::io::Read;
            let n = std::fs::File::open(src)?.read(&mut head)?;
            if n < 4 {
                return Err(AppError::validation("글꼴 파일이 아니거나 읽을 수 없는 형식입니다", None));
            }
        }
        let ext = check_header(src.extension().and_then(|e| e.to_str()).unwrap_or(""), &head)?;
        checked.push((src, ext));
    }
    std::fs::create_dir_all(dir)?;
    let mut added = Vec::new();
    for (src, ext) in checked {
        let dest = dir.join(format!("{}.{ext}", uuid::Uuid::new_v4()));
        std::fs::copy(src, &dest)?;
        added.push(UserFont {
            path: dest.to_string_lossy().into_owned(),
            name: display_name(src),
        });
    }
    Ok(added)
}

/// 앱이 복사해 둔 파일(fonts/ 바로 아래)이면 지운다. 다른 곳의 파일은 건드리지 않는다
pub fn remove_font_file(dir: &Path, path: &str) {
    let p = PathBuf::from(path);
    if p.parent() == Some(dir) {
        let _ = std::fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sc-fonts-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn header_must_match_extension() {
        assert_eq!(check_header("TTF", &[0, 1, 0, 0]).unwrap(), "ttf");
        assert_eq!(check_header("otf", b"OTTO").unwrap(), "otf");
        assert_eq!(check_header("otf", &[0, 1, 0, 0]).unwrap(), "otf");
        assert_eq!(check_header("woff", b"wOFF").unwrap(), "woff");
        assert_eq!(check_header("woff2", b"wOF2").unwrap(), "woff2");
        // 확장자와 머리가 어긋나거나 글꼴이 아닌 것, 글꼴 묶음(ttc)
        assert!(check_header("woff2", b"wOFF").is_err());
        assert!(check_header("ttf", b"%PDF").is_err());
        assert!(check_header("ttf", b"ttcf").is_err());
        assert!(check_header("png", &[0, 1, 0, 0]).is_err());
        assert!(check_header("ttf", &[0, 1]).is_err());
    }

    #[test]
    fn import_copies_valid_fonts_and_names_them() {
        let base = tmp("ok");
        let src = base.join("My Font.ttf");
        std::fs::write(&src, [0u8, 1, 0, 0, 9, 9, 9, 9]).unwrap();
        let dir = base.join(FONTS_DIR);
        let added = import_fonts(&dir, std::slice::from_ref(&src)).unwrap();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].name, "My Font");
        assert!(added[0].path.ends_with(".ttf"));
        assert!(Path::new(&added[0].path).is_file());
        assert!(src.is_file(), "원본은 그대로");
        std::fs::remove_dir_all(base).ok();
    }

    #[test]
    fn one_bad_file_copies_nothing() {
        let base = tmp("bad");
        let good = base.join("a.woff2");
        let bad = base.join("b.ttf");
        std::fs::write(&good, b"wOF2....").unwrap();
        std::fs::write(&bad, b"not a font").unwrap();
        let dir = base.join(FONTS_DIR);
        assert!(import_fonts(&dir, &[good, bad]).is_err());
        assert!(
            !dir.exists() || std::fs::read_dir(&dir).unwrap().next().is_none(),
            "검사에서 걸리면 복사하지 않음"
        );
        std::fs::remove_dir_all(base).ok();
    }

    #[test]
    fn count_limit() {
        let base = tmp("limit");
        let dir = base.join(FONTS_DIR);
        let many: Vec<PathBuf> = (0..MAX_PER_CALL + 1).map(|i| base.join(format!("{i}.ttf"))).collect();
        assert!(import_fonts(&dir, &many).is_err());
        std::fs::remove_dir_all(base).ok();
    }

    #[test]
    fn remove_only_touches_the_fonts_dir() {
        let base = tmp("rm");
        let dir = base.join(FONTS_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        let mine = dir.join("x.ttf");
        let other = base.join("y.ttf");
        std::fs::write(&mine, b"x").unwrap();
        std::fs::write(&other, b"y").unwrap();
        remove_font_file(&dir, other.to_str().unwrap());
        assert!(other.is_file(), "fonts/ 밖의 파일은 지우지 않음");
        remove_font_file(&dir, mine.to_str().unwrap());
        assert!(!mine.exists());
        std::fs::remove_dir_all(base).ok();
    }
}
