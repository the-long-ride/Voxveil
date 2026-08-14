# Voxveil Design System

## Direction

Voxveil uses **Editorial Monochrome**: x.ai-like restraint with fashion-editorial typography, spacing, and composition. The product identity comes from proportion, hierarchy, motion, and contrast rather than decorative color.

## Principles

- Minimal, calm, precise.
- No gradients, glow, glassmorphism, oversized shadows, or decorative dashboards.
- Use whitespace as the primary separator.
- Avoid card grids unless the content truly needs containment.
- Prefer open lists, rails, panels, and tables.
- Motion is subtle (120-180 ms) and disabled/reduced when the OS requests reduced motion.
- All controls must work with mouse, keyboard, touch, and screen readers.
- Minimum touch target: 44x44 CSS px.
- No remote fonts, icons, images, or runtime UI assets.

## Color Tokens

### Light

- `--bg`: `#F7F7F5`
- `--surface`: `#FFFFFF`
- `--surface-subtle`: `#EFEFEC`
- `--text`: `#111111`
- `--text-muted`: `#6B6B68`
- `--border`: `#DCDCD8`
- `--focus`: `#262626`
- `--success`: `#277A49`
- `--warning`: `#9A6700`
- `--danger`: `#B42318`

### Dark

- `--bg`: `#111111`
- `--surface`: `#171717`
- `--surface-subtle`: `#202020`
- `--text`: `#F3F3F1`
- `--text-muted`: `#A3A39E`
- `--border`: `#30302D`
- `--focus`: `#F3F3F1`
- `--success`: `#58A975`
- `--warning`: `#D6A13B`
- `--danger`: `#E36D64`

Semantic colors appear only when meaning requires them. Active/selected state normally uses foreground/background contrast rather than a brand accent.

## Typography

Use only system font stacks.

- UI: `Inter`-like system sans stack: `ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`
- Technical values: `ui-monospace, "SFMono-Regular", Consolas, monospace`
- Display headings use restrained tracking and stronger scale rather than decorative fonts.

Suggested scale:

- Display: 36/40, 600
- H1: 28/34, 600
- H2: 20/26, 600
- Body: 15/22, 400
- UI: 14/20, 500
- Meta: 12/17, 500

## Spacing

Base unit: 4 px.

Primary scale: 4, 8, 12, 16, 20, 24, 32, 40, 48, 64.

Desktop app chrome should be compact. On onboarding/settings pages, use more negative space and larger vertical rhythm.

## Radius / Elevation

- Small: 6 px
- Standard: 10 px
- Large: 14 px
- No pill shapes unless the control semantics require one.
- Prefer borders to shadows.
- Use one restrained shadow only for transient overlays/popovers.

## Navigation

### Desktop

`Home · Apps · Routing · Engine · Settings`

Persistent left navigation above tablet breakpoint. Collapse to compact rail when width is constrained.

### Mobile

`Home · Apps · Routing · Settings`

Engine controls live in Home on mobile. Use bottom navigation and sheets instead of shrinking desktop panels.

## Responsive Breakpoints

- Compact phone: `< 480px`
- Phone / small tablet: `480-767px`
- Tablet: `768-1023px`
- Desktop: `>= 1024px`

Layouts must tolerate at least 40% text expansion for translations.

## Main Home Surface

The primary screen exposes only high-value controls:

- Master processing toggle
- All Output / Per-App mode
- Vocal level / suppression
- Latency ↔ Quality
- Engine selector
- Current output destination
- Current engine/latency status

Advanced values belong behind detail disclosure, not on the primary surface.

## Component Language

- Buttons: text-first, minimal chrome.
- Toggles: compact, strong binary contrast.
- Sliders: thin track, large accessible thumb, precise keyboard support.
- Rows: use alignment and spacing before boxes.
- Tables/lists: subtle separators, no zebra stripes unless needed for dense technical data.
- Dialogs/sheets: one clear title, concise body, one primary action.
- Icons: local SVG only, small audited set, `currentColor`, consistent stroke.

## Accessibility

- WCAG AA contrast minimum.
- Visible keyboard focus.
- No information encoded by color alone.
- Respect system text scaling where available.
- Respect `prefers-reduced-motion` and `prefers-color-scheme`.
- Every interactive control has an accessible name.

## i18n

Supported languages:

- English (`en`)
- Vietnamese (`vi`)
- Chinese (`zh`)
- Korean (`ko`)
- Japanese (`ja`)
- Spanish (`es`)
- French (`fr`)

Rules:

- No user-visible hardcoded strings in feature components.
- Locale files are bundled locally.
- No cloud translation SDK/backend.
- English is the fallback language.
- Layout must not rely on fixed text widths.
- Avoid concatenating translated fragments.
- Technical units are localized through formatter helpers.
