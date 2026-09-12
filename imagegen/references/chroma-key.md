# Native chroma-key removal

Prefer GPT Image 2.5 native transparency for newly generated images. For an existing image on a flat key color:

```text
imagegen remove-chroma-key --input keyed.png --out transparent.png --key 00ff00 --tolerance 30 --softness 60 --despill
```

No Python/Pillow dependency is involved. Output must be PNG. The algorithm computes RGB distance from the key color, creates a soft alpha transition, preserves existing alpha, and optionally suppresses the dominant key channel near transparent edges. `--force` permits replacing an existing output.

Inspect edges and subject coverage. Hair, glass, reflections and translucent material may need another approach. The v1 helper's border-key detection and advanced edge filters are not part of this command; choose an explicit key and thresholds.
