# 轻量快速粘贴面板(Quick Paste Panel)设计

- 日期:2026-06-21
- 分支:`feat/quick-paste-panel`
- 状态:待实现

## 1. 目标

为 XCopy 增加一个类似 PowerToys Win+V 的**轻量快速粘贴面板**:用户在任何应用打字时,双击 Ctrl 唤起一个跟随鼠标位置的小窗口,从最近的文本/链接历史中选一条,内容直接粘贴到当前光标位置,无需再按 Ctrl+V。

这是对现有主窗口(Ctrl+Shift+V 唤起、点条目仅复制回剪贴板)的补充,定位为"高频快速粘贴"场景的轻量入口。

## 2. 核心交互流程

1. 用户在任意应用打字,**双击 Ctrl**(两次 keydown 间隔在 200-400ms 内)。
2. 后端低级键盘钩子检测到双击 → 记录当前前台窗口 HWND(目标应用)与鼠标坐标。
3. 在鼠标位置弹出无边框、置顶、轻量的 `quick-paste` 窗口(约 320×360),显示最近 N 条文本/链接。
4. 用户点击或键盘选中某条 → 面板隐藏 → 内容写入剪贴板 → 激活目标窗口 → 发送 Ctrl+V → 内容出现在光标处。
5. 按 Esc 或面板失焦自动隐藏。

## 3. 关键决策(已与用户确认)

| 决策点 | 选择 | 备注 |
|--------|------|------|
| 选中后行为 | 直接粘贴到光标 | 而非仅复制回剪贴板 |
| 粘贴注入技术 | 写剪贴板 + 模拟 Ctrl+V | 兼容性最好;会覆盖当前剪贴板内容(可接受,与 PowerToys 一致) |
| 面板弹出位置 | 跟随鼠标位置 | 实现可靠,鼠标通常在打字区域附近 |
| 唤起方式 | 独立手势:双击 Ctrl | 不复用主窗口快捷键 |
| 唤起检测技术 | 低级键盘钩子 WH_KEYBOARD_LL | global-shortcut 插件不支持双击手势 |
| 双击键 | Ctrl | 最常用、最自然 |
| 防误触 | 时间窗 200-400ms | 两次 Ctrl keydown 间隔在此区间算双击 |
| 面板内容范围 | 仅文本/链接 | 不含图片 |
| 面板入口 | 独立 standalone HTML | 参考 `preview.html`,与主 bundle 解耦 |

## 4. 架构与组件

### 4.1 后端(Rust)

**新增模块 `src-tauri/src/quick_paste.rs`**,封装:
- 双击检测状态机(上次 Ctrl keydown 时间戳)
- 面板唤起逻辑(记录 HWND + 鼠标坐标 + 计算 clamp 后位置 + 显示窗口)
- 粘贴注入逻辑(写剪贴板 + SetForegroundWindow + SendInput Ctrl+V)

**新增模块 `src-tauri/src/hotkey_hook.rs`**(或并入 quick_paste.rs):
- `SetWindowsHookExW(WH_KEYBOARD_LL, ...)` 安装低级键盘钩子
- 钩子回调里:拦截 `VK_CONTROL` 的 keydown,做双击判定
- 判定命中 → 通过 `AppHandle` 在主线程显示 quick-paste 面板
- 双击判定只看 keydown 时间窗,不消费按键事件(返回 `CallNextHookEx` 透传),保证不影响正常 Ctrl 使用

**新增 Tauri 命令**:
- `show_quick_paste_panel()` — 钩子命中时调用(也可前端调试用)。记录目标 HWND + 鼠标坐标,定位并显示窗口。
- `paste_from_quick_paste(content: String)` — 前端选中条目后调用。执行写剪贴板 → 激活目标 → Ctrl+V → 隐藏面板。

**历史数据获取**:复用现有 `get_history` 命令,前端传入 `ClipboardFilter` 限定 `text`/`link` 类型。不新增查询命令。

