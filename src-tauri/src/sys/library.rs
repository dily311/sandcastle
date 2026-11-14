use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRoot {
    pub folders: HashMap<String, FolderNode>,
    pub root_files: Vec<Value>,
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderNode {
    pub name: String,
    pub folders: HashMap<String, FolderNode>,
    pub files: Vec<Value>,
}

impl FolderNode {
    /// 폴더 트리 구현을 위한 재귀함수
    pub fn insert_file(&mut self, folder_parts: &[&str], file: Value) {
        if folder_parts.is_empty() {
            self.files.push(file);
            return;
        }

        let current_folder = folder_parts[0];
        let remaining_parts = &folder_parts[1..];

        let next_node = self
            .folders
            .entry(current_folder.to_string())
            .or_insert_with(|| FolderNode {
                name: current_folder.to_string(),
                folders: HashMap::new(),
                files: Vec::new(),
            });

        // 재귀 호출을 통해 다음 레벨로 가변 참조를 안전하게 넘김
        next_node.insert_file(remaining_parts, file);
    }
}

/// 폴더 트리
pub fn grouped_by_folder(file_list: Vec<(String, Value)>, selected_folder: Option<&str>) -> LibraryRoot {
    let mut root_tree: HashMap<String, FolderNode> = HashMap::new();
    let mut root_files: Vec<Value> = Vec::new();

    let root_path = selected_folder
        .filter(|f| !f.is_empty())
        .map(|f| format!("{}\\", f))
        .unwrap_or_default();

    for (path, file) in file_list {
        let path = if !root_path.is_empty() {
            path.strip_prefix(&root_path).unwrap_or(&path)
        } else {
            &path
        };

        let parts: Vec<&str> = path.split('\\').collect();
        if parts.is_empty() {
            continue;
        }

        let folder_parts = &parts[..parts.len() - 1];

        if folder_parts.is_empty() {
            root_files.push(file);
            continue;
        }

        // 재귀 메서드 호출/ 복잡한 루프 대여 문제를 한 줄로 해결
        let first_folder = folder_parts[0];
        let remaining_parts = &folder_parts[1..];

        root_tree
            .entry(first_folder.to_string())
            .or_insert_with(|| FolderNode {
                name: first_folder.to_string(),
                folders: HashMap::new(),
                files: Vec::new(),
            })
            .insert_file(remaining_parts, file.clone());
    }

    LibraryRoot {
        folders: root_tree,
        root_files,
    }
}
