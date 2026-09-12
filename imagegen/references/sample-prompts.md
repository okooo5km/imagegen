# Example prompts

Adapt examples to the request. These are prompt text for the bundled CLI, not a separate image-generation tool.

## Product photograph

```text
A matte cream ceramic teapot on a warm gray studio surface, soft window light from the left, realistic ceramic texture, wide composition, no lettering or logo.
```

## Article illustration

```text
Editorial ink illustration of a person sorting a tangled pile of threads into three labeled trays. Simple readable gestures, limited charcoal and amber palette, white background. Labels exactly: "输入", "判断", "行动".
```

## Transparent object

```text
A small folded paper fox, complete subject with generous padding, crisp folds and soft internal shading, isolated on a transparent background, no floor or cast shadow, no text.
```

Use `--background transparent --output-format png`; inspect the alpha channel after saving.

## Reference-based background edit

```text
Change only the background of the supplied product photo to a pale blue studio backdrop. Preserve the product geometry, position, camera angle, lettering and material texture. Match the existing light direction.
```

## Portrait lighting edit

```text
Adjust only the lighting to a soft studio key light from camera-left. Keep the person's identity, expression, pose, clothes, framing and background unchanged. Preserve natural skin texture.
```

## Text replacement

```text
Replace only the headline with "今天开始". Keep the existing lettering style, alignment and visual weight. Preserve all other text, illustration, colors and layout.
```
