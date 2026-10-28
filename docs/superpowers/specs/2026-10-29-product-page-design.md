# Trellis product page

Build an inviting product page for GitHub Pages that extends Trellis's warm paper, forest-green palette, Instrument Serif headings, and DM Sans body text. The page should make curiosity tangible and lead visitors to self-host the app.

## Direction

Use an interactive field guide: generous editorial typography, original botanical line drawings, specimen annotations, and a small branching demonstration. A conventional screenshot-led showcase would be clear but less distinctive; a dense editorial journal would bury the product. The field guide combines the app's visual language with a concrete invitation to explore. The user selected the interactive field-guide direction.

## Experience

1. A strong editorial opening: a small question can lead somewhere wonderful.
2. An interactive, clearly labeled sample trail, with selectable questions and an accompanying field note. Every selection works with a keyboard and on a narrow screen; no API calls.
3. Real screenshots explain the map, cited reader, and saved knowledge.
4. A short account of the research workflow, then practical setup information and a GitHub call to action.

## Constraints

- Buildless HTML, CSS, and JavaScript under `site/`; relative assets work at `/trellis/`.
- Use local fonts and local images; no analytics, external scripts, provider credentials, or backend dependency.
- Product claims follow the app's implementation. Distinguish the sample trail from live research.
- Respect reduced motion, use semantic controls and visible focus, and avoid horizontal overflow at 320px.
- Preserve the existing dirty README, Docker files, and screenshot originals. Publish only the dedicated static directory.
- Configure the official GitHub Pages Actions deployment with minimal permissions and pinned actions.

## Research

- Maggie Appleton, [A Brief History & Ethos of the Digital Garden](https://maggieappleton.com/garden-history): connected ideas, exploration, ownership.
- [BHL Field Notes Project](https://field.biodiversitylibrary.org/collection/FieldNotesProject): specimen annotations and observation-led compositions.
- [Calm Tech principles](https://www.calmtech.institute/calm-tech-principles): quiet interaction and minimal demands on attention.
- NN/g, [Progressive Disclosure](https://www.nngroup.com/articles/progressive-disclosure/): let visitors choose a branch and reveal its explanation.
- [GitHub Pages custom workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages): configure, upload, deploy.
