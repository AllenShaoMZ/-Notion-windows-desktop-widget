# Notion 四象限桌面小组件（Tauri 2）

一个基于 **Tauri 2 + WebView2** 的 Windows 桌面 Widget，用于在桌面上以“四象限”（重要/紧急矩阵）的方式展示并管理 Notion Database 里的任务。

> 设计目标：  
> - 嵌入 Notion 数据（Database），而不是重造一套任务系统；  
> - 像桌面小组件一样轻量、半透明、不打扰当前应用；  
> - 一次配置后，日常使用基本无需碰命令行。

---

## 功能特性

- **四象限任务展示**
  - 按 Notion Database 中的 `Tags` 列，将任务分成四个象限：
    - 重要紧急
    - 重要不紧急
    - 不重要紧急
    - 不重要不紧急
  - 未设置 Tags 的任务会归入“未分类”，当前合并显示在「重要不紧急」象限中。

- **任务信息与操作**
  - 每条任务展示：**任务名称 + 状态**（未开始 / 进行中 / 已完成）。
  - 状态可直接在 Widget 中通过下拉框修改，实时写回 Notion 的状态列。
  - 在任一象限下方点击 **「+ 新增任务」**，可创建新任务：
    - 自动带上当前象限对应的 Tags；
    - 可设置初始状态；
    - 可选设置日期（对应 Notion Database 中的 Date 列）。

- **一键跳转 Notion 桌面应用**
  - 点击任务名称时，优先将 Notion 返回的 `https://...` 链接转换为 `notion://...` 协议；
  - 通过 `cmd /C start` 调用系统默认 URL 处理程序，直接唤起 **Notion 桌面版** 打开对应页面。

- **内置配置面板**
  - 窗口右上角的白色小圆点（齿轮）打开配置弹窗：
    - 数据库 ID（`database_id`）
    - 标题列名（`title_property`）
    - Tags 列名（`tags_property`）
    - 状态列名（`done_property`）
    - 日期列名（`date_property`，可选）
  - 所有修改会写入 `src-tauri/notion-api-config.json`，并立即刷新任务列表。
  - **Notion token 不会出现在 UI 中，只保存在本地配置文件里。**

- **窗口行为与交互**
  - 无边框透明窗口（`decorations: false`），顶部自定义细拖动条。
  - 拖动条右侧两个小圆点：
    - 白色：打开配置面板；
    - 红色：关闭应用（调用 Tauri `app.exit(0)`，彻底退出，不留后台进程）。
  - 半透明毛玻璃背景（`backdrop-filter: blur(...)`），弱化外框，不显突兀。
  - 默认 **不再总是置顶**，行为与普通窗口一致，可以拖到桌面一角当 Widget 使用。

---

## 环境要求

