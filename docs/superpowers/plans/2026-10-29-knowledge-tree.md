# Knowledge Tree Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans for inline execution of this bounded site change.

**Goal:** Replace the product page's abstract sculpture with an interactive branching knowledge tree.

**Architecture:** Semantic HTML question cards over a responsive SVG tree. A small local module measures card anchors and draws their connecting paths; existing trail data and dialogs retain ownership of sample content.

**Tech Stack:** Static HTML, CSS, native JavaScript modules and SVG, GitHub Pages.

**Spec:** `docs/superpowers/specs/2026-10-29-knowledge-tree-design.md`

## Global Constraints

- Preserve the readable 18/16/14px text scale and static GitHub Pages architecture.
- Keep existing sample notes, sources, dialog semantics, and screenshot modes.
- Do not read or edit the prior README or unrelated dirty files.

## Review Focus

- Narrow and enlarged text: cards remain readable and connected without page overflow.
- Trail changes: branch endpoints follow the replacement cards.
- Pointer and keyboard focus: preview and selected paths agree with the current card.
- Dialog close: focus returns to its triggering question.
- Script failure and reduced motion: content stays usable without animated drawing.

## Task: Replace the sample composition

- [x] Replace the sculpture, constellation labels, and old scene CSS in `site/index.html` and `site/styles.css` with the knowledge tree and responsive card arrangement.
- [x] Implement measured branches in `site/tree.js`; integrate drawing and path selection in `site/main.js`.
- [x] Retain existing behavior tests; use browser checks to verify real SVG endpoints, trail replacement, text enlargement, and keyboard interactions.
- [x] Update sharing artwork and metadata to reflect the tree.
- [x] Run formatter, static checks, browser accessibility checks, and a fresh review; fix material issues.
- [x] Prepare reviewed, task-scoped files for GitHub Pages publication; verify the live deployment before handoff.
