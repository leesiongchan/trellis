# Trellis Product Page Implementation Plan

> **For agentic workers:** Execute inline; request a fresh final code review.

**Goal:** Create and publish a distinctive, accessible product page on GitHub Pages.

**Architecture:** Buildless static HTML/CSS/JavaScript in `site/`, with a small illustrative trail and local assets. A dedicated GitHub Actions workflow uploads only this directory.

**Tech Stack:** Semantic HTML, CSS, vanilla JavaScript, Node test runner, GitHub Pages.

**Spec:** `docs/superpowers/specs/2026-10-29-product-page-design.md`

## Global constraints

Preserve unrelated work; local assets; relative paths; 320px reflow; keyboard support; reduced motion; no API keys or live provider calls; explicit sample content.

## Review focus

- The site must work from the GitHub project subdirectory.
- Demo choices must keep labels, pressed states, and displayed notes consistent.
- Narrow screens, keyboard focus, and reduced motion must remain usable.
- Setup copy must explain the backend requirement and provider configuration.
- Deployment must publish only the static page and its public assets.

## Tasks

- [x] Build the page and original vector art in `site/index.html`, `site/styles.css`, and `site/assets/`; copy the app screenshot and licensed fonts.
- [x] Verify trail selection and reset with a failing behavior test, then implement the data model in `site/trail.js` and DOM behavior in `site/main.js`.
- [x] Add `.github/workflows/pages.yml` using pinned official actions and a Node test gate.
- [x] Preview locally; exercise desktop, 320px mobile, keyboard, reduced motion, and no-JavaScript content. Check every asset and local link.
- [x] Obtain a fresh code review and address material findings.

Release: commit only this task's files, publish with the dedicated Pages workflow, and verify the live page and assets.


## Verification evidence

Four state-model tests pass. Browser checks covered all nine branches at 320px; 390px, 768px, 1024px, and 1440px layouts; keyboard activation and visible focus; reduced motion; FAQ expansion; and a blocked-script fallback with seven disabled controls and a readable static note. Local anchors and assets resolve, and the public artifact is under 700 KB. A fresh reviewer found no blocking defects; its disabled-fallback observation was addressed.
