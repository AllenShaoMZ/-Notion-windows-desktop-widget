use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::{Manager, Window};
use std::process::Command;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct NotionApiConfig {
    token: String,
    database_id: String,
    title_property: String,
    done_property: String,
    /// Tags 列名，用于四象限分类（如 "Tags"）
    #[serde(alias = "Tags_property", default)]
    tags_property: Option<String>,
    /// 可选：日期/截止日期列名（如 "日期"），新建任务时可写入
    #[serde(default)]
    date_property: Option<String>,
}

/// 只暴露给前端的安全配置（不包含 token）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublicConfig {
    pub database_id: String,
    pub title_property: String,
    pub done_property: String,
    pub tags_property: Option<String>,
    pub date_property: Option<String>,
}

/// 任务结构：含象限标签与状态
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub title: String,
    /// 是否已完成（状态=已完成）
    pub done: bool,
    /// 状态文案：未开始 / 进行中 / 已完成
    pub status: String,
    /// 象限标签：重要紧急 / 重要不紧急 / 不重要紧急 / 不重要不紧急
    pub tag: String,
    /// 对应 Notion 页面 URL，用于在浏览器中打开
    pub url: String,
}

/// 解析配置文件路径（开发 & 打包场景通用）
fn resolve_config_path() -> Result<PathBuf, String> {
    // 开发 / dev 模式下：优先从当前工作目录及上级 src-tauri 目录读取
    let mut tried_paths = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        // 1) 直接在当前目录下找（tauri dev 时，cwd 一般是 src-tauri）
        let direct = cwd.join("notion-api-config.json");
        tried_paths.push(direct.clone());
        if direct.exists() {
            return Ok(direct);
        }

        // 2) 假设 cwd 是项目根目录：src-tauri/notion-api-config.json
        let dev_path = cwd.join("src-tauri").join("notion-api-config.json");
        tried_paths.push(dev_path.clone());
        if dev_path.exists() {
            return Ok(dev_path);
        }
    }

    // 打包后：尝试从可执行文件同目录读取
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let prod_path = dir.join("notion-api-config.json");
            tried_paths.push(prod_path.clone());
            if prod_path.exists() {
                return Ok(prod_path);
            }
        }
    }

    Err(format!(
        "未找到 notion-api-config.json，已尝试路径：{:?}",
        tried_paths
    ))
}

fn load_config() -> Result<NotionApiConfig, String> {
    let path = resolve_config_path()?;

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取配置文件失败: {e}"))?;

    serde_json::from_str::<NotionApiConfig>(&content)
        .map_err(|e| format!("解析配置 JSON 失败: {e}"))
}

/// 读取当前配置（不返回 token）
#[tauri::command]
fn get_config_safe() -> Result<PublicConfig, String> {
    let cfg = load_config()?;
    Ok(PublicConfig {
        database_id: cfg.database_id,
        title_property: cfg.title_property,
        done_property: cfg.done_property,
        tags_property: cfg.tags_property,
        date_property: cfg.date_property,
    })
}

/// 更新除 token 外的配置字段
#[tauri::command]
fn update_config_safe(
    database_id: String,
    title_property: String,
    done_property: String,
    tags_property: Option<String>,
    date_property: Option<String>,
) -> Result<(), String> {
    let path = resolve_config_path()?;
    let mut cfg = load_config()?;

    cfg.database_id = database_id;
    cfg.title_property = title_property;
    cfg.done_property = done_property;
    cfg.tags_property = tags_property;
    cfg.date_property = date_property;

    let content = serde_json::to_string_pretty(&cfg)
        .map_err(|e| format!("序列化配置失败: {e}"))?;

    std::fs::write(path, content).map_err(|e| format!("写入配置失败: {e}"))?;
    Ok(())
}

/// 从 Notion Database 读取任务列表
#[tauri::command]
async fn get_tasks() -> Result<Vec<Task>, String> {
    let cfg = load_config()?;

    #[derive(Deserialize)]
    struct TitleText {
        plain_text: String,
    }
    #[derive(Deserialize)]
    struct TitlePropertyValue {
        title: Vec<TitleText>,
    }
    #[derive(Deserialize)]
    struct StatusName {
        name: String,
    }
    #[derive(Deserialize)]
    struct StatusPropertyValue {
        status: Option<StatusName>,
    }
    #[derive(Deserialize)]
    struct MultiSelectOption {
        name: String,
    }
    #[derive(Deserialize)]
    struct MultiSelectPropertyValue {
        multi_select: Vec<MultiSelectOption>,
    }

    let tags_key = cfg
        .tags_property
        .as_deref()
        .unwrap_or("Tags");

    let client = reqwest::Client::new();
    let url = format!(
        "https://api.notion.com/v1/databases/{}/query",
        cfg.database_id
    );

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Notion-Version", "2022-06-28")
        .header("Content-Type", "application/json")
        .body("{}")
        .send()
        .await
        .map_err(|e| format!("请求 Notion 失败: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "<无法读取响应体>".to_string());
        return Err(format!("Notion 返回错误状态: {status}, 详情: {body}"));
    }

    let raw = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("解析 Notion 响应失败: {e}"))?;

    let mut tasks = Vec::new();

    if let Some(results) = raw.get("results").and_then(|r| r.as_array()) {
        for page in results {
            let id = page
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let url = page
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let props = page.get("properties").cloned().unwrap_or_default();

            let title_val = props
                .get(&cfg.title_property)
                .cloned()
                .unwrap_or(serde_json::json!({ "title": [] }));
            let status_val = props
                .get(&cfg.done_property)
                .cloned()
                .unwrap_or(serde_json::json!({ "status": null }));
            let tags_val = props
                .get(tags_key)
                .cloned()
                .unwrap_or(serde_json::json!({ "multi_select": [] }));

            let title_struct: TitlePropertyValue =
                serde_json::from_value(title_val).unwrap_or(TitlePropertyValue { title: vec![] });
            let status_struct: StatusPropertyValue =
                serde_json::from_value(status_val).unwrap_or(StatusPropertyValue { status: None });
            let tags_struct: MultiSelectPropertyValue =
                serde_json::from_value(tags_val).unwrap_or(MultiSelectPropertyValue { multi_select: vec![] });

            let title = title_struct
                .title
                .get(0)
                .map(|t| t.plain_text.clone())
                .unwrap_or_else(|| "<未命名>".to_string());

            let status_name = status_struct
                .status
                .map(|s| s.name)
                .unwrap_or_else(|| String::new());

            let tag = tags_struct
                .multi_select
                .get(0)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| "未分类".to_string());

            let done = status_name == "已完成";

            tasks.push(Task {
                id,
                title,
                done,
                status: status_name,
                tag,
                url,
            });
        }
    }

    Ok(tasks)
}

