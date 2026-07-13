# boinc-core

The conversion engine. UI- and OS-agnostic: no Floem, no platform integration
code. Everything downstream — context-menu entries, CLI listings, the app's
format picker — is generated from the `ConverterRegistry`, so a conversion
added here shows up everywhere automatically.

## How a conversion runs

`pipeline::convert` detects the input format from file content (magic bytes;
the extension is only a tie-breaker for ZIP containers), looks up an available
`Converter` for the (from, to) pair, resolves the output path via
`OutputPolicy` (same directory, new extension, ` (1)` suffix instead of
overwriting), and runs it with a progress callback.

## Adding a new conversion

1. **New module** under `src/converters/` implementing the `Converter` trait:
   - `supports()` — the (from, to) pair.
   - `convert()` — do the work; call `progress` with fractions in `0.0..=1.0`.
   - `is_available()` — override only if the converter delegates to an
     external tool that may be missing (see `libreoffice.rs`); unavailable
     converters are hidden from menus but stay registered.
2. **New `Format` variant?** Add it to the enum in `src/format.rs` plus its
   `extension()`, `display_name()`, `from_extension()` arms, and its magic
   bytes in `sniff()`.
3. **Register it** in `ConverterRegistry::with_defaults` (`src/registry.rs`) —
   the single registration point.
4. **Test it** in `tests/`: a round-trip through the public `convert` API if
   the pair is invertible, plus a detection test for any new format. Tests
   that need an external tool must self-skip when it is absent (see
   `tests/libreoffice.rs`).

## Conventions

- Converters never overwrite: the pipeline hands them a resolved,
  non-existing output path.
- Options shared across converters live in `ConversionOptions`; converters
  read what applies to them and ignore the rest.
- External-tool converters must work headlessly and must not depend on user
  configuration (see `find_soffice` for the lookup pattern: env override →
  `PATH` → known install locations).
