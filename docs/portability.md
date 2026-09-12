# Portability decisions

Research checked 2026-09-12 against primary sources. These decisions are implemented and tested, rather than requiring users to translate shell commands.

| Concern | Implementation | Verification |
| --- | --- | --- |
| Skill discovery | Named folder + concise SKILL.md YAML; optional agents metadata | Structure and local-link tests; skill-creator validator |
| Workspace-relative outputs | Invoke absolute script from the desired working directory | CLI subprocesses run from a separate temporary directory |
| Windows/Unix paths | pathlib, quoted command examples, no string slash concatenation | Native three-OS CI; Unicode and space-containing paths |
| Credential location | APPDATA on Windows, XDG/home on Unix; optional override | Platform location tests and isolated key resolution |
| Terminal differences | getpass; explicit UTF-8 output; BOM/CRLF input support | Noninteractive refusal, Unicode subprocess and BOM tests |
| Dependency setup | Standard library for API operations; Pillow only for local image processing | `python -S` dependency-free smoke in CI |
| HTTP flow | stdlib urllib, bounded calls, no shell curl, no authenticated redirect following | Local HTTP server integration tests |
| Duplicate billing | No retry of generation POST; receipt and fetch recovery | Submission count and resume tests |
| Output portability | Python creates directories; preflight collision checks and exclusive file writes | Batch traversal, case/extension collision and overwrite tests |
| CI evidence | macOS, Ubuntu and Windows × Python 3.10–3.14 | Public workflow run on every push/PR |

A hosted-service failure is distinct from an OS compatibility failure. Unit/integration tests cannot prove upstream availability or image quality; live smoke checks complement them. The bundled model catalog intentionally marks unadapted models as discovery-only.

References:

- [Agent Skills specification](https://agentskills.io/specification)
- [Using scripts in skills](https://agentskills.io/skill-creation/using-scripts)
- [Python pathlib](https://docs.python.org/3/library/pathlib.html)
- [Python getpass](https://docs.python.org/3/library/getpass.html)
- [GitHub Actions matrix strategy](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/run-job-variations)
- [skills CLI](https://github.com/vercel-labs/skills)

The local skill-creator validator used for this release predates the optional top-level `compatibility` field accepted by agentskills.io. Runtime requirements are therefore in the skill body and setup reference so both validators/formats remain compatible.
