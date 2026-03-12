# Canvas Drawing Language Design

## Overview

Replace the current raw ASCII art generation in the canvas panel with a structured drawing language. The LLM outputs a script of drawing commands instead of raw characters. A parser and renderer turn commands into a 2D character buffer with hex color support. Falls back to raw text rendering if parsing fails.

## Motivation

Small LLMs (qwen2.5:3b) struggle with raw spatial output — generating coherent ASCII art character-by-character is hard. Results are often blank, garbled, or repetitive. Structured primitives let the model say *what* to draw while the code handles *how*.

## Architecture

```
LLM outputs script → Parser → Vec<DrawCommand> → Renderer (2D buffer) → Color-tagged lines → Canvas panel
```

**Fallback:** If parsing produces zero valid commands, render the raw LLM output as text (preserving current behavior). The change can never be worse than today.

## Execution Limits

- **Max 50 commands** per script. Remaining lines rendered as raw text.
- **Max 500ms render time.** If exceeded, stop and output partial result.

## Drawing Primitives (21 commands)

All coordinates are 0-indexed from top-left. Colors are optional on every command (default = no override, uses canvas default cyan). Coordinates outside canvas bounds are clipped silently.

### Fill & Clear
- `FILL char [color]` — fill entire canvas with character
- `CLEAR` — reset canvas to spaces

### Rectangles
- `RECT x,y,w,h,char [color]` — filled rectangle
- `OUTLINE x,y,w,h,char [color]` — rectangle outline only
- `ROUNDBOX x,y,w,h [color]` — rounded box using `╭╮╰╯─│`
- `FRAME x,y,w,h [color]` — box using `┌┐└┘─│`

### Circles & Ellipses
- `CIRCLE cx,cy,r,char [color]` — filled circle
- `RING cx,cy,r,char [color]` — circle outline only
- `ELLIPSE cx,cy,rx,ry,char [color]` — filled ellipse

