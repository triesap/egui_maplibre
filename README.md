# egui_maplibre

MapLibre Native bindings for egui in Rust

## Goals

- Render and control a MapLibre map in egui/eframe apps.
- Provide an ergonomic Rust API for style, camera, interaction, and events.
- Keep core non-opinionated and leave control UI to application code.
- Support an image rendering path by default, with optional glow support.

## Renderer source

This crate uses `maplibre_native` as the rendering backend.

## Contributing

See `CONTRIBUTING.md`.

## License

MIT OR Apache-2.0. See `LICENSE-MIT` and `LICENSE-APACHE`.
