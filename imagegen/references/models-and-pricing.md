# Models and pricing

Verified against APIMART documentation and public endpoints on 2026-09-12. Query again for current prices.

| Runnable model | Use | Images/request |
| --- | --- | --- |
| `gpt-image-2.5-flare` | Default; routine generation and editing | 1–4 |
| `gpt-image-2.5-sunburst` | Precision editing | 1–4 |
| `gpt-image-2` | Legacy minimal parameter profile | 1 |
| `gpt-image-2-official` | Legacy quality controls and masks | 1–4 |
| `gemini-3.1-flash-image-preview` | Google search/image grounding | 1–4 |

The two 2.5 models accept the same parameter contract. Use low, medium, high, xhigh, max or auto quality; the CLI defaults to medium to avoid auto reserving the maximum quality budget. 2.5 supports all 15 documented aspect ratios at 1K/2K/4K, plus exact dimensions validated locally. The size tier is not a promise that every side is that many pixels: square 4K maps to 2880×2880. Exact dimensions ignore the resolution field. For edits, omitting size lets the service infer dimensions from references.

Local reference files must be PNG/JPEG/WebP/GIF, at most 20 MiB each. They upload using `POST /v1/uploads/images`; generation receives URL strings, never Base64. Upload URLs are public and temporary (72 hours according to the upload documentation).

## Commands

```text
imagegen models
imagegen models --live --json
imagegen models --prices
imagegen prices --model gpt-image-2.5-flare --json
imagegen config set model gpt-image-2.5-sunburst
```

`models` is offline and lists bundled runnable profiles. `--live` calls the authenticated `GET /v1/models?expand=parameters&category=image`; account restrictions affect its results. New or unadapted models are marked `discovery only` and include the server parameter contract in JSON. A catalog entry does not imply the CLI can safely submit to it; adding a runnable profile requires verified parameters and tests. `gpt-image-2.5-ext` is a separate APIMART route, not an alias for either official 2.5 profile.

`prices` calls the public `GET /api/pricing/model?model=...` without Authorization. For token models it returns `data.pricing`, including `effective_rates` and units. Use those effective rates directly; do not apply the discount again. Missing rates mean unknown or inapplicable, never zero. The public default group quote can differ from account billing.

For older per-image contracts, JSON preserves the legacy quote with an explicit **payable rate unverified** status. Do not compare those raw numbers as verified account prices. Check the APIMART pricing page/account before choosing on that basis.

The public 2.5 quote observed on 2026-09-12 was USD per million tokens: text input 4, cached text input 1, image input 6.4, cached image input 1.6, image output 24. This is a dated example, not a baked-in tariff. Total cost also depends on actual input/output tokens, quality, dimensions, reference images and count. The CLI returns task-reported `cost` and `usage` when available; it does not invent a per-image total. Check actual balance and spend with `imagegen balance`, `imagegen usage` and `imagegen logs` ([cli.md](cli.md#account-balance-usage-and-logs)); `auth check` does not verify balance.

## Authoritative references

- [GPT Image 2.5 request contract](https://docs.apimart.ai/en/api-reference/images/gpt-image-2.5/generation)
- [Model metadata API](https://docs.apimart.ai/en/api-reference/texts/models/list)
- [Public pricing contract](https://docs.apimart.ai/en/api-reference/texts/qwen3.8-max/pricing) — generic TokenPricingV2 contract despite the URL name
- [Image uploads](https://docs.apimart.ai/en/api-reference/uploads/images)
- [APIMART pricing](https://apimart.ai/pricing)