- Windows 10 / 11
- 开发环境需要：
  - [Rust](https://www.rust-lang.org/tools/install)（`rustup` 推荐）
  - Node.js 18+（LTS 即可）
  - Git（可选，用于版本管理）
  - Microsoft Edge WebView2 Runtime（Windows 通常已自带）

---

## 快速开始（开发）

### 1. 克隆项目并安装依赖

```bash
git clone https://github.com/<your-username>/notion-widget.git
cd notion-widget

npm install
```

### 2. 开发模式运行

```bash
npm run tauri dev
```

- 将启动 Tauri dev server，并弹出 Widget 窗口；
- 当你修改前端或后端代码时，会自动重新构建 / 刷新。

---

## Notion 集成配置

本项目通过 **Notion 官方 API** 读取 / 写入任务。第一次运行前需要完成一次集成与配置。

### 1. 创建 Internal Integration

1. 打开浏览器访问：`https://www.notion.so/my-integrations`
2. 点击 **New integration**，选择 **Internal** 类型（只给自己/团队用，不需要复杂的 OAuth）。
3. 填写：
   - **Integration name**：例如 `Desktop Widget`
   - **Associated workspace**：选择你的工作区
4. 创建完成后，在集成详情页复制：
   - **Internal Integration Token**（以 `secret_` 开头）

> ⚠ 安全提示：请不要将该 token 提交到公开 Git 仓库或截图分享。

### 2. 准备 Notion Database

1. 在 Notion 中找到要展示的 **Database**（不是单独 Page）。
2. 确保包含以下列（列名可以自定义，但要与配置一致）：
   - **标题列**（Title）：例如 `任务`（Notion 默认列名是 `Name`）
   - **Tags 列**（Multi-select）：例如 `Tags`
   - **状态列**（Status）：例如 `状态`，候选值建议包含：
     - `未开始`
     - `进行中`
     - `已完成`
   - **日期列**（Date，可选）：例如 `日期` 或 `截止日期`
3. 在数据库右上角菜单中，选择 **「以页面形式打开 / Open as full page」**。
4. 此时浏览器地址类似：

   ```text
   https://www.notion.so/xxxx/xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx?v=...
   ```

   中间那段 32 字符（`xxxxxxxx...`）就是该数据库的 **`database_id`**。

### 3. 授权集成访问该 Database

1. 在该 Database 页面右上角点击 **Share / 共享**。
2. 在弹出的对话框中，将刚创建的 Integration（如 `Desktop Widget`）添加进来。
3. 确认集成对该 Database 有访问权限（否则 API 会返回 400 / 403）。

### 4. 配置 `src-tauri/notion-api-config.json`

在项目的 `src-tauri` 目录下，有一个 `notion-api-config.json` 文件。其格式大致如下：

```json
{
  "token": "secret_xxx你的Integration密钥xxx",
  "database_id": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
  "title_property": "任务",
  "tags_property": "Tags",
  "done_property": "状态",
  "date_property": "日期"
}
```

- `token`：第 1 步中复制的 Notion Internal Integration Token。
- `database_id`：第 2 步中从 Database URL 中间截取的 ID。
- `title_property`：标题列列名（如 `任务`、`Name` 等）。
- `tags_property`：Tags 列列名（如 `Tags`）。
- `done_property`：状态列列名（如 `状态`）。
- `date_property`：日期列列名（可选；不需要可以设为空字符串或删掉字段）。

> 推荐：将 `src-tauri/notion-api-config.json` 加入 `.gitignore`，避免误推到远程仓库。

### 5. 运行并验证

配置正确后，重新运行：

```bash
npm run tauri dev
```

若一切正常，Widget 中会显示目标数据库中的任务；如果有错误，会在底部红色错误条中提示（例如未授权、列名不匹配等）。

---

## 打包与分发

### 1. 生成可执行文件（.exe）

```bash
cd notion-widget
npm run tauri build
```

构建成功后，会在：

```text
src-tauri/target/release/notion-widget.exe
```

生成一个可直接运行的 Windows 可执行文件。

### 2. 使用方式

- 双击 `notion-widget.exe` 启动 Widget。
- 可以对 exe：
  - 发送到桌面快捷方式；
  - 固定到任务栏；
  - 放入 `shell:startup` 目录实现开机自启（可选）。

> 说明：如果 `tauri build` 过程中下载 WiX（用于创建 .msi 安装包）失败，只会影响安装包生成，对 `notion-widget.exe` 无影响。

### 3. 生产环境的配置文件

打包后的 exe 会在其同目录下寻找 `notion-api-config.json`，因此需要将配置文件复制到同一目录，例如：

```text
src-tauri/target/release/
  notion-widget.exe
  notion-api-config.json
```

这样无论在开发还是打包环境，都能使用同样的配置结构。

---

## 使用说明（交互层面）

### 顶部条（拖动条 + 控制按钮）

- 拖动：在顶部窄条空白区域按住拖动，即可移动窗口。
- 配置按钮（白色圆点）：打开 Notion 配置弹窗。
- 关闭按钮（红色圆点）：退出应用（结束进程）。

### 四象限任务区

- 每个象限代表一个 Tags 类别：
  - 重要紧急
  - 重要不紧急
  - 不重要紧急
  - 不重要不紧急
- 已有任务：
  - 左侧显示任务名称；
  - 右侧状态下拉框可修改状态（未开始 / 进行中 / 已完成），自动同步到 Notion。
- 新增任务：
  - 每个象限底部有「+ 新增任务」按钮；
  - 弹出的表单中填写任务名、状态、日期；
  - 保存后自动写入该 Database，对应 Tags 为该象限名。
- 打开 Notion：
  - 点击任务名称，会构造 `notion://...` 链接，通过系统打开 Notion 桌面应用的对应页面。

### 配置弹窗

- 通过右上白色圆点打开。
- 修改 Database ID / 列名等后点击保存：
  - 写回 `notion-api-config.json`；
  - 自动重新加载任务列表。

---

## 安全与隐私

- Notion Integration Token 仅存放在本地 `notion-api-config.json` 中，不在 UI / 日志中直接展示。
- 建议：
  - 将 `src-tauri/notion-api-config.json` 加入 `.gitignore`；
  - 将 GitHub 仓库设为私有，或在公开仓库中使用示例配置文件（不含真实 token）。

---

## TODO / 未来规划

- 支持多数据库配置与快捷切换（例如工作/生活分组）。
- 支持点击穿透模式（只看不挡鼠标，适合作为“透明备忘板”）。
- 支持定时自动刷新（当前为手动刷新或重新打开应用）。
- 更丰富的过滤和排序（按日期 / 状态 / 标签过滤）。

---



MIT License
