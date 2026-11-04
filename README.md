# Tool4J Paste

常驻 Windows 托盘的剪贴板历史管理器。全局快捷键呼出卡片式面板，历史保存在本地 SQLite，重启不丢。

视觉参考 macOS Paste：圆角面板、材质背景、低对比卡片、克制动效。

[下载安装包](https://tool4j.com/#/detail/1015)
---

## 环境准备

| 依赖 | 说明 |
| --- | --- |
| Node.js ≥ 20 | 前端构建 |
| Rust ≥ 1.77.2 | 后端，通过 [rustup](https://rustup.rs) 安装 |
| MSVC 生成工具 | Rust 在 Windows 上的链接器，需勾选「C++ 生成工具」+ Windows SDK |
| WebView2 Runtime | Windows 11 已内置；Windows 10 需单独安装 Evergreen Runtime |

Rust 未安装时：

```powershell
winget install Rustlang.Rustup
winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

安装后重开终端，`rustc --version` 能输出版本即可。

---

## 运行

```bash
npm install

# 完整应用（Rust + 前端）
npm run app

# 只跑前端界面（浏览器里看样式，使用示例数据）
npm run dev
```

`npm run dev` 后打开 http://localhost:1420 ，不需要 Rust 就能看到完整界面和交互。

打安装包：

```bash
npm run app:build
```

产物在 `src-tauri/target/release/bundle/nsis/`。

---

## 使用

| 操作 | 说明 |
| --- | --- |
| `Ctrl + Alt + V` | 呼出 / 收起面板 |
| 左键托盘图标 | 面板开关 |
| 右键托盘图标 | 显示面板 / 退出 |
| 鼠标滚轮 | 横向浏览卡片（无需按住 Shift） |
| `←` `→` | 切换选中卡片 |
| `Enter` | 粘贴选中项到呼出面板前的窗口 |
| `1` – `9` | 直接粘贴第 N 条（搜索框未聚焦时） |
| `Delete` | 删除选中项 |
| `Esc` | 关闭面板 |

粘贴行为：把历史内容写回剪贴板，收起面板，等待修饰键松开后自动发送 `Ctrl+V`。若目标窗口权限更高或前台切换失败，内容仍在剪贴板里，手动 `Ctrl+V` 即可。

---

## 数据位置

```
%APPDATA%\com.tool4j.paste\
├── tool4j_paste.db     历史与会话设置
└── images\         图片条目（PNG 原图，按内容哈希命名）
```

删除数据库文件即可清空全部历史。

---

## 当前进度

已实现：

- 托盘常驻、单实例、全局快捷键呼出/收起、点击面板外区域不退出
- 剪贴板监听（250ms 轮询序列号），支持纯文本、HTML 富文本、RTF、文件路径、PNG 图片
- 隐私格式过滤：密码管理器标记为「不入历史」的内容直接跳过，且在读取内容之前判断
- 剪贴板被占用时自动重试，避免偶发漏记
- SQLite 持久化，同内容相邻去重，超出上限自动清理最旧的非钉住记录
- 粘贴编排：记录呼出前的前台窗口 → 写回多格式 → 收起面板 → 等修饰键松开 → 注入 `Ctrl+V`
- 自写入抑制：用「内容哈希 + 1.5 秒时效」判断，本人写入不产生重复记录
- 面板贴屏幕底部横条展开，宽度随显示器自适应（120–1400px），高度 380px
- 顶部工具栏：产品标识 + 搜索框 + 类型标签页（带彩色圆点，形式对齐 Paste 的 Pinboard 标签）
- 方形卡片流（250px 宽、随面板高度自适应），卡片刻度与结构参照 Paste：
  顶部彩色色带放「类型名 + 时间 + 来源」，中部是内容，底部两行放标题与来源
- 卡片配色按内容类型：文本蓝、图片橙、链接青、文件灰蓝、富文本紫；色值卡片的色带就是该颜色本身
- 内容识别：链接用链接色、色值直接铺成色块、代码与路径用等宽字体
- 深色 / 浅色跟随系统（调试时可用 `?theme=dark` / `?theme=light` 强制指定）

待完成：

- 图片缩略图（目前直接加载原图，大图会占内存）
- 仅提供 `CF_DIB` 而未提供 `PNG` 格式的图片源（部分老程序）暂不记录
- Pinboard 分组（表结构已就绪，界面未做）
- 设置页、开机自启、隐藏后挂起 WebView 以进一步降内存
- 粘贴失败时的界面提示（事件已发出，前端尚未展示）

---

## 与设计文档的差异

| 项 | 设计文档 | 当前实现 | 原因 |
| --- | --- | --- | --- |
| 中文搜索 | jieba 分词 + FTS5 | `LIKE` 全表匹配 | 万条量级下耗时在几十毫秒内；且两字中文在 FTS5 trigram 下仍需回退，直接 LIKE 更准更简单 |
| 剪贴板监听 | `AddClipboardFormatListener` 消息窗口 | 250ms 轮询序列号 | 少一个窗口过程与消息循环，代码路径短；250ms 延迟无感知 |
| 样式方案 | Tailwind CSS 4 | 手写 CSS + 设计变量 | 组件内有作用域样式，语义类名比长串工具类更好读，也少一层构建依赖 |
| SQLite 并发 | 单写线程 + 只读连接池 | 单个 `Mutex<Connection>` | 写入频率是「用户复制时」，查询是毫秒级，锁竞争可忽略；避免引入连接池复杂度 |
| 全文索引列 | `tokenized_text` | `search_text` | 供 LIKE 使用，存前 4000 字符 |

---

## 目录结构

```
src/                    前端（Svelte 5）
├── App.svelte          面板装配、键盘交互、数据加载
├── app.css             设计变量（颜色、圆角、字体、卡片层次）
└── lib/
    ├── api.ts          唯一的后端调用入口，浏览器下自动走示例数据
    ├── mock.ts         浏览器预览用示例数据
    ├── types.ts        与 Rust 共享的数据结构
    ├── format.ts       相对时间、文本压平
    ├── accent.ts       来源配色与来源名称
    ├── content.ts      内容识别（链接 / 色值 / 代码 / 路径）
    ├── card.ts         卡片视觉定义：色带颜色、类型名、底部标题
    └── components/     TopBar / ItemCard / HorizontalList / Icon

src-tauri/src/          后端（Rust）
├── lib.rs              应用装配、插件注册、命令注册
├── state.rs            全局状态：数据库句柄、自写入标记、上次前台窗口、设置
├── db.rs               表结构、增删查、容量清理
├── clipboard.rs        监听、隐私过滤、多格式读写
├── paste.rs            粘贴编排、修饰键等待、目标窗口恢复
├── window.rs           面板显示隐藏与光标定位
├── util.rs             Win32 封装：光标、前台窗口、进程名、按键注入
├── tray.rs             托盘图标与菜单
├── hotkey.rs           全局快捷键
└── commands.rs         IPC 命令
```