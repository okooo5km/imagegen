# Prompt development

Preserve the user's goal, requested text and constraints. Add details only when they clarify the intended image. A useful brief can describe subject, medium, composition, lighting, text and exclusions; it does not need every field.

For an edit, name what changes and what remains: identity, pose, camera angle, geometry, lettering and untouched regions. Label the role of each reference image. For multiple assets, keep the intended shared style explicit without making all compositions identical.

## Useful brief

```text
Asset: website hero
Request: a ceramic teapot in a quiet studio
Medium: product photography
Composition: wide image with usable space for page copy
Lighting: soft window light
Constraints: no logos, no text; preserve the supplied teapot shape
```

Use `--prompt-file` for multiline briefs. Fields above are prompt text, not CLI options.

## Text and information graphics

Quote exact visible text. Specify hierarchy, label placement, reading order and font character when relevant. Avoid cramming dense information into a small canvas. Higher quality/resolution may help, but inspect generated spelling, quantities, axes and relationships; generation does not verify factual accuracy.

## Transparency

GPT Image 2.5 supports native `--background transparent` with PNG or WebP. Ask for a complete, uncropped subject and inspect alpha and edges. Read [chroma-key.md](chroma-key.md) only for a keyed-image workflow. Do not promise that a white-looking background is transparent.

## Iteration

Inspect the saved image and make one targeted revision when the user's scope permits. Preserve successful parts. A request for a preview or prompt draft does not authorize a paid generation. Actual retry behavior and model capabilities are described in [cli.md](cli.md) and [models-and-pricing.md](models-and-pricing.md).