/// 更新任务状态（未开始 / 进行中 / 已完成）
#[tauri::command]
async fn set_task_status(id: String, status_name: String) -> Result<(), String> {
    let cfg = load_config()?;

    let client = reqwest::Client::new();
    let url = format!("https://api.notion.com/v1/pages/{id}");

    let body = serde_json::json!({
        "properties": {
            cfg.done_property.clone(): {
                "status": { "name": status_name }
            }
        }
    });

    let resp = client
        .patch(&url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Notion-Version", "2022-06-28")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("更新 Notion 失败: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "<无法读取响应体>".to_string());
        return Err(format!("Notion 更新失败，状态: {status}, 详情: {body}"));
    }

    Ok(())
}

/// 在 Notion 数据库中新增一条任务（指定标题、象限标签、初始状态；可选日期）
#[tauri::command]
async fn create_task(
    title: String,
    tag: String,
    status_name: String,
    date_iso: Option<String>,
) -> Result<(), String> {
    let cfg = load_config()?;
    let tags_key = cfg.tags_property.as_deref().unwrap_or("Tags");

    let client = reqwest::Client::new();
    let url = "https://api.notion.com/v1/pages";

    let mut properties = serde_json::json!({
        cfg.title_property.clone(): {
            "title": [{ "text": { "content": title } }]
        },
        cfg.done_property.clone(): {
            "status": { "name": status_name }
        },
        tags_key: {
            "multi_select": [{ "name": tag }]
        }
    });

    if let (Some(date_key), Some(ref iso)) = (cfg.date_property.as_deref(), date_iso) {
        if !date_key.is_empty() && !iso.is_empty() {
            let obj = properties.as_object_mut().unwrap();
            obj.insert(
                date_key.to_string(),
                serde_json::json!({ "date": { "start": iso } }),
            );
        }
    }

    let body = serde_json::json!({
        "parent": { "database_id": cfg.database_id },
        "properties": properties
    });

    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Notion-Version", "2022-06-28")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("创建任务失败: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "<无法读取响应体>".to_string());
        return Err(format!("Notion 创建失败，状态: {status}, 详情: {body}"));
    }

    Ok(())
}

/// 预留：点击穿透（当前未实现，仅占位）
#[tauri::command]
fn set_click_through(_window: Window, _enabled: bool) {
    // TODO: 未来在 Windows 下通过原生 API 实现点击穿透
}

/// 预留：调整透明度（当前未实现，仅占位）
#[tauri::command]
fn set_opacity(_window: Window, _opacity: f64) {
    // TODO: 未来接入 Tauri 2 对应 API 或自定义扩展
}

/// 预留：切换可见性（当前未实现，仅占位）
#[tauri::command]
fn toggle_visible(_window: Window) {
    // TODO: 未来在这里实现显示/隐藏逻辑
}

/// 预留：锁定到桌面角落（当前未实现，仅占位）
#[tauri::command]
fn lock_to_desktop_corner(_window: Window) {
    // TODO: 未来实现固定到屏幕某个角的位置
}

/// 在系统中打开 Notion 页面 URL（包括 notion:// 协议）
#[tauri::command]
fn open_notion_url(url: String) -> Result<(), String> {
    // Windows 下通过 `start` 命令让系统用默认程序处理 URL
    // start "" "<url>"
    Command::new("cmd")
        .args(["/C", "start", "", &url])
        .spawn()
        .map_err(|e| format!("启动 Notion 链接失败: {e}"))?;
    Ok(())
}

/// 退出整个应用（用于前端自定义关闭按钮）
#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_config_safe,
            update_config_safe,
            open_notion_url,
            get_tasks,
            set_task_status,
            create_task,
            set_click_through,
            set_opacity,
            toggle_visible,
            lock_to_desktop_corner,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running Notion Widget application");
}
