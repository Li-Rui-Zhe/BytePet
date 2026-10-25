# BytePet

BytePet 是一个基于 Tauri 2、React 和 Rust 构建的桌面 AI 桌宠。它支持可导入 PNG 序列帧角色、独立人物档案、AI 聊天（默认 DeepSeek，支持高级自定义接口）、主题颜色切换、托盘菜单和本地数据持久化。

> 下文功能与教程对应 v0.2.0。源码推送不等于安装包已发布，请以对应标签的 Release 附件为准；main 可能包含尚未发布的后续改动。

## 下载

最新版本可以在 GitHub Releases 下载：

[下载 BytePet v0.2.0](https://github.com/Li-Rui-Zhe/BytePet/releases/tag/v0.2.0)

当前版本提供：

- Windows：`BytePet_0.2.0_x64-setup.exe`
- Windows 备用安装包：`BytePet_0.2.0_x64.msi`
- macOS Apple Silicon 实验包：`BytePet_0.2.0_arm64.dmg`
- macOS Intel 实验包：`BytePet_0.2.0_x86_64.dmg`

macOS 为实验性支持，未完成全面实机验收和 Apple 公证。首次打开可能出现安全提示；应先确认来源，再按 [Apple 官方说明](https://support.apple.com/en-us/102445) 操作，不要全局关闭系统安全保护。

### 当前开发版的 Windows / macOS 构建

当前开发分支按 macOS 13.3+ 适配，Intel 与 Apple Silicon 分别原生构建，包含透明窗口、菜单栏托盘、Retina／Dock 可用区域处理和系统钥匙串存储。macOS 实机验收仍须在首轮 Actions 构建后完成，不能仅凭 Windows 测试宣称全部通过。

GitHub Actions 的 `Build Desktop (Windows & macOS)` 支持推送 main 自动构建、手动运行和版本标签触发，一次并行构建 Windows x64、Mac Intel 和 Mac Apple Silicon。Windows 输出 EXE / MSI，Mac 输出 DMG / 保留权限的 `.app.zip`，各自附带 SHA-256 校验文件。其他开发分支推送不触发打包。只有版本标签构建全部成功后才自动发布同名 Release；main 和手动构建只生成 Artifacts。Windows 包尚未代码签名；Mac 使用 ad-hoc 临时签名，未做 Developer ID 签名与 Apple 公证。不需要将 DeepSeek Key 加入构建配置。

详细步骤见 [跨平台 Actions 打包指南](docs/跨平台Actions打包指南.md)；Mac 适配细节见 [macOS 兼容与 Actions 打包指南](docs/macOS兼容与Actions打包指南.md)。

本次功能收敛、安全修复与跨平台适配内容见 [2026-10-25 更新说明](docs/main更新说明-2026-10-25.md)。

本次版本相对旧版的完整对比见 [v0.2.0 发布说明](docs/releases/v0.2.0.md)。

## 皮肤素材

也可以在 GitHub Releases 下载：

[下载 BytePet v0.1.0](https://github.com/hub12345666/BytePet/releases/tag/v0.1.0)

当前版本提供一些作者本人提供的素材

## 功能特性

- 桌面透明悬浮桌宠，支持拖拽、抛出、落地、睡眠、唤醒和待机动作。
- 人物管理：创建、切换、编辑、删除人物，每个人物拥有独立描述、聊天记录、会话和序列帧资源；「记忆」页用于查看记录与人物存档，并非自动永久记忆。
- PNG 序列帧资源导入：支持系统默认资源和用户自定义角色资源包。
- AI 聊天：支持流式输出、打字机效果、停止生成和动作标签触发桌宠表情动作。
- 默认聊天：内置 DeepSeek 官方连接，支持模型选择、多个具名密钥配置及单个配置启用。
- 高级设置：手动填写 OpenAI 兼容接口的地址、模型名和 API Key；仅维护一套 Chat Completions 协议。
- 轻量陪伴：已移除喂食、库存、能量、好感和每日结算，保留自主动作、拖拽与哄睡。
- 主题系统：内置主题和用户自定义颜色主题，基于 CSS 变量。
- 本地优先：数据库和导入资源保存在用户本机 AppData。AI 聊天会把必要上下文发送给所配置的 API 服务，Key 仅用于对应服务鉴权；应用不另行上传这些数据到项目方服务器。
- 轻量入门：首次启动提供可跳过的三步介绍，不强制配置 AI；完整教程独立放在「指南」页，可随时重新阅读。

## 配置聊天

首次启动可跳过轻量引导，先使用本地陪伴功能。「指南」分为「先开始」「连接 AI」「问题排查」，设置入口可直达 DeepSeek 教程；也可查看仓库内的 [DeepSeek 新手指南](docs/DeepSeek新手指南.md)。应用提供官方密钥管理页面入口；在平台创建并复制 Key 后，回到应用粘贴、测试，再点击“保存并使用”。连接测试不会保存配置，也可能产生少量 API 费用。

默认使用官方地址和 deepseek-flash，并关闭深度思考以便日常陪伴聊天快速响应。「聊天模型」可切换内置 DeepSeek 模型，沿用对应配置已保存的 Key，保存后生效。模型价格与可用性以官方说明为准。

有多个密钥时，点击「新增配置」，分别设置名称、模型与 Key。列表只显示脱敏标识；「仅保存，不切换」可保留备用配置，「启用」仅切换已保存信息，同一时间只启用一个。删除配置需要再次确认，正在使用的配置须先切换后再删除。删除本地配置不会撤销服务商平台上的密钥。

需要使用其他服务或自行填写模型 ID 时，展开“高级设置”，勾选“使用自定义接口”，填写 Base URL 与模型名称。程序自动追加 /chat/completions，也接受完整接口地址。免鉴权的本地接口可不填写 Key，例如 Ollama 的 http://localhost:11434/v1。

不再维护 Gemini、Anthropic、Ollama 等各自的原生协议。旧版已启用的兼容配置会迁入自定义连接；原生配置会保留在本地数据库但停用。旧喂食与状态数据也仅保留，不再参与运行。

DeepSeek 接口与模型参考：[官方文档](https://api-docs.deepseek.com/)。

### 支持范围与问题反馈

项目以 **DeepSeek 为主要维护和测试对象**。高级设置支持手动配置采用 OpenAI 兼容 Chat Completions 协议的服务；这里的“OpenAI 兼容”指请求和响应格式，不代表只能使用 OpenAI 的模型。

自定义接口的地址、模型名称和 API Key 由用户自行填写与验证。项目维护通用的请求发送、流式解析和错误处理逻辑，不逐家适配其他厂商的原生协议，也不承诺所有兼容服务或模型始终可用。服务地址或模型名称变化时，通常可通过修改配置解决；特殊参数、鉴权方式或协议差异则可能无法直接接入。

**虽然不逐家适配，仍然欢迎提交 [Issue](https://github.com/hub12345666/BytePet/issues) 或反馈使用体验和兼容性问题。** 反馈时请提供服务商与模型名称、复现步骤，以及经过脱敏的错误信息。我们会根据问题的通用性、影响范围和维护成本评估改进，但不保证为每个服务提供单独适配。请勿在反馈中公开 API Key、完整聊天记录或其他敏感信息。

## 技术栈

- Frontend: React 19, TypeScript, Vite, Tailwind CSS, Zustand
- Desktop: Tauri 2
- Backend: Rust, SQLite, rusqlite, reqwest
- Package manager: pnpm

## 开发环境

请先安装：

- Node.js 22.12+（22 LTS）或 24 LTS；依赖要求见 package.json 的 engines
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

检查 Windows／macOS 版本、配置和图标：

```bash
pnpm check:desktop
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

Windows 用户数据默认保存在以下目录；其他系统可在「设置 → 本地数据 → 打开数据目录」确认实际位置：

```text
%APPDATA%/com.bytepet.app/bytepet-data/
```

主要内容包括：

- `bytepet.db`：SQLite 数据库
- `skins/`：用户导入的角色序列帧资源
- `logs/`、`backups/`、`exports/`：日志、备份和导出目录

这些数据属于本地运行数据，不应该提交到 Git 仓库。

## API Key 说明

API Key 保存在系统凭据库中（Windows 凭据管理器 / macOS 钥匙串），SQLite 仅保存不透明引用。旧版明文密钥会在启动时迁移并验证；迁移失败时保留原数据库，不静默丢弃密钥。密钥只用于请求用户选择的模型服务，不返回给前端展示。

“测试连接”只测试当前草稿，检查接口是否返回有效的 Chat Completions 文字内容；不会自动保存或启用配置。点击“保存并使用”才会写入配置。

数据库备份不包含系统凭据本体；换电脑或系统用户时需重新填写 API Key。**迁移前生成的旧数据库备份可能仍含明文密钥**，请妥善保存或自行清理，不要公开上传。系统凭据库也不能防护已控制当前系统账号的恶意程序。

请不要提交以下内容：

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
