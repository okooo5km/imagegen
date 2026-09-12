# APIMart Imagegen

[![Cross-platform validation](https://github.com/okooo5km/apimart-imagegen/actions/workflows/ci.yml/badge.svg)](https://github.com/okooo5km/apimart-imagegen/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Python 3.10+](https://img.shields.io/badge/Python-3.10%2B-blue.svg)](https://www.python.org/downloads/)

**让 AI Agent 调用 APIMART 生图、编辑图片、查询模型与实时价格。macOS / Linux / Windows，核心功能零第三方依赖。**

中文 · [English](README.en.md)

默认使用 **`gpt-image-2.5-flare`**，同时支持 **`gpt-image-2.5-sunburst`**。它既是符合 [Agent Skills 标准](https://agentskills.io/specification) 的技能，也是可以单独运行的 Python CLI。

作者：**okooo5km(十里)**。更多创作技能、YouMind 上架作品与使用入口：[sink.5km.tech/skills](https://sink.5km.tech/skills)。

## 能做什么

- 文生图、参考图变体、图片编辑、原生透明背景。
- 本地图片上传、异步任务轮询、结果下载与超时恢复。
- 查询账号可见的生图模型、参数元数据与公开价格。
- 保存默认模型；单次请求仍可指定其他模型。
- JSONL 批量生成，先验证全部任务，再限量并发执行。
- 密钥在本机终端隐藏输入，保存在项目之外。

不会自动换模型或重复提交付费任务。遇到不支持的参数会报错，避免参数被丢弃后仍然扣费。

## 安装技能

已有 Node.js/npm 的用户可以使用 [skills CLI](https://github.com/vercel-labs/skills)：

```sh
npx skills add okooo5km/apimart-imagegen --skill imagegen
```

选择你的 Agent 和安装范围。运行技能只需要 **Python 3.10+**，不需要 Node.js。若已安装同名 `imagegen` 技能，先确认要保留哪个版本；本项目不是对所有同名技能的无条件替换。

也可以克隆后，将仓库中的 **整个 `imagegen/` 目录**复制到 Agent 支持的技能目录：

```sh
git clone https://github.com/okooo5km/apimart-imagegen.git
```

不要只复制 `SKILL.md`，它需要相邻的 scripts 和 references。技能入口是 `imagegen/SKILL.md`；仓库根目录负责 README、测试和 CI。

安装后可以这样说：

```text
使用 imagegen，为这篇文章生成一张 16:9 的封面，保存到项目的 output 目录。
使用 imagegen，比较 Flare 和 Sunburst 当前价格，先不要生成图片。
```

支持 `$` 调用的 Agent 也可以使用 `$imagegen`。宿主需要具备读取技能、运行 Python 和联网的能力；仅能聊天的界面不能执行本技能。

## 配置 APIMART 密钥

1. 在 [APIMART 官网](https://apimart.ai) 注册账号。
2. 在 [API Key 页面](https://apimart.ai/keys) 创建密钥，并确保账号有可用额度。
3. 在自己的终端运行配置命令，输入密钥；**不要将密钥发到聊天里**。

下面的示例都从克隆的仓库根目录运行。安装到 Agent 后，应将脚本路径换成安装位置的绝对路径。

**macOS / Linux**

```sh
python3 imagegen/scripts/image_gen.py configure --key
python3 imagegen/scripts/image_gen.py check-key
```

**Windows PowerShell**

```powershell
py -3 imagegen/scripts/image_gen.py configure --key
py -3 imagegen/scripts/image_gen.py check-key
```

如果没有 `py`，使用已加入 PATH 的 `python`。确认 `python --version` 至少为 3.10。隐藏输入时终端不会显示密钥字符，这是正常行为。

密钥优先从环境变量 `APIMART_API_KEY` 读取，其次是本机配置文件：

| 平台 | 配置目录 |
| --- | --- |
| macOS / Linux | `$XDG_CONFIG_HOME/imagegen`；未设置时为 `~/.config/imagegen` |
| Windows | `%APPDATA%\imagegen` |
| 所有平台 | 可用 `IMAGEGEN_CONFIG_DIR` 指定其他目录 |

兼容旧技能的 `~/.config/imagegen/apimart.key`。文件支持 UTF-8 BOM 与 CRLF。Unix 密钥文件权限为 600，Windows 使用用户目录的访问控制。`check-key` 只检查本地配置，不验证余额或远程授权。CI 中通过 secret 环境变量提供密钥；CLI 不会自动读取项目 `.env`。

## 先看模型和价格

以下命令中的 `python` 可替换为 `python3` 或 `py -3`：

```sh
python imagegen/scripts/image_gen.py models
python imagegen/scripts/image_gen.py models --prices
python imagegen/scripts/image_gen.py models --live --prices --json
python imagegen/scripts/image_gen.py prices --model gpt-image-2.5-flare --json
```

- `models`：离线列出已适配、可生成的模型。
- `--live`：读取当前 API Key 可见的生图模型和参数，需要密钥。
- `prices` / `--prices`：实时读取公开价格，不需要密钥。
- `runnable`：CLI 已适配；`discovery only`：平台可发现，CLI 尚无经过验证的生成参数配置。

| 已适配模型 | 用途 | 每次图片数 |
| --- | --- | --- |
| **gpt-image-2.5-flare** | 默认；日常生图与编辑 | 1–4 |
| **gpt-image-2.5-sunburst** | 注重编辑精度 | 1–4 |
| gpt-image-2 | 旧版精简参数 | 1 |
| gpt-image-2-official | 旧版质量参数与蒙版 | 1–4 |
| gemini-3.1-flash-image-preview | Google 搜索 / 图片搜索辅助 | 1–4 |

2.5 的质量档位为 `low / medium / high / xhigh / max / auto`，CLI 默认 **1K、medium、1 张**。支持 15 种宽高比及符合规则的精确像素尺寸，透明背景需 PNG/WebP。`gpt-image-2.5-ext` 是另一条平台路由，不等于 Flare 或 Sunburst 的别名。

**价格不是一张图的固定报价。** 2.5 按实际 Token 用量结算，涉及提示词、参考图、质量、尺寸和数量。CLI 使用 API 返回的 `effective_rates`，显示单位和查询时间；不重复乘折扣，不将缺失值当作免费。公开默认组价格可能与账号结算价格不同。旧版按张计费接口只提供原始报价时，会标为“payable rate unverified”。[计费与模型说明](imagegen/references/models-and-pricing.md)

更改默认模型：

```sh
python imagegen/scripts/image_gen.py configure --default-model gpt-image-2.5-sunburst
```

优先级：`--model` > `APIMART_IMAGE_MODEL` > 保存的默认值 > `gpt-image-2.5-flare`。`--model auto` 使用同一顺序，不代表自动尝试多个模型。

## 生成和编辑

```sh
python imagegen/scripts/image_gen.py generate --prompt "A ceramic teapot in soft studio light" --size 16:9 --resolution 2k --out output/teapot.png
python imagegen/scripts/image_gen.py edit --image "photos/my product.png" --prompt "Change only the background to warm gray; preserve the product and lettering" --out output/product-edited.png
python imagegen/scripts/image_gen.py generate --prompt "A folded paper fox, complete subject" --background transparent --output-format png --out output/fox.png
```

长提示词、引号和多行文本建议存入 UTF-8 文件，用 `--prompt-file prompt.txt`。重复 `--image` 可传多张参考图；2.5 最多 16 张。编辑不传尺寸时，由服务根据输入比例计算；传入 `--size` 会覆盖该行为。

本地输入先上传到 APIMART，得到临时**公开 URL**，再提交生成。支持 PNG/JPEG/WebP/GIF，单文件不超过 20 MiB。API 已停止接受直接传入 Base64 图片；不要上传你无权分享的素材。输出 URL 会过期，CLI 会下载到本机。

预演不花费额度、不上传文件、不联网：

```sh
python imagegen/scripts/image_gen.py generate --prompt-file prompt.txt --size 1600x1200 --dry-run
```

结果默认保存在 `output/imagegen/`。现有文件会被保护；只有明确需要覆盖时才传 `--force`。生成后的图片仍需检查文字、内容和透明通道。

## 批量生成和任务恢复

创建 `jobs.jsonl`：

```jsonl
{"prompt":"A red ceramic teapot", "out":"teapot.png"}
{"prompt":"A paper fox sticker", "background":"transparent", "out":"fox.png"}
```

```sh
python imagegen/scripts/image_gen.py generate-batch --input jobs.jsonl --out-dir output/batch --dry-run
python imagegen/scripts/image_gen.py generate-batch --input jobs.jsonl --out-dir output/batch --concurrency 2
```

任务上限 500，并发范围 1–8、默认 2。输出路径必须位于指定目录内。失败返回非零退出码，成功的文件会保留。

任务提交后会打印 ID，并在本机配置目录 `tasks/` 保存不含提示词或密钥的记录。超时、断网或下载失败时，用原任务恢复：

```sh
python imagegen/scripts/image_gen.py fetch TASK_ID --out output/recovered.png
python imagegen/scripts/image_gen.py fetch TASK_ID --n 4 --out-dir output/recovered
```

`fetch` 不会提交新的生成任务。按原记录填写图片数和输出路径。如果提交失败且没有拿到 ID，先去 APIMART 查看任务记录，再决定是否重试。[完整 CLI 说明](imagegen/references/cli.md)

## 跨平台与验证

实现使用 `pathlib`、标准库 HTTPS、隐藏输入和显式 UTF-8，不依赖 Bash、`chmod` 命令、curl、符号链接或 shell 激活。所有脚本均可由 Python 解释器执行。

[GitHub Actions](https://github.com/okooo5km/apimart-imagegen/actions/workflows/ci.yml) 在 **macOS、Ubuntu、Windows × Python 3.10–3.14** 上执行相同测试，覆盖配置路径、Unicode/空格路径、BOM/CRLF、上传→提交→轮询→下载、费用信息、超时恢复、覆盖保护、批处理和结构检查。CI 使用隔离配置与本地 HTTP 模拟服务，不读取真实密钥、不产生 API 费用。

```sh
python -m pip install -r requirements-dev.txt
python -m ruff check .
python -m unittest discover -s tests -v
```

另按 skill-creator 的 `quick_validate.py` 检查了技能结构，并做独立 Agent 使用测试。网络服务的可用性、账号额度和生成内容质量不由跨平台 CI 保证。

可选的 `remove_chroma_key.py` 用于本地色键抠图；它和蒙版验证需要 Pillow。核心生图不需要安装 Pillow。使用受系统保护的 Python 时，可建立虚拟环境或用 `uv run --with pillow`，不必激活 shell 环境。[跨平台设计依据](docs/portability.md)

## 项目结构

```text
apimart-imagegen/
├── README.md / README.en.md
├── LICENSE / NOTICE
├── imagegen/                  ← 可独立安装的技能
│   ├── SKILL.md
│   ├── LICENSE.txt / NOTICE
│   ├── agents/openai.yaml
│   ├── scripts/
│   └── references/
├── tests/
├── docs/portability.md
└── .github/workflows/ci.yml
```

## 贡献与许可

欢迎提交问题和 PR。新增模型请附 APIMART 官方参数来源、dry-run 测试及三平台 CI 结果；不要把未核实参数直接透传为“已支持”。公开错误日志时移除密钥、私人提示词和临时图片 URL。API 冒烟测试需维护者授权并使用自己的账号。

本项目基于原有 imagegen 技能重构，保留 **Apache-2.0** 许可。改动涵盖网络 CLI、跨平台配置、模型发现、价格查询、文档和测试。它是独立社区项目，与 OpenAI、APIMART、YouMind 无官方隶属关系。

喜欢这个技能的话，可以看看十里的 [更多技能与 YouMind 作品](https://sink.5km.tech/skills)，也欢迎给仓库一个 Star。
