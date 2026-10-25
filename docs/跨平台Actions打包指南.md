# Windows / macOS 一起打包

更新时间：2026-10-25。main / 手动构建生成 Artifacts；版本标签构建全部成功后发布同名 Release，不自动创建或移动 Git 标签。

## 一次运行产出什么

下表为 Actions 保留的完整构建产物。面向普通用户的 Release 只发布 Windows EXE、Mac Intel DMG、Mac Apple Silicon DMG 及这三个文件的校验值，不发布 MSI 和 app.zip。GitHub 自动生成的 Source code 源码项不属于安装包。

| 任务 | 运行环境 | 安装包 |
| --- | --- | --- |
| Windows x64 | windows-2022，原生 MSVC | EXE（推荐）和 MSI（备用，二选一安装） |
| Mac Intel | macos-15-intel | x86_64 DMG 和 app.zip |
| Mac Apple Silicon | macos-15 | arm64 DMG 和 app.zip |

每份 Artifact 都包含 `SHA256SUMS.txt`，保留 14 天。Mac 最低系统为 13.3。Windows 当前验证目标是 Windows 10 / 11 x64，未提供 Windows ARM64 原生包，也不承诺旧版 Windows 兼容。

## 怎么触发

1. 提交并推送完整源码（包括新增的 scripts、平台配置、图标和文档）到 main，自动触发三个平台并行构建。
2. 在 GitHub 仓库打开 **Actions → Build Desktop (Windows & macOS)**，找到此次提交对应的运行记录。
3. 如需重试或构建其他分支，使用 **Run workflow** 选择目标分支运行；main 已自动启动时，不必再手动重复触发。
4. 等三个任务全部成功，在该次运行页面底部下载对应的 Artifacts。

main 的 push 自动触发打包，其他开发分支 push / pull request 不触发。推送 `v*` 标签也会触发，标签必须精确等于 `v` 加项目版本号。目前 package.json、Cargo.toml 和 tauri.conf.json 都是 `0.2.0`；Cargo.lock 的项目条目也必须一致。构建不会自动递增版本。以后发新版先同步版本和锁文件、补充 `docs/releases/v版本号.md`，再创建新的、不重复的版本标签。同一分支重复推送会取消此前尚未完成的同分支构建。

手动入口受 GitHub 默认分支的 workflow 存在性限制，详见 [GitHub 手动运行说明](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow)。为保留已有 workflow 的文件身份，本流程文件仍叫 `.github/workflows/build-macos.yml`，网页显示名已经改为跨平台名称；不要再新建同内容 workflow，避免重复打包。

## Windows 做了哪些检查

- 冻结 pnpm 及 Cargo 锁文件，检查三个版本号、平台配置、图标、类型、lint、前端与 Rust 测试。
- 在临时 CI runner 测试 Windows 凭据库，只写入和删除随机测试凭据，不使用真实 API Key。
- 原生构建 NSIS EXE 和 WiX MSI；NSIS 默认为当前用户安装，提供中文 / 英文界面。
- 校验 release 应用的 PE x64 架构、版本，检查恰好生成一个 EXE 和一个 MSI；启动 release 应用观察进程是否提前退出。
- 启动检查只在 CI 执行，不会在开发者本机自动运行安装器，不等于完成安装 / 卸载 / 升级验收。
- 只上传安装包、校验文件；失败时仅上传本次启动日志，不上传数据库、凭据或整个构建目录。
- WiX / NSIS 构建工具缓存到项目 `src-tauri/target/.tauri/`，本地构建时随项目留在 D 盘，不进入 Git。

Windows 依赖系统 WebView2；缺失时安装器会联网下载，当前不是完整离线安装包。构建 MSI 需要 Windows 的 VBScript 可选功能，固定 runner 以降低环境漂移影响；若云端报告 light.exe 相关错误，先检查 runner 镜像变化。[Tauri Windows 安装器文档](https://v2.tauri.app/distribute/windows-installer/)

## 发布与安全边界

构建任务只有仓库读取权限。仅版本标签事件启用单独的发布任务：三个平台全部成功后，下载本次运行对应的三个 Artifact，重新校验全部六个包的 SHA-256，但只选取三个推荐安装包并生成对应的三条校验值，然后先上传到草稿 Release，完整成功后发布为 `BytePet v版本号`。失败不会公开发布半套安装包；已经发布的 Release 不会被自动覆盖。草稿失败时可重跑发布任务继续上传。MSI 和 app.zip 仅留在对应 Actions 的 Artifacts 中，保留期为 14 天。

只有发布任务获得 `contents: write`，使用 GitHub 自动提供的临时 GITHUB_TOKEN，不需要自己创建 PAT 或把聊天 API Key 放进仓库。main / 手动运行不会创建 Release，标签也不会由 workflow 自动新建。未来每个正式标签都需要对应的 Markdown 发布说明。

Windows 当前未配置代码签名，可能出现 SmartScreen 提示；Mac 只做 ad-hoc 签名，未公证。不要关闭系统全局安全保护。公开发行及减少警告需要后续单独接入代码签名；证书和密码只能放受保护的 Secrets，不能提交。

不需要添加 DeepSeek 或其他聊天服务的 API Key 到 GitHub，构建和启动检查不发起聊天请求。Mac 的钥匙串隔离、签名校验、DMG 检查与实机验收要求见 [Mac 指南](macOS兼容与Actions打包指南.md)。

云端首次成功后仍需实际验证：Windows 安装 / 卸载 / 升级与数据保留、透明窗口与托盘恢复；两种 Mac 架构的启动、菜单栏、Retina / Dock、钥匙串与素材导入。不能用构建成功替代这些交互验收。
