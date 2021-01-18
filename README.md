# ttf-utils

Post-processing utilities for [ttf-parser](https://github.com/RazrFalcon/ttf-parser).

## Example

Embolden a glyph outline.

```rust
let face = ttf_parser::Face::from_slice(&font_data, 0).unwrap();
let glyph_id = face.glyph_index('c').unwrap();
let mut outline = ttf_utils::Outline::new(&face, glyph_id).unwrap();
outline.embolden(20.0);
outline.oblique(0.25);
outline.emit(&mut builder);
```

## Credits

The embolden algorithm is derived from the algorithm in the
[FreeType](https://www.freetype.org).

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
