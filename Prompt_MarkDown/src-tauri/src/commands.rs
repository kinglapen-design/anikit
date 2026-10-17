use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct FileNode {
    name: String,
    path: String,
    kind: &'static str,
    children: Vec<FileNode>,
}

fn is_md(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

fn scan_dir(dir: &Path) -> Result<Vec<FileNode>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("读取目录失败 {}: {e}", dir.display()))?;
    let mut nodes: Vec<FileNode> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            let children = scan_dir(&path)?;
            if children.is_empty() {
                continue;
            }
            nodes.push(FileNode {
                name,
                path: path.to_string_lossy().to_string(),
                kind: "dir",
                children,
            });
        } else if is_md(&path) {
            nodes.push(FileNode {
                name,
                path: path.to_string_lossy().to_string(),
                kind: "file",
                children: Vec::new(),
            });
        }
    }
    nodes.sort_by(|a, b| {
        let dir_first = (b.kind == "dir").cmp(&(a.kind == "dir"));
        dir_first.then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(nodes)
}

#[tauri::command]
pub fn scan_markdown(root: String) -> Result<FileNode, String> {
    let path = PathBuf::from(&root);
    if !path.is_dir() {
        return Err(format!("不是文件夹: {root}"));
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root.clone());
    Ok(FileNode {
        name,
        path: root,
        kind: "dir",
        children: scan_dir(&path)?,
    })
}

#[tauri::command]
pub fn read_markdown(path: String) -> Result<String, String> {
    let bytes = fs::read(&path).map_err(|e| format!("读取文件失败 {path}: {e}"))?;
    String::from_utf8(bytes).map_err(|_| format!("文件不是有效 UTF-8: {path}"))
}

#[tauri::command]
pub fn write_markdown(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| format!("写入文件失败 {path}: {e}"))
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect::<String>()
        .trim()
        .to_string()
}

#[tauri::command]
pub fn create_markdown(dir: String, name: String) -> Result<String, String> {
    let mut stem = sanitize_name(&name);
    if stem.is_empty() {
        stem = "未命名".to_string();
    }
    if stem.to_lowercase().ends_with(".md") {
        stem.truncate(stem.len() - 3);
    }
    let dir_path = PathBuf::from(&dir);
    if !dir_path.is_dir() {
        return Err(format!("目录不存在: {dir}"));
    }
    for i in 1..1000 {
        let file_name = if i == 1 {
            format!("{stem}.md")
        } else {
            format!("{stem}_{i}.md")
        };
        let candidate = dir_path.join(&file_name);
        if !candidate.exists() {
            fs::write(&candidate, "").map_err(|e| format!("新建文件失败: {e}"))?;
            return Ok(candidate.to_string_lossy().to_string());
        }
    }
    Err("同名文件过多".to_string())
}

// ===== 模板配置（exe 旁 templates/ 目录，首运行播种默认可改） =====

#[derive(Serialize)]
pub struct TemplateFile {
    group: &'static str,
    name: String,
    path: String,
}

fn templates_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("templates")
}

fn seed(path: &Path, content: &str) -> Result<(), String> {
    if !path.exists() {
        fs::write(path, content).map_err(|e| format!("模板播种失败 {}: {e}", path.display()))?;
    }
    Ok(())
}

fn seed_templates(root: &Path) -> Result<(), String> {
    let docs = root.join("docs");
    let menus = root.join("menus");
    fs::create_dir_all(&docs).map_err(|e| format!("创建模板目录失败: {e}"))?;
    fs::create_dir_all(&menus).map_err(|e| format!("创建模板目录失败: {e}"))?;
    seed(&docs.join("分镜提示词.md"), include_str!("../defaults/docs/分镜提示词.md"))?;
    seed(&menus.join("机位.md"), include_str!("../defaults/menus/机位.md"))?;
    seed(&menus.join("仰俯.md"), include_str!("../defaults/menus/仰俯.md"))?;
    seed(&menus.join("摄影机类型.md"), include_str!("../defaults/menus/摄影机类型.md"))?;
    seed(&menus.join("运镜.md"), include_str!("../defaults/menus/运镜.md"))?;
    seed(&root.join("chains.json"), include_str!("../defaults/chains.json"))?;
    Ok(())
}

fn collect_md(dir: &Path, group: &'static str, out: &mut Vec<TemplateFile>) -> Result<(), String> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_md(&path) {
            continue;
        }
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        out.push(TemplateFile {
            group,
            name,
            path: path.to_string_lossy().to_string(),
        });
    }
    Ok(())
}

#[tauri::command]
pub fn template_files() -> Result<Vec<TemplateFile>, String> {
    let root = templates_root();
    seed_templates(&root)?;
    let mut out = Vec::new();
    collect_md(&root.join("docs"), "docs", &mut out)?;
    collect_md(&root.join("menus"), "menus", &mut out)?;
    let chains = root.join("chains.json");
    if chains.exists() {
        out.push(TemplateFile {
            group: "chains",
            name: "规则链配置".to_string(),
            path: chains.to_string_lossy().to_string(),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}
