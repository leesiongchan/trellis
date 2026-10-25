# Offline microSD demo

The user approved a clearly labelled offline demo with a preloaded microSD → NAND flash → memory cells tree, expansion, explanations, citations and custom branches. Real topic generation and its verification are excluded. Export/import is a separate future feature.

The existing app already saves complete explorations in IndexedDB and separates selecting from expanding topics. A demo will use that same tree, reader, persistence and phone navigation rather than a second UI.

## Approaches

- Preloaded exploration, chosen by the user's approval: predictable, works without credentials, retains normal saved-tree behavior. Content is bounded and explicitly prewritten.
- Intercept research responses: would retain a misleading research lifecycle and make custom questions look generated. Rejected for this offline demo.
- Separate static mockup: would not exercise the real saved tree or custom branches. Rejected.

## Behavior

Add “Try the microSD demo” on the start screen with “Prewritten content · No API keys needed.” Opening it creates a fresh exploration with a unique ID, ready explanations and only root branches expanded. Preload eight topics: microSD cards, NAND flash, Memory cells, Quantum tunneling, Flash controller, Error correction, Pages and blocks, and Wear leveling. Source links point to primary educational/manufacturer material checked during preparation.

Persist a `mode` field (`research` or `demo`), defaulting legacy saved explorations to `research`. A demo banner and library label stay visible after resuming. Prewritten content must not claim to have been newly researched. Hide refresh actions for demos and guard all research entry points in the session so even direct refresh/clarification calls cannot contact a provider.

Custom questions create ordinary local child nodes and remain accessible through tree navigation. They do not receive invented explanations: the reader says the question is saved and that demo questions are not researched. It still offers adding another local question. Duplicate and ancestor checks remain unchanged. All curated nodes retain normal explanation and citation UI.

Every launch creates a new copy; it never overwrites existing work. Save conflicts, quota errors, deletion and viewport restoration use existing mechanisms. The custom-question notice and demo banner must fit the phone reader/map layout and use normal keyboard-accessible buttons.

## Boundaries

No provider requests, credentials, dependency additions, server endpoints, public hosting, cross-device synchronization or export/import. Existing research behavior remains available outside demo mode. Sources are bundled references; external source links need connectivity, while tree interactions require no provider access. An app reload still requires reaching the app server; this is not a service-worker offline application.

## Acceptance

Launch from home, expand three levels, inspect a source, collapse/reopen, add and revisit a local question, reload and resume with demo labels and questions intact, and navigate on a phone viewport. Assert zero `/api/research` calls for the entire demo. Unit tests cover independent copies, schema compatibility, all demo research entry points, duplicates and real-mode behavior.
