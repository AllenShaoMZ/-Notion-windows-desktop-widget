const { invoke } = window.__TAURI__.core;

const QUADRANTS = ["重要紧急", "重要不紧急", "不重要紧急", "不重要不紧急"];
const STATUS_OPTIONS = ["未开始", "进行中", "已完成"];

function toNotionAppUrl(url) {
  if (!url) return url;
  if (url.startsWith("notion://")) return url;
  return url.replace(/^https?:\/\//, "notion://");
}

function renderOneTask(ul, task) {
  const li = document.createElement("li");
  li.className = "task-item";
  li.dataset.id = task.id;

  const nameEl = document.createElement("span");
  nameEl.className = "task-name";
  nameEl.textContent = task.title;

  // 点击任务标题，优先用 notion:// 协议在桌面版 Notion 中打开
  if (task.url) {
    nameEl.addEventListener("click", (e) => {
      e.stopPropagation();
      const appUrl = toNotionAppUrl(task.url);
      // 通过后端命令在系统中打开 notion:// 链接，交给 Notion 桌面版
      invoke("open_notion_url", { url: appUrl }).catch(console.error);
    });
  }

  const statusWrap = document.createElement("div");
  statusWrap.className = "task-status-wrap";
  const statusSelect = document.createElement("select");
  statusSelect.className = "task-status-select";
  STATUS_OPTIONS.forEach((opt) => {
    const o = document.createElement("option");
    o.value = opt;
    o.textContent = opt;
    if (opt === (task.status || "")) o.selected = true;
    statusSelect.appendChild(o);
  });
  statusSelect.addEventListener("change", async () => {
    const value = statusSelect.value;
    try {
      await invoke("set_task_status", { id: task.id, statusName: value });
    } catch (e) {
      console.error(e);
    }
  });
  statusWrap.appendChild(statusSelect);

  li.appendChild(nameEl);
  li.appendChild(statusWrap);
  ul.appendChild(li);
}

function renderQuadrants(tasks) {
  const byTag = {};
  QUADRANTS.forEach((q) => {
    byTag[q] = [];
  });
  byTag["未分类"] = [];

  for (const task of tasks || []) {
    const tag = task.tag && QUADRANTS.includes(task.tag) ? task.tag : "未分类";
    if (!byTag[tag]) byTag[tag] = [];
    byTag[tag].push(task);
  }

  QUADRANTS.forEach((tag) => {
    const ul = document.getElementById("q-" + tag);
    if (!ul) return;
    ul.innerHTML = "";
    const list = byTag[tag] || [];
    list.forEach((t) => renderOneTask(ul, t));
    if (tag === "重要不紧急") {
      (byTag["未分类"] || []).forEach((t) => renderOneTask(ul, t));
    }
  });
}

async function loadTasks() {
  const errorBox = document.getElementById("error-box");
  errorBox.style.display = "none";

  try {
    const tasks = await invoke("get_tasks");
    renderQuadrants(tasks);
  } catch (e) {
    console.error(e);
    const msg = typeof e === "string" ? e : e?.message || JSON.stringify(e);
    errorBox.textContent = "加载任务失败：" + msg;
    errorBox.style.display = "block";
  }
}

function openAddModal(tag) {
  const modal = document.getElementById("add-modal");
  const titleEl = document.getElementById("modal-quadrant-title");
  const inputTitle = document.getElementById("new-task-title");
  const inputStatus = document.getElementById("new-task-status");
  titleEl.textContent = "新增任务 · " + tag;
  inputTitle.value = "";
  inputTitle.placeholder = "输入任务名称";
  inputStatus.value = "未开始";
  document.getElementById("new-task-date").value = "";
  modal.classList.add("show");
  inputTitle.focus();
  modal._currentTag = tag;
}

function closeAddModal() {
  document.getElementById("add-modal").classList.remove("show");
}

async function confirmAddTask() {
  const modal = document.getElementById("add-modal");
  const tag = modal._currentTag;
  const title = document.getElementById("new-task-title").value.trim();
  const statusName = document.getElementById("new-task-status").value.trim() || "未开始";
  const dateInput = document.getElementById("new-task-date").value; // YYYY-MM-DD 或空
  if (!title) return;
  closeAddModal();
  try {
    await invoke("create_task", {
      title,
      tag,
      statusName,
      dateIso: dateInput || null,
    });
    await loadTasks();
  } catch (e) {
    console.error(e);
    const errorBox = document.getElementById("error-box");
    const msg = typeof e === "string" ? e : e?.message || JSON.stringify(e);
    errorBox.textContent = "新增任务失败：" + msg;
    errorBox.style.display = "block";
  }
}

function openSettingsModal() {
  const overlay = document.getElementById("settings-modal");
  const errorBox = document.getElementById("error-box");
  errorBox.style.display = "none";

  invoke("get_config_safe")
    .then((cfg) => {
      document.getElementById("cfg-database-id").value = cfg.database_id || "";
      document.getElementById("cfg-title-property").value = cfg.title_property || "";
      document.getElementById("cfg-tags-property").value = cfg.tags_property || "";
      document.getElementById("cfg-done-property").value = cfg.done_property || "";
      document.getElementById("cfg-date-property").value = cfg.date_property || "";
      overlay.classList.add("show");
    })
    .catch((e) => {
      console.error(e);
      const msg = typeof e === "string" ? e : e?.message || JSON.stringify(e);
      errorBox.textContent = "读取配置失败：" + msg;
      errorBox.style.display = "block";
    });
}

function closeSettingsModal() {
  document.getElementById("settings-modal").classList.remove("show");
}

async function saveSettings() {
  const overlay = document.getElementById("settings-modal");
  const errorBox = document.getElementById("error-box");
  errorBox.style.display = "none";

  const databaseId = document.getElementById("cfg-database-id").value.trim();
  const titleProperty = document.getElementById("cfg-title-property").value.trim();
  const tagsProperty = document.getElementById("cfg-tags-property").value.trim();
  const doneProperty = document.getElementById("cfg-done-property").value.trim();
  const dateProperty = document.getElementById("cfg-date-property").value.trim();

  if (!databaseId || !titleProperty || !doneProperty) {
    errorBox.textContent = "数据库 ID、标题列名、状态列名不能为空";
    errorBox.style.display = "block";
    return;
  }

  try {
    await invoke("update_config_safe", {
      databaseId,
      titleProperty,
      doneProperty,
      tagsProperty: tagsProperty || null,
      dateProperty: dateProperty || null,
    });
    overlay.classList.remove("show");
    await loadTasks();
  } catch (e) {
    console.error(e);
    const msg = typeof e === "string" ? e : e?.message || JSON.stringify(e);
    errorBox.textContent = "保存配置失败：" + msg;
    errorBox.style.display = "block";
  }
}

window.addEventListener("DOMContentLoaded", () => {
  loadTasks();

  document.querySelectorAll(".btn-add").forEach((btn) => {
    btn.addEventListener("click", () => {
      const tag = btn.getAttribute("data-tag");
      openAddModal(tag);
    });
  });

  document.getElementById("modal-cancel").addEventListener("click", closeAddModal);
  document.getElementById("modal-confirm").addEventListener("click", confirmAddTask);

  document.getElementById("new-task-title").addEventListener("keydown", (e) => {
    if (e.key === "Enter") confirmAddTask();
  });

  // 关闭按钮
  const closeBtn = document.getElementById("btn-close");
  if (closeBtn) {
    closeBtn.addEventListener("click", () => {
      try {
        invoke("quit_app");
      } catch (e) {
        window.close();
      }
    });
  }

  const settingsBtn = document.getElementById("btn-settings");
  if (settingsBtn) {
    settingsBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      openSettingsModal();
    });
  }

  document.getElementById("settings-cancel").addEventListener("click", closeSettingsModal);
  document.getElementById("settings-save").addEventListener("click", saveSettings);
});
