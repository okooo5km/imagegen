# Imagegen

[![Rust build, test and install](https://github.com/okooo5km/imagegen/actions/workflows/ci.yml/badge.svg)](https://github.com/okooo5km/imagegen/actions/workflows/ci.yml)
[![Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024-orange)](Cargo.toml)

**一个原生 Rust 生图 CLI，一份标准 Agent Skill。安装可执行文件后，即可通过 APIMART 生成、编辑图片、管理默认参数、查询模型与价格。**

中文 · [English](README.en.md)

默认 **gpt-image-2.5-flare**，支持 **gpt-image-2.5-sunburst**。覆盖 macOS、Linux、Windows 的 x64 与 ARM64；运行时无需 Python、Node.js、Rust 编译器、Pillow 或 OpenSSL。

作者：**okooo5km(十里)**。更多创作技能与 YouMind 作品：[sink.5km.tech/skills](https://sink.5km.tech/skills)。

## 1. 安装 CLI

### macOS / Linux

```sh
curl --proto '=https' --tlsv1.2 -fLsS https://github.com/okooo5km/imagegen/releases/download/v2.0.1/install.sh -o /tmp/imagegen-install.sh
sh /tmp/imagegen-install.sh
export PATH="$HOME/.local/bin:$PATH"
imagegen --version
```

安装到 `~/.local/bin`，不需要 sudo。将最后的 PATH 配置加入自己的 shell 配置文件后，新终端也能使用；安装器不会修改 shell 配置文件。Agent 也可以立即使用绝对路径 `~/.local/bin/imagegen`。

### Windows PowerShell

```powershell
Invoke-WebRequest https://github.com/okooo5km/imagegen/releases/download/v2.0.1/install.ps1 -OutFile "$env:TEMP\imagegen-install.ps1"
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:TEMP\imagegen-install.ps1"
$env:Path = "$env:LOCALAPPDATA\imagegen\bin;$env:Path"
imagegen --version
```

安装到 `%LOCALAPPDATA%\imagegen\bin`，加入用户 PATH，无需管理员权限。上面的 PATH 赋值同时更新当前终端。

安装器会识别架构、下载对应压缩包、验证 SHA-256，然后替换可执行文件。校验失败不会覆盖已有安装。只用到系统下载、解压和校验工具。

| 平台 | 发布目标 |
| --- | --- |
| macOS Apple Silicon | aarch64-apple-darwin |
| macOS Intel | x86_64-apple-darwin |
| Linux x64 | x86_64-unknown-linux-musl |
| Linux ARM64 | aarch64-unknown-linux-musl |
| Windows x64 | x86_64-pc-windows-msvc |
| Windows ARM64 | aarch64-pc-windows-msvc |

Linux 使用静态 musl；Windows 静态链接 C 运行库。macOS 包带 ad-hoc 签名，未做 Apple Developer ID 公证；Windows 包未做 Authenticode 签名。企业设备的执行策略仍然有效。当前平台覆盖以 [CI 实际结果](https://github.com/okooo5km/imagegen/actions/workflows/ci.yml) 为准，不包含 32 位系统。

也可以从 [Releases](https://github.com/okooo5km/imagegen/releases/latest) 手动下载压缩包和 `.sha256`，校验后解压可执行文件到 PATH。安装器支持 `IMAGEGEN_INSTALL_DIR`、`IMAGEGEN_VERSION`；离线安装可用 `IMAGEGEN_ARCHIVE_DIR` 指定存放压缩包和校验文件的目录。

## 2. 安装技能

CLI 可以独立使用。要让 Agent 使用它，安装标准 `imagegen` Skill：

```sh
npx skills add okooo5km/imagegen --skill imagegen
```

这条可选安装命令需要 Node.js；**CLI 本身不需要**。也可以直接下载 [imagegen.skill](https://github.com/okooo5km/imagegen/releases/latest/download/imagegen.skill)，在支持该格式的应用内导入技能。包内是标准的 `imagegen/SKILL.md`、`references/` 和 `agents/` 结构，本质为 ZIP，附带 SHA-256 校验文件。

若导入窗口只接受 ZIP，使用内容相同的 [ZIP 兼容包](https://github.com/okooo5km/imagegen/releases/latest/download/imagegen-skill.zip)。Claude 官方提供的路径是 **Customize → Skills → + Create skill → Upload a skill**；ChatGPT 的技能能力和导入入口以当前账户及客户端为准。扩展名不保证系统双击安装。参见 [Claude 导入说明](https://support.claude.com/en/articles/12512180-use-skills-in-claude)和 [OpenAI 技能结构说明](https://learn.chatgpt.com/docs/build-skills)。

使用本地技能目录的 Agent，可以将包作为 ZIP 解压，把整个 `imagegen/` 文件夹放进技能目录。同名技能已存在时，先确认是否替换。**导入技能不等于安装 CLI**：实际执行环境仍需支持运行命令、安装对应 CLI、联网和配置 APIMART 密钥；仅能读取附件的聊天环境无法执行生图。

技能会检查 CLI 是否安装、引导密钥设置，然后调用 `imagegen`。Agent 需要能读取技能、运行本地命令、访问网络。

```text
使用 imagegen，把默认比例设为 16:9、分辨率设为 2K，然后为这篇文章生成封面。
使用 imagegen，比较 Flare 和 Sunburst 当前价格，先不要生成图片。
```

## 3. 配置密钥

在 [APIMART 官网](https://apimart.ai) 注册，在 [API Key 页面](https://apimart.ai/keys) 创建密钥，并确保账号有可用额度。作者的邀请链接申请中，当前先使用官网。

在自己的终端运行：

```sh
imagegen auth set
imagegen auth status
imagegen auth check
imagegen doctor
```

密钥输入不可见，保存在项目之外。不要将密钥发到聊天里。`auth status` 只检查本地配置，不检查授权或余额。CI 可通过 secret 环境变量 `APIMART_API_KEY` 提供密钥，CLI 不自动读取项目 `.env`。

| 平台 | 配置目录 |
| --- | --- |
| macOS / Linux | `$XDG_CONFIG_HOME/imagegen`，否则 `~/.config/imagegen` |
| Windows | `%APPDATA%\imagegen` |
| 覆盖 | `IMAGEGEN_CONFIG_DIR` |

密钥保存在 `apimart.key`，默认参数保存在 `config.toml`。配置写入采用原子替换；Unix 文件权限为 600，Windows 使用用户目录的访问控制。

`auth check` 通过只读模型列表请求验证密钥，不产生生图费用；成功不代表余额充足或所有模型均获授权。401 引导更换密钥，403 检查权限，限流或网络故障保留密钥稍后重试。公开价格、本地配置和 dry-run 无需密钥。

## 保存自己的默认参数

```sh
imagegen config set model gpt-image-2.5-flare
imagegen config set aspect_ratio 16:9
imagegen config set resolution 2k
imagegen config set quality high
imagegen config set output_format png
imagegen config set n 1
imagegen config show
```

之后只需要提示词：

```sh
imagegen generate --prompt "A ceramic teapot in soft studio light" --out output/teapot.png
```

临时改成正方形，不影响保存的默认值：

```sh
imagegen generate --prompt "A paper fox" --aspect-ratio 1:1 --out output/fox.png
```

比例、尺寸、分辨率各管一件事：

| 设置 | 示例 | 作用 |
| --- | --- | --- |
| aspect_ratio | 16:9 | 输出画面的比例 |
| size | 1600x1200 | 精确像素尺寸，优先于保存的比例和分辨率 |
| resolution | 1k / 2k / 4k | 配合比例选择像素档位 |

```sh
imagegen config set size 1600x1200
imagegen config unset size
imagegen config path
imagegen config reset
```

`reset` 清空默认选项，但保留密钥。命令行参数覆盖本地默认值；显式 `--aspect-ratio` 或 `--resolution` 会覆盖保存的精确尺寸。显式 `--size` 不与比例/分辨率同时使用。

还支持保存 `background`、`concurrency`、`timeout`、`poll_interval`。内置默认值是 Flare、1:1、1K、medium、1 张、并发 2。模型优先级为 `--model` > `APIMART_IMAGE_MODEL` > 本地配置 > Flare。`auto` 不代表自动换模型。[完整配置规则](imagegen/references/configuration.md)

## 生图、编辑、透明背景

```sh
imagegen generate --prompt-file prompt.txt --aspect-ratio 16:9 --resolution 2k --out output/hero.png
imagegen edit --model gpt-image-2.5-sunburst --image "photos/my product.png" --prompt "Change only the background to warm gray; preserve the product and lettering" --out output/product.png
imagegen generate --prompt "A folded paper fox, complete subject" --background transparent --output-format png --out output/fox.png
imagegen generate --prompt "A ceramic mug" --size 1600x1200 --dry-run
```

`--dry-run` 不需要密钥、不联网、不上传、不创建输出目录。长提示词和复杂引号用 UTF-8 的 `--prompt-file`；BOM、CRLF、中文与空格路径均受测试覆盖。

编辑默认保留输入比例，不套用保存的比例/精确尺寸。显式传入尺寸参数，或加 `--use-default-size`，才会改变这一点。重复 `--image` 可传多张参考图，GPT Image 2.5 最多 16 张。

本地 PNG/JPEG/WebP/GIF 图片先上传到 APIMART，得到临时**公开 URL**再生成；单文件上限 20 MiB。不要上传无权分享的素材。结果会下载到本机，避免依赖即将过期的链接。图片内容、文字和透明通道仍需检查。

已存在的文件不会被覆盖，除非传入 `--force`。不支持的参数会在提交前报错，不会静默丢弃。

已有绿幕图片也能本地处理，不需要 Pillow：

```sh
imagegen remove-chroma-key --input keyed.png --out transparent.png --key 00ff00 --despill
```

## 模型与实时价格

```sh
imagegen models
imagegen models --prices
imagegen models --live --prices --json
imagegen prices --model gpt-image-2.5-flare --json
```

| 已适配模型 | 用途 | 单次图片数 |
| --- | --- | --- |
| **gpt-image-2.5-flare** | 默认；日常生图和编辑 | 1–4 |
| **gpt-image-2.5-sunburst** | 注重编辑精度 | 1–4 |
| gpt-image-2 | 旧版精简参数 | 1 |
| gpt-image-2-official | 旧版质量控制与蒙版 | 1–4 |
| gemini-3.1-flash-image-preview | Google 搜索辅助 | 1–4 |

`models` 离线列出已适配的模型；`--live` 读取当前账号可见的生图模型与参数。新增但未适配的模型会标为 `discovery only`，不能直接生成。

价格查询走公开接口，不发送密钥；按接口返回的 `effective_rates`、单位和时间展示，不重复乘折扣、不将缺失值当作免费。2.5 按实际 Token 用量结算，公开默认组价格可能与账号结算不同。旧版按张计费原始报价会标为“payable rate unverified”。[模型与计费说明](imagegen/references/models-and-pricing.md)

## 批量任务与恢复

`jobs.jsonl` 每行一个请求，未填写的选项使用本地默认值：

```jsonl
{"prompt":"A red ceramic teapot", "out":"teapot.png"}
{"prompt":"A paper fox sticker", "background":"transparent", "out":"fox.png"}
```

```sh
imagegen batch --input jobs.jsonl --out-dir output/batch --dry-run
imagegen batch --input jobs.jsonl --out-dir output/batch --concurrency 2
imagegen fetch TASK_ID
```

最多 500 个任务、1–8 个工作线程。全部任务验证后才开始调用 API。失败不会删除成功文件，部分失败会返回非零退出码。

提交后打印任务 ID，并保存不含提示词/密钥的任务记录。`fetch` 读取记录中的输出路径与数量，继续查询或下载，**不会重新提交生图**。换机器恢复时使用 `--out`/`--out-dir` 和 `--n`。若提交失败且没有 ID，先检查 APIMART 任务历史。[完整 CLI 说明](imagegen/references/cli.md)

## 开发与验证

```sh
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo audit
cargo build --release --locked
```

Rust 2024 edition，最低 Rust 1.88，提交 `Cargo.lock`，禁止本项目的 unsafe 代码。HTTP 使用 reqwest + rustls，参数使用 clap，序列化使用 serde；这些依赖编译进程序，用户无需安装它们。网络生图速度主要取决于服务端，Rust 的收益是部署和运行时统一。

CI 在六个原生系统/架构上运行测试、编译 Release、制作压缩包、调用安装器，再运行已安装的二进制；还测试损坏包拒绝与旧安装保护。另检查格式、Clippy、最低编译器版本、依赖漏洞与标准技能结构。CI 不使用真实密钥、不产生生图费用。[跨平台实现依据](docs/portability.md)

v1 Python 实现保留在历史版本，v2 仓库不再混用两套 CLI。Rust 会读取旧 `config.json` 的默认模型和旧密钥；新修改保存为 TOML。[迁移说明](imagegen/references/configuration.md#migration-from-v1)

## 许可与更多技能

Apache-2.0，见 [LICENSE](LICENSE) 和 [NOTICE](NOTICE)。独立社区项目，与 OpenAI、APIMART、YouMind 无官方隶属关系。新增模型请附官方参数来源、行为测试和跨平台验证；公开日志前移除密钥、私人提示词和临时图片 URL。

欢迎查看十里的 [更多技能与 YouMind 作品](https://sink.5km.tech/skills)，也欢迎 Star 或提交 PR。
