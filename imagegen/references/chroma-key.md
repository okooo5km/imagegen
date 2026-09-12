# Optional chroma-key removal

Prefer native `--background transparent` with GPT Image 2.5. Use this helper for an existing image on a flat key color, or when the requested workflow needs a chroma key.

Install Pillow in the chosen Python environment, then run:

```text
python scripts/remove_chroma_key.py --input keyed.png --out transparent.png --auto-key border --soft-matte --transparent-threshold 12 --opaque-threshold 220 --despill
```

Inspect alpha coverage and edges; hair, glass, reflections and translucent subjects may need another workflow. Pick a key color absent from the subject. The helper performs local processing and makes no network request. Use `--help` for threshold, despill and edge controls.