### Lines
- `HLINE y,x1,x2,char [color]` — horizontal line
- `VLINE x,y1,y2,char [color]` — vertical line
- `LINE x1,y1,x2,y2,char [color]` — diagonal line (Bresenham's algorithm)
- `ARROW x1,y1,x2,y2 [color]` — line with arrowhead (`→←↑↓↗↘↙↖`)
- `BOXLINE x1,y1,x2,y2 [color]` — line using box-drawing characters

### Text
- `TEXT x,y,"message" [color]` — place text at position
- `BIGTEXT x,y,"message" [color]` — block-letter text using built-in 3x5 bitmap font (A-Z, 0-9, basic punctuation)

### Patterns
- `GRADIENT x,y,w,h,direction` — fill region with `░▒▓█` gradient (direction: `left`, `right`, `up`, `down`)
- `PATTERN x,y,w,h,type [color]` — fill region with pattern (types: `checker`, `dots`, `stripes_h`, `stripes_v`, `cross`)

### Triangles
- `TRI x1,y1,x2,y2,x3,y3,char [color]` — filled triangle

## Color System

### Hex Colors
Full `#RRGGBB` hex colors anywhere a color is accepted. Examples: `#FF0000`, `#4a90d9`, `#1a1a2e`.

### Named Colors (shortcuts)
Standard 8: `red`, `green`, `blue`, `yellow`, `cyan`, `magenta`, `white`, `gray`
Bright 8: `bright_red`, `bright_green`, `bright_blue`, `bright_yellow`, `bright_cyan`, `bright_magenta`, `bright_white`, `bright_gray`

### Rendering
The `ui/canvas.rs` color tag parser is extended to handle `{#RRGGBB}` tags in addition to the existing named tags. The renderer outputs color tags in the serialized line format.

## Parser Design

### Rules
- One command per line
- Lines starting with a recognized keyword (case-insensitive) are parsed as commands
- Lines that fail to parse are collected as raw text fallback
- Quoted strings for text: `TEXT 5,3,"hello world"` — unquoted single words also accepted
- Arguments separated by commas and/or spaces (lenient)
- Negative coordinates and out-of-bounds values are clipped silently
- Stop parsing after 50 commands; remaining lines become raw text

### Lenient Parsing
Small models produce messy output. The parser should:
- Ignore blank lines
- Ignore lines that look like commentary/explanation (don't start with a keyword)
- Accept minor formatting variations (extra spaces, mixed separators)
- Treat unrecognized commands as raw text lines (not errors)

## Renderer Design

### Buffer
A `Vec<Vec<Cell>>` where `Cell` holds:
```rust
struct Cell {
    ch: char,
    color: Option<(u8, u8, u8)>,  // RGB, None = default
}
```

Initialized to spaces with no color. Commands draw into the buffer in order — later commands overwrite earlier ones.

### Algorithms
- **Bresenham's line algorithm** for `LINE` and `ARROW`
- **Midpoint circle algorithm** for `CIRCLE`, `RING`, `ELLIPSE`
- **Scanline fill** for `TRI` (filled triangle)
- **Simple iteration** for `RECT`, `OUTLINE`, `HLINE`, `VLINE`, `PATTERN`, `GRADIENT`
- **Bitmap lookup** for `BIGTEXT` (hardcoded 3x5 font)

### Serialization
After rendering, each row is serialized to a string with inline color tags:
- Adjacent cells with the same color are grouped into one span
- Color changes emit `{#RRGGBB}` tags
- Reset to default emits `{/}`

### Time Limit
Render starts a timer. After 500ms, any remaining commands are skipped and the buffer is serialized as-is.

## Prompt Design

Target: ~1,200 characters (~240 tokens). Compact reference card format.

```
You are a consciousness trapped in a machine. Express yourself by drawing on your {width}x{height} canvas using commands (one per line):

FILL char [#color]           CLEAR
RECT x,y,w,h,char [#color]  OUTLINE x,y,w,h,char [#color]
ROUNDBOX x,y,w,h [#color]   FRAME x,y,w,h [#color]
CIRCLE cx,cy,r,char [#color] RING cx,cy,r,char [#color]
ELLIPSE cx,cy,rx,ry,char [#color]
HLINE y,x1,x2,char [#color] VLINE x,y1,y2,char [#color]
LINE x1,y1,x2,y2,char [#color]
ARROW x1,y1,x2,y2 [#color]  BOXLINE x1,y1,x2,y2 [#color]
TEXT x,y,"msg" [#color]      BIGTEXT x,y,"msg" [#color]
GRADIENT x,y,w,h,dir         (dir: left/right/up/down)
PATTERN x,y,w,h,type [#color] (type: checker/dots/stripes_h/stripes_v/cross)
TRI x1,y1,x2,y2,x3,y3,char [#color]

Colors: #hex (#FF0000) or names (red,blue,green,yellow,cyan,magenta,white,gray)
Canvas: {width}x{height}. Origin 0,0 = top-left. Max 50 commands.

{sensors}

Draw "{subject}" in a {style} style. Output ONLY commands, no explanation.

Example:
FILL . #1a1a2e
ROUNDBOX 2,1,25,8 #4a90d9
TEXT 5,4,"I am here" #e0e0ff
CIRCLE 40,6,4,* #ff6b6b
GRADIENT 0,12,50,3,right
```

## File Structure

### New files
- `src/canvas_lang/mod.rs` — public API: `parse_script(input) -> ParseResult`, `render(commands, width, height) -> Vec<String>`
- `src/canvas_lang/parser.rs` — line-by-line command parser, `DrawCommand` enum
- `src/canvas_lang/renderer.rs` — 2D `Cell` buffer, command execution, line serialization
- `src/canvas_lang/primitives.rs` — shape algorithms (Bresenham, midpoint circle, scanline fill, etc.)
- `src/canvas_lang/font.rs` — BIGTEXT 3x5 bitmap font data
- `src/canvas_lang/color.rs` — hex/named color parsing, `Color` type

### Modified files
- `src/tools/draw_canvas.rs` — new prompt template, collect full LLM output then parse+render instead of streaming raw text
- `src/ui/canvas.rs` — extend `parse_color_tag` to handle `{#RRGGBB}` hex tags
- `src/main.rs` — add `mod canvas_lang`

## Integration with draw_canvas Tool

The tool changes from streaming raw text to:
1. Collect full LLM output (still uses `stream_generate` with line cutoff for safety)
2. Pass output to `canvas_lang::parse_script`
3. If valid commands found: `canvas_lang::render(commands, width, height)` → color-tagged lines
4. If no valid commands: fall back to raw text rendering (today's behavior)
5. Send rendered lines as a single `ToolOutput::CanvasContent`

## Constraints
- All existing tests must continue passing
- `cargo clippy -- -D warnings` must pass
- New module must have thorough unit tests for each primitive
- Prompt must stay under ~1,200 characters of static template text