**窗口声明**:在 `tauri.conf.json` 的 `app.windows` 新增 `quick-paste` 窗口配置:
```json
{
  "label": "quick-paste",
  "url": "quick_paste.html",
  "title": "Quick Paste",
  "width": 320,
  "height": 360,
  "resizable": false,
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "visible": false,
  "skipTaskbar": true,
  "shadow": true
}
```

**能力配置** `capabilities/default.json`:
- `windows` 数组加 `"quick-paste"`
- 补权限:`core:window:allow-set-position`、`core:window:allow-show`、`core:window:allow-hide`、`core:window:allow-set-focus`(部分已有)

**AppSettings 扩展**(`app_settings.rs`):
- 新增字段 `quick_paste_enabled: bool`(默认 `true`,控制双击唤起总开关)
- 新增字段 `double_click_interval_ms: u32`(默认 300,双击时间窗,范围 200-400)
- 序列化 camelCase:`quickPasteEnabled`、`doubleClickIntervalMs`
- 更新 `Default`、`normalize_settings`、相关单元测试

**lib.rs setup 改动**:
- `app.manage(AppState {...})` 后安装键盘钩子(传入 AppHandle)
- 钩子需常驻,在 setup 阶段一次性安装,生命周期随进程
- `save_app_settings` 命令:当 `quick_paste_enabled` 变化时,动态安装/卸载钩子

### 4.2 前端(React)

**新增页面 `src/quick_paste.tsx` + `quick_paste.html`**(standalone,参考 `preview.html`/`preview.tsx`):
- 极简列表:每条单行截断 + 类型小圆点 + 来源 + 时间
- 键盘导航:↑↓ 选中、Enter 确认粘贴、Esc 关闭
- 点击条目 → 调 `paste_from_quick_paste(content)`
- 空状态:"暂无文本记录"
- 不含搜索框、不含删除按钮(YAGNI,管理走主窗口)
- 复用 `useClipboardHistory` 的数据获取逻辑(或直接 `invoke('get_history', { filter })`)

### 4.3 数据流

```
双击 Ctrl
 → 钩子回调判定命中(200-400ms 内两次 keydown)
 → 记录 GetForegroundWindow() 的 HWND + GetCursorPos() 坐标
 → 显示 quick-paste 窗口(set_position 到鼠标处 + show + set_focus)
 → 前端渲染最近文本/链接列表
 → 用户选中一条
 → 前端 invoke('paste_from_quick_paste', { content })
 → Rust:
     1. arboard 写剪贴板(文本)
     2. SetForegroundWindow(目标 HWND)
     3. sleep ~80ms 等焦点稳定
     4. SendInput 模拟 Ctrl 按下 + V 按下 + V 释放 + Ctrl 释放(用 scan code)
     5. 隐藏 quick-paste 窗口
 → 内容出现在目标应用光标处
```

## 5. 边界与错误处理

