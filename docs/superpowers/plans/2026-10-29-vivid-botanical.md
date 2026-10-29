# Vivid Botanical Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans for this bounded inline redesign.

**Goal:** Implement the user-selected botanical concept A throughout the Trellis product page.

**Architecture:** Static HTML/CSS with an optimized original illustration and semantic question buttons; existing trail data, dialog, and screenshot interactions remain native JavaScript.

**Tech Stack:** HTML, CSS, native JS, local fonts, WebP, GitHub Pages.

**Spec:** `docs/superpowers/specs/2026-10-29-vivid-botanical-design.md`

## Global Constraints

- Preserve the 18/16/14px reading scale, cited sample content, native dialogs, focus return, screenshot controls, and self-hosting links.
- Keep actual reading text in HTML and retain 320px/200% text support.
- Do not read or modify the prior README or unrelated dirty files.

## Review Focus

- Long sample questions must fit on the illustrated opening without collisions.
- Enlarged text must flow instead of being clipped by decorative shapes.
- Selected and visited states must follow the current sample trail.
- Foreground/background color combinations must remain readable.
- Missing scripts and reduced motion must preserve the page's content.

## Implementation

- [x] Optimize the original canopy artwork into the workspace.
- [x] Replace the sparse tree markup and page stylesheet with the approved botanical composition and palette.
- [x] Simplify selection handling, remove obsolete SVG drawing code, and retain existing sample/dialog behavior.
- [x] Update the favicon and sharing artwork.
- [x] Verify the rendered site, static references, behavior tests, and accessibility; obtain fresh review.
- [x] Prepare task-scoped publication and verify the live GitHub Pages deployment before handoff.
