# Example files

Sample inputs for manual smoke-testing (`boinc convert …`, the tray app,
context menus). They are **not** used by automated tests.

| File | Format | Source |
|------|--------|--------|
| `sample.png` | PNG | [W3C image format test suite](https://www.w3.org/People/mimasa/test/imgformat/) (`w3c_home.png`) |
| `sample.jpg` | JPEG | Same suite (`w3c_home.jpg`) |
| `sample.bmp` | BMP | Same suite (`w3c_home.bmp`) |
| `sample.gif` | GIF | Same suite (`w3c_home.gif`) |
| `sample.webp` | WebP | [Google WebP gallery](https://developers.google.com/speed/webp/gallery1) (`1.webp`) |
| `sample.svg` | SVG | Wikimedia Commons — [Ghostscript Tiger](https://commons.wikimedia.org/wiki/File:Ghostscript_Tiger.svg) |
| `sample.pdf` | PDF | [W3C WAI dummy PDF](https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf) |
| `sample.docx` | DOCX | Learning Container sample Word document |
| `sample.md` | Markdown | Written for this repo |
| `flower.jpg` | JPEG | Wikimedia Commons — [JPEG example flower](https://commons.wikimedia.org/wiki/File:JPEG_example_flower.jpg) (photo by David Crawshaw, 2002) |
| `transparency.png` | PNG | Wikimedia Commons — [PNG transparency demonstration](https://commons.wikimedia.org/wiki/File:PNG_transparency_demonstration_1.png) |

## Quick try

```sh
# from the repo root
cargo run -p boinc-cli -- convert examples/sample.png --to svg
cargo run -p boinc-cli -- convert examples/sample.webp --to png
cargo run -p boinc-cli -- convert examples/sample.bmp --to jpg
cargo run -p boinc-cli -- list-conversions examples/sample.gif
```
