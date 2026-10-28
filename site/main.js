import { createTrailState, getSelection, selectTopic } from './trail.js';

const experience = document.querySelector('[data-trail-experience]');
if (experience) {
  let state = createTrailState();
  const rootButton = experience.querySelector('[data-root]');
  const branchContainer = experience.querySelector('[data-branches]');
  const trailButtons = experience.querySelectorAll('[data-trail]');

  function renderNote() {
    const { trail, topic } = getSelection(state);
    experience.querySelector('[data-root-question]').textContent = trail.question;
    experience.querySelector('[data-note-title]').textContent = topic.title;
    experience.querySelector('[data-note-text]').textContent = topic.text;
    experience.querySelector('[data-note-connection]').textContent = topic.connection;
    const source = experience.querySelector('[data-note-source]');
    source.href = topic.source.url;
    source.replaceChildren(document.createTextNode(topic.source.label + ' '));
    const arrow = document.createElement('span');
    arrow.setAttribute('aria-hidden', 'true');
    arrow.textContent = '↗';
    source.append(arrow);
    rootButton.setAttribute('aria-pressed', String(topic.id === 'root'));
    for (const button of branchContainer.querySelectorAll('[data-topic]')) {
      const active = button.dataset.topic === topic.id;
      button.setAttribute('aria-pressed', String(active));
      button.querySelector('.branch-symbol').textContent = active ? '↗' : '+';
    }
    for (const button of trailButtons) {
      button.setAttribute('aria-pressed', String(button.dataset.trail === state.trailId));
    }
  }

  function renderBranches() {
    const { trail } = getSelection(state);
    const buttons = trail.branches.map((topic) => {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'question-node branch-node';
      button.dataset.topic = topic.id;
      button.setAttribute('aria-pressed', 'false');
      button.setAttribute('aria-controls', 'trail-note');
      const kind = document.createElement('span');
      kind.className = 'node-kind';
      kind.textContent = 'Follow a thread';
      const title = document.createElement('span');
      title.className = 'branch-title';
      title.textContent = topic.title;
      const symbol = document.createElement('span');
      symbol.className = 'branch-symbol';
      symbol.setAttribute('aria-hidden', 'true');
      symbol.textContent = '+';
      button.append(kind, title, symbol);
      return button;
    });
    branchContainer.replaceChildren(...buttons);
  }

  for (const button of trailButtons) {
    button.addEventListener('click', () => {
      state = createTrailState(button.dataset.trail);
      renderBranches();
      renderNote();
    });
  }
  rootButton.addEventListener('click', () => {
    state = selectTopic(state, 'root');
    renderNote();
  });
  branchContainer.addEventListener('click', (event) => {
    const button = event.target.closest('[data-topic]');
    if (!button || !branchContainer.contains(button)) return;
    state = selectTopic(state, button.dataset.topic);
    renderNote();
  });
  renderNote();
  for (const button of experience.querySelectorAll('button')) button.disabled = false;
}
