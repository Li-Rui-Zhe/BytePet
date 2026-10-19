# BytePet

BytePet 是一个基于 Tauri 2、React 和 Rust 构建的桌面 AI 桌宠。它支持可导入 PNG 序列帧角色、独立人物档案、AI 聊天、多模型 Provider 配置、喂食与好感/能量系统、主题颜色切换、托盘菜单和本地数据持久化。

## 下载

最新版本可以在 GitHub Releases 下载：

[下载 BytePet v0.1.0](https://github.com/hub12345666/BytePet/releases/tag/v0.1.0)

当前版本提供：

- Windows：`BytePet_0.1.0_x64-setup.exe`
- Windows 备用安装包：`BytePet_0.1.0_x64_en-US.msi`
- macOS Apple Silicon 测试包：`BytePet_0.1.0_aarch64.dmg`

macOS 版本目前属于早期测试包，暂未进行完整测试、Apple 签名和公证。首次打开时 macOS 可能会显示安全提示，可以右键点击应用并选择“打开”。Mac 版后续会继续优化稳定性和兼容性。

## 皮肤素材

也可以在 GitHub Releases 下载：

[下载 BytePet v0.1.0](https://github.com/hub12345666/BytePet/releases/tag/v0.1.0)

当前版本提供一些作者本人提供的素材

## 功能特性

- 桌面透明悬浮桌宠，支持拖拽、抛出、落地、睡眠、唤醒和待机动作。
- 人物管理：创建、切换、编辑、删除人物，每个人物拥有独立 prompt、记忆、聊天 session 和序列帧资源。
- PNG 序列帧资源导入：支持系统默认资源和用户自定义角色资源包。
- AI 聊天：支持流式输出、打字机效果、停止生成和动作标签触发桌宠表情动作。
- 多 Provider 配置：OpenAI、Gemini、DeepSeek、MiMo、Zhipu GLM、Kimi、Qwen、Ollama、MiniMax、Claude 等。
- 喂食系统：每个人物独立食物类型、库存、能量和好感变化。
- 状态系统：能量、好感、今日聊天轮数、有效互动、每日结算和每周库存清空。
- 主题系统：内置主题和用户自定义颜色主题，基于 CSS 变量。
- 本地优先：数据库和导入资源保存在用户本机 AppData，不上传 API Key 或聊天数据。

## 技术栈

- Frontend: React 19, TypeScript, Vite, Tailwind CSS, Zustand
- Desktop: Tauri 2
- Backend: Rust, SQLite, rusqlite, reqwest
- Package manager: pnpm

## 开发环境

请先安装：

- Node.js
- pnpm
- Rust stable toolchain
- Tauri 2 所需的 Windows 构建依赖

安装依赖：

```bash
pnpm install
```

启动前端开发服务器：

```bash
pnpm run dev
```

启动 Tauri 桌面应用：

```bash
pnpm run tauri dev
```

如果 `1420` 端口被占用，请先关闭已有的 Vite/Tauri 开发进程。

## 常用命令

类型检查：

```bash
pnpm run typecheck
```

代码检查：

```bash
pnpm run lint
```

测试：

```bash
pnpm run test
```

打包：

```bash
pnpm run tauri build
```

打包产物通常位于：

```text
src-tauri/target/release/bundle/
```

## 本地数据

BytePet 的用户数据默认保存在：

```text
%APPDATA%/com.bytepet.app/bytepet-data/
```

主要内容包括：

- `bytepet.db`：SQLite 数据库
- `skins/`：用户导入的角色序列帧资源
- `food_icons/`：用户替换的食物图标
- `logs/`、`backups/`、`exports/`：日志、备份和导出目录

这些数据属于本地运行数据，不应该提交到 Git 仓库。

## API Key 说明

API Key 仅保存在本机 SQLite 数据库中，用于本地桌面应用请求用户选择的模型服务。请不要提交以下内容：

- `.env` 文件
- SQLite 数据库文件
- 截图中可见的 API Key
- 带有真实密钥的配置、日志或导出文件

## 自定义角色资源包

角色序列帧资源包需要包含项目要求的动作目录，例如：

```text
calm/
sleeping/
wake_up/
yawn/
sit/
sit_down/
happy/
cheer_up/
sad/
angry/
comfort/
thinking/
eat_food/
run_left/
run_right/
fly_up/
fall_down/
dizzy/
error/
box/
action1/
```

导入后资源会复制到 AppData 的 `bytepet-data/skins/` 下，安装版也会从该目录读取。

## License

This project is licensed under the MIT License. See [LICENSE](./LICENSE) for details.
