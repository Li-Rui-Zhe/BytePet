# README 展示素材

三个 GIF 由项目现有 `public/assets/skins/rick_default/` 中的 calm、happy、sleeping 序列帧编码而来，保留角色外观，不新增外部角色素材，不是桌面功能录屏。

- `bytepet-idle.gif`：平静，4 FPS。
- `bytepet-happy.gif`：开心，4 FPS，使用已经修复天线的原始序列帧。
- `bytepet-sleep.gif`：睡眠，1 FPS。

GIF 采用透明背景、统一画布和同一动画内共享调色板，以降低体积与减少闪色；实际应用继续使用原始 PNG 序列帧，不读取这些文档素材。

SVG 徽章是本仓库内的静态装饰，不依赖外部徽章服务。版本徽章在后续发布时应随项目版本同步更新。
