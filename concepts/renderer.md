# Renderer

## Render Context

A Render Context needs to provide all of the necessary functions to build all primitives of graphics renderer, eg. Buffers, Textures, Samplers, Command Buffers (although this should probably be abstracted away), Render Passes (at least at the moment until we abstract that behind a Render Graph) and maybe even Synchronisation primitives.