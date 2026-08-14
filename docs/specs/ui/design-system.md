# Voxveil UI Design System

## Direction

Editorial Monochrome: x.ai-like restraint combined with fashion-editorial typography and spacing. Identity comes from proportion, contrast, hierarchy, and motion rather than decorative color.

## Palette

Light uses warm near-white, white surface, near-black text, muted graphite, and hairline gray borders. Dark uses near-black, charcoal surfaces, soft-white text, and restrained gray borders. Color appears primarily for focus and semantic warning/error/success states.

No gradients, glow, glassmorphism, oversized shadows, colorful dashboards, or remote fonts.

## Layout

Desktop: fixed left navigation plus open main workspace. Mobile <= 760px: top master control plus four-item bottom navigation. Main content supports 320px width without horizontal page scrolling.

## Controls

Touch targets are at least 44px where practical. Sliders expose text labels and numeric state. Switches use native button semantics plus `role=switch`. Focus is always visible. Reduced-motion preference collapses transitions.

## Typography

Use local system UI fonts only. Large page titles use tight editorial tracking; technical latency values use a system monospace stack. Do not load web fonts.

## Themes

System, Light, and Dark are user-selectable and persisted locally. Both themes share the same geometry and information hierarchy.
