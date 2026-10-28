# Glyphibe

沿着代码之间的关系理解项目。Glyphibe 将文件、函数和更小的源码结构变成图谱。
选择节点即可查看证据、追踪调用、探索潜在修改影响，或通过提问、提示、答案、
回顾循序学习相关编程概念。

源码版本 **1.0.0**，安装器未签名。已发布产物及其审批状态见
[GitHub Releases](https://github.com/styayur/glyphibe/releases)。
[发布门禁](docs/development/release-gates.md)区分已验证、待验项目及性能边界。
[English](README.md)。

![已安装 Windows 应用读取原创公开 Python 夹具](docs/images/native-source.png)

截图来自实际安装运行的 Windows 应用，展示本项目原创公开测试夹具的源码。
各项验证状态见发布门禁记录。

## 已实现的能力

- 使用 Tree-sitter 在本地解析 Python、Rust、JavaScript 和 TypeScript/TSX，
  保守解析跨文件调用、导入、引用与类型关系。
- 桌面、CLI 和标准 stdio MCP 共享带证据的图谱。MCP 允许外部 Agent 在
  所选仓库范围内调用明确的查询工具。
- 架构与解释两个平面共享六种粒度，从领域到表达式，支持多对多源码映射，
  并明确标记陈旧关系。
- 确定性 Louvain 结构社区、独立目录组件、循环检测、调用链和有方向的
  修改影响可达查询。
- 原创双语概念知识、前置概念路线与四档独立教学深度，逐步揭示语法和证据。
- 全窗口画布提供 Frontier、Dense、Cluster 和真正的三维 Galaxy，支持
  玻璃工具岛、键盘操作、专注模式与不透明回退。
- 外置 SQLite 缓存、内容哈希、身份重识别、有上限的扫描、取消和真实文件进度。

核心无须模型、API Key 或云服务，不执行被扫描项目，也不安装其依赖。
用户主动进行的 GitHub 知识导入会联网，结果保持为不可信、仅供本地使用的资料。

## 安装已发布的程序

打开 [Releases](https://github.com/styayur/glyphibe/releases)，先阅读已发布版本的
验证说明。Windows x64 分发文件如下：

| 文件 | 用途 |
|---|---|
| `Glyphibe_1.0.0_x64-setup.exe` | 桌面安装器；安装后启动 Glyphibe |
| `Glyphibe-1.0.0-windows-x64-portable.zip` | 完整解压后运行 `glyphibe-desktop.exe` |
| `Glyphibe-1.0.0-windows-x64-cli.zip` | 完整解压后运行 `glyphibe.exe --help`，无需 GUI |

保留随附的 `licenses` 目录。单独的 CLI 可执行文件应同时取得
`Glyphibe-1.0.0-licenses.zip`。桌面程序需要 Microsoft WebView2。
CLI 需要 [Microsoft Visual C++ v14 x64 运行库](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)。
安装包未签名，Windows 可能显示发布者或 SmartScreen 警告。
使用 `Get-FileHash 文件 -Algorithm SHA256` 与发布的 `SHA256SUMS` 比对。
平台和安装生命周期证据见[安装指南](docs/user-guide/installation.md)与发布门禁。
源码版本号本身不表示下载产物已经获准发布。

## 从源码构建

需要 Rust 1.98.1、Node.js 24 或更新版本，以及对应平台的
[Tauri 环境](https://v2.tauri.app/start/prerequisites/)。Windows 需要 MSVC
Build Tools 和 WebView2。在可信的 Glyphibe 工程目录执行：

```sh
cargo install --locked --path crates/glyphibe-cli
glyphibe analyze tests/fixtures/python
glyphibe --repo tests/fixtures/python search calculate
cd apps/desktop
npm ci
npm run tauri -- dev
```

`npm run tauri -- build --bundles nsis` 构建 Windows 开发安装器。
构建成功不等于安装验证通过。未签名程序可能触发 Windows 发布者或 SmartScreen 提示。

## 第一次使用

1. 在桌面程序选择 **Open folder**；没有 Git 的普通目录也可以分析。缓存位于
   应用的用户缓存目录。
2. 搜索文件或函数，或选择图谱节点。**Evidence** 显示来源和映射，**Source**
   提供经过新鲜度检查的源码片段。
3. 切换 **Frontier** 局部邻域、**Dense** 当前层级网络、**Cluster** 社区与组件、
   **Galaxy** 可旋转三维视图。
4. 使用 **Architecture** 切换六层粒度；沿 `maps_to` 或切换平面进入解释层。
   **Explanation** 独立选择四档教学深度。
5. 在 **Learn** 中先尝试回答，再查看提示和答案。**Knowledge** 提供概念、
   前置概念和归属信息。
6. 在 **Evidence** 调整影响方向和跳数，查看高亮及路径。可达关系不是故障概率。

快捷键：Ctrl+K 搜索、Ctrl+O 打开、F 专注、Esc 恢复。可以固定或折叠工具岛，
选择深浅主题与不透明面板。浏览器导入 `glyphibe export` 的 JSON；完整源码教学
通过桌面 Rust 宿主运行。

参见[桌面指南](docs/user-guide/desktop.md)、
[CLI/MCP 指南](docs/user-guide/cli-and-mcp.md)及
[知识与教学](docs/user-guide/knowledge-and-teaching.md)。

## 限制、证据与隐私

Observed 表示语法事实，Inferred 表示算法推断，Proposed 表示未接受的建议，
Verified 要求独立验证。结构社区不能证明业务领域。动态调用、反射、未经证明的
跨语言桥接、通配导入与再导出、TypeScript 路径别名和自定义 Rust 源码布局仍有
限制，尚未实现 LSP 补充。教学解释已记录的语法与概念，不计算运行时值，也不证明
业务意图。核心分析期间源码保留在本地。

[基准报告](docs/benchmarks/)区分实测和未验证目标。密度只改变投影，不改变事实；
二维上限为 10,000 节点，Galaxy 最大 2,000 节点并提供设备降级。默认扫描预算为
20,000 文件、单文件 2 MiB、总源码 256 MiB、250,000 条提取事实。
跨平台和安装声明以实际验证为准。

原创代码和默认原创知识采用 MPL-2.0，依赖保留各自许可证。参见
[THIRD_PARTY_NOTICES](THIRD_PARTY_NOTICES)、
[清单](docs/development/dependency-inventory.json)及
[研究](docs/research/competitive-analysis.md)。未复制受限制的参考源码。
另请阅读 [SECURITY.md](SECURITY.md)、[CONTRIBUTING.md](CONTRIBUTING.md)
和 [SUPPORT.md](SUPPORT.md)。