- **焦点抢占时序**:面板显示会抢焦点,粘贴前必须 `SetForegroundWindow` 回目标窗口,且 `SendInput` 前 sleep 50-100ms 等焦点稳定。这是最易出 bug 处,需重点测试。
- **SetForegroundWindow 限制**:Windows 限制只有当前前台窗口的进程才能直接 SetForegroundWindow。绕过手法:`AttachThreadInput` 把目标窗口线程附着到当前线程,或先 `keybd_event(Alt)` 解锁。需在实现里处理。
- **面板出屏**:鼠标在屏幕边缘时,面板位置需边界 clamp(右/下边缘向左/上偏移窗口尺寸)。clamp 逻辑写为纯函数,单测覆盖四角。
- **多显示器**:首版只保证主屏正确。`GetCursorPos` 拿全局坐标,若负坐标/超主屏,clamp 到主屏范围内。
- **管理员窗口(UIPI)**:目标应用以管理员运行时,普通进程 `SetForegroundWindow`/`SendInput` 被 UIPI 拦截,粘贴静默失败。首版接受此限制,不模拟提权。文档中注明。
- **剪贴板被覆盖**:写入内容会覆盖用户当前剪贴板 —— "写剪贴板+Ctrl+V"方案的已知代价,与 PowerToys 行为一致。
- **失焦隐藏**:quick-paste 窗口 `Focused(false)` 事件即隐藏(无延迟,区别于主窗口的 150ms 延迟)。
- **空历史**:面板显示"暂无文本记录"占位。
- **钩子线程**:低级键盘钩子要求在拥有消息循环的线程安装,或运行独立消息循环线程。需确保钩子线程持续 `GetMessage`/`PeekMessage` pump,否则钩子会被系统自动卸载。
- **误触**:200-400ms 时间窗,仅按时间判定,不排除中间夹带其他按键。正常打字不会连续两次 keydown Ctrl。首版接受"做 Ctrl 快捷键操作后紧接另一个 Ctrl 操作"理论上的极小误触概率,不引入中间键排除逻辑。
- **双击判定不消费按键**:钩子回调始终 `CallNextHookEx` 透传,Ctrl 的正常功能(Ctrl+C 等)不受影响。
- **应用退出**:钩子需在进程退出时 `UnhookWindowsHookEx`,Tauri 无明确退出钩子,可用 `Drop` 守卫或 `on_window_event` 的 `CloseRequested`/进程退出处理。

## 6. 测试

### 6.1 Rust 单测
- `AppSettings` 新字段的 default/normalize/roundtrip/legacy 加载测试
- 双击判定状态机:单次 keydown 不触发、间隔过短(<200ms)不触发、间隔过长(>400ms)不触发、合法间隔(300ms)触发
- 面板位置 clamp 纯函数:四角 + 屏幕内 + 超出右/下边界

### 6.2 集成手测(必须,涉及 UI 时序)
- 记事本/Chrome 地址栏/VSCode/Word 里双击 Ctrl → 面板在鼠标处出现 → 选一条 → 内容出现在光标
- Esc 关闭、点外面关闭、↑↓ + Enter 选择
- 鼠标在四角双击 → 面板不超出屏幕
- 设置页关闭双击唤起 → 双击 Ctrl 无反应;再开 → 恢复
- 主窗口 Ctrl+Shift+V 与双击 Ctrl 面板互不干扰
- 多条记录时列表滚动正常
- 空历史时面板显示占位

## 7. 不在范围内(YAGNI)

- 面板内搜索框
- 面板内删除/置顶条目(管理走主窗口)
- 图片粘贴
- 跨多显示器精确定位(首版主屏)
- 管理员窗口提权粘贴
- 双击 Alt/双击 Shift 等其他键(首版仅 Ctrl;设置页暂不提供键选择,仅总开关)
- 面板内编辑内容后再粘贴

## 8. 受影响文件清单(预估)

**新增**:
- `src-tauri/src/quick_paste.rs`(唤起 + 粘贴注入)
- `src-tauri/src/hotkey_hook.rs`(低级键盘钩子 + 双击检测)
- `src/quick_paste.tsx` + `quick_paste.html`(前端面板)

**修改**:
- `src-tauri/src/lib.rs`(setup 安装钩子、注册命令、窗口事件)
- `src-tauri/src/app_settings.rs`(新字段 + normalize + 测试)
- `src-tauri/src/models.rs`(`ClipboardFilter` 若现有实现无法只取 text/link,补充类型过滤能力;实现阶段先验证现有 filter 字段是否满足)
- `src-tauri/tauri.conf.json`(新窗口声明)
- `src-tauri/capabilities/default.json`(新窗口 + 权限)
- `src-tauri/Cargo.toml`(若需新 windows feature flag,如 `Win32_UI_Input_KeyboardAndMouse` 的 `SendInput` 相关、`SetWindowsHookEx`)
- `src/components/SettingsPanel.tsx`(双击唤起开关 UI)
- `vite.config.ts`(多入口 HTML 构建,参考现有 preview.html 配置)
