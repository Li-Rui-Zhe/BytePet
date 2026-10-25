# macOS 兼容与 GitHub Actions 打包指南

适用日期：2026-10-25。当前准备的是 macOS 测试包流程，不是已经过真机验收、Developer ID 签名和 Apple 公证的正式版本。

## 支持基线

- macOS 13.3 或更新版本；建议及时安装系统安全更新。这个最低基线用于保证当前 WKWebView 界面所需的 CSS 和浏览器能力，不代表所有更早系统绝对无法运行。
- Apple Silicon（M 系列）：`aarch64-apple-darwin`，下载名称中 `arm64` 的包。
- Intel：`x86_64-apple-darwin`，下载名称中 `x86_64` 的包。
- 原生双架构分别构建，不要求 Apple Silicon 用户通过 Rosetta 运行 Intel 包。目前不是 Universal 单文件包。
- 桌宠活动范围为主显示器可用区域；窗口避开菜单栏和 Dock，不承诺跨屏行走。Dock 位置、缩放和可用区域改变后会重新对齐。
- 使用菜单栏图标管理：显示工具箱、哄睡、隐藏桌宠、退出。常驻时隐藏 Dock 应用图标，保留顶部菜单栏入口。

## 本轮已经做的适配

1. 独立 `src-tauri/tauri.macos.conf.json`：透明无边框、置顶、跨工作区显示、系统版本与临时签名配置，不改变 Windows 安装配置。
2. 补充 `.icns` 图标，直接从现有应用图标编码生成，不重新设计角色。
3. 屏幕计算区分物理像素与逻辑坐标；Mac 窗口覆盖主屏可用区域，避免把顶部菜单栏误算为底部 Dock 高度。支持 Retina、侧边 Dock、非零／负显示器原点的计算。
4. 菜单栏恢复／素材选择返回时重置鼠标穿透缓存；隐藏使用 Mac 的窗口隐藏，不调用 Windows 任务栏显示逻辑。
5. GUI 启动时不依赖 shell PATH；打开目录使用 `/usr/bin/open`。
6. 数据使用 Tauri 系统应用数据目录，通常为 `~/Library/Application Support/com.bytepet.app/bytepet-data/`，不写入 DMG、`.app` 或源码目录。
7. API Key 继续存于系统钥匙串，数据库只保存引用。不把本机 API Key 或数据库打进安装包，也不需要将 DeepSeek Key 添加到 GitHub Secrets。

## 后续在 GitHub 上操作

推送 main 后将自动启动跨平台构建；需要构建其他分支时按以下步骤手动运行。不要重复触发已经开始的 main 构建。

1. 推送代码后进入仓库 **Actions → Build Desktop (Windows & macOS) → Run workflow**。该流程现在同时构建 Windows，完整入口见 [跨平台 Actions 打包指南](跨平台Actions打包指南.md)。
2. 在分支选择中选要构建的开发分支，例如 `codex/simplify-companion`，不要误选尚未包含改动的 main。
3. 手动运行后会并行出现 Windows x64、Mac Intel、Mac Apple Silicon 三个任务。两个 Mac 任务都成功后再下载对应的 Mac 包；公开发布前须确认三个任务全部成功。
4. 进入该次运行底部 **Artifacts**，下载对应 CPU 的 `BytePet-macos-…`。下载 Artifact 通常需要登录 GitHub。
5. 外层 Artifact 解压后会得到 `.dmg`、`.app.zip` 和 `SHA256SUMS.txt`。优先使用 DMG；打开后把应用拖入「应用程序」，退出 DMG 后再启动。
6. `.app.zip` 是使用 `ditto` 制作的权限保留备份，避免直接上传松散 `.app` 后丢失可执行权限。不要把未压缩 `.app` 文件夹直接用 Artifact 上传。

GitHub 要求手动触发的 workflow 已存在于默认分支，才能显示 Run workflow 入口。仓库已有同名 workflow 时可选择其他分支运行；若入口尚未出现，需要后续把 workflow 配置纳入默认分支，不能假装仅在本地修改就能立即触发。[官方手动运行说明](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow)

当前 workflow 也保留 `v*` 标签触发，并校验标签必须为 `v` 加当前版本号。现有版本配置为 `0.2.0`，Cargo.lock 中项目版本也需同步；以后发布前先同步更新，不要重复覆盖已有标签。手动构建不要求创建标签。版本标签运行全部成功后会自动发布同名 Release，main / 手动运行不会发布。

## Actions 自动检查的内容

- Node 24、pnpm 10.33.2、Rust stable；按 CPU 分开的依赖缓存。
- 冻结 pnpm 锁文件、锁定 Cargo 依赖；类型检查、lint、前端及原生架构 Rust 测试。
- 独立临时钥匙串的真实测试：只创建／读取／删除随机测试凭据，退出时恢复 runner 原默认钥匙串并删除临时钥匙串。
- 原生构建 `app,dmg`；校验 plist、版本、最低系统、Mach-O 架构、可执行权限与临时签名。
- 启动打包后的应用并观察进程存活，不使用 `tauri dev` 冒充安装包测试；无 API Key，不发起聊天请求。
- 校验 DMG、生成权限保留 ZIP、SHA-256 校验文件。失败时仅上传此次启动日志，不上传用户数据目录或钥匙串。
- Artifacts 产物保留 14 天，失败启动日志保留 7 天。构建任务只有读取仓库权限；仅版本标签的发布任务拥有写权限，全部平台通过后发布 Release 附件。

## 签名与首次打开

默认 `signingIdentity: "-"` 是 ad-hoc 临时签名，用于测试与本地完整性验证；不代表可验证开发者身份，也没有 Apple 公证。无需 Apple Developer 证书即可尝试 Actions 测试构建。

Mac 从网络下载后可能触发 Gatekeeper。先确认来源、核对校验值；确实信任来源时，按 Apple 官方的「系统设置 → 隐私与安全性 → 仍要打开」流程处理。不要全局关闭 Gatekeeper；若提示已知恶意软件或文件损坏，先停止并排查，不应一律强行放行。[Apple 安全打开应用说明](https://support.apple.com/en-us/102445)

以后若要公开正式分发、减少首次打开阻力，需要另行配置 Developer ID 签名和公证。证书、密码、Apple 账号凭据只能放受保护的 GitHub Secrets，不能提交到仓库；是否接入由用户后续决定。

## 仍需完成的真实验收

本地是 Windows，不能在这里声称已经运行过 macOS 安装包。首次云端构建后仍要用实际 Mac 检查：

- 透明背景、菜单栏图标在明暗模式下可见；隐藏／恢复／退出可用。
- Retina 缩放、底部／左侧／右侧 Dock、自动隐藏 Dock；切换工作区和全屏应用的覆盖行为。
- 拖动、抛出、落地、哄睡、人物尺寸和面板定位；不挡住底层应用操作。
- 中文路径素材导入与重启后显示；钥匙串保存、更换 Key 与升级后的权限提示。
- 默认 DeepSeek 与高级兼容接口的实际聊天（由用户自主配置，测试可能计费）。

参考：[Tauri macOS 应用包](https://v2.tauri.app/distribute/macos-application-bundle/)、[macOS 签名](https://v2.tauri.app/distribute/sign/macos/)、[配置合并](https://v2.tauri.app/reference/config/)、[WebKit 16.4](https://webkit.org/blog/13966/webkit-features-in-safari-16-4/)。
