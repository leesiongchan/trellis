import { createTrailState, getSelection, selectTopic } from './trail.js';

const constellation = document.querySelector('.constellation');
const dialog = document.querySelector('#thought-dialog');
if (constellation && dialog) {
  let state = createTrailState('memory');
  const visited = new Set();
  const branches = document.querySelector('[data-branches]');
  const root = document.querySelector('[data-root]');
  const choices = document.querySelectorAll('[data-trail]');
  const branchLabels = ['THE THREAD', 'THE PERSPECTIVE', 'THE POSSIBILITY'];
  let returnFocus = null;

  function updateNote() {
    const { trail, topic } = getSelection(state);
    document.querySelector('[data-note-title]').textContent = topic.title;
    document.querySelector('[data-note-text]').textContent = topic.text;
    document.querySelector('[data-note-connection]').textContent = topic.connection;
    document.querySelector('[data-dialog-path]').textContent =
      trail.label + (topic.id === 'root' ? '' : ' → ' + topic.title);
    const source = document.querySelector('[data-note-source]');
    source.href = topic.source.url;
    source.replaceChildren(document.createTextNode(topic.source.label + ' '));
    const arrow = document.createElement('span');
    arrow.setAttribute('aria-hidden', 'true');
    arrow.textContent = '↗';
    source.append(arrow);
  }

  function openNote(topicId, trigger) {
    state = selectTopic(state, topicId);
    visited.add(state.trailId + ':' + topicId);
    returnFocus = trigger;
    trigger.classList.add('is-visited');
    updateNote();
    for (const path of constellation.querySelectorAll('[data-connection]')) {
      path.classList.toggle('is-active', path.dataset.connection === trigger.dataset.branchIndex);
    }
    dialog.showModal();
  }

  function renderTrail() {
    const { trail } = getSelection(state);
    document.querySelector('[data-root-question]').textContent = trail.question;
    root.classList.toggle('is-visited', visited.has(trail.id + ':root'));
    const cards = trail.branches.map((topic, index) => {
      const card = document.createElement('button');
      card.type = 'button';
      card.className =
        'thought-card branch-card ' + ['branch-one', 'branch-two', 'branch-three'][index];
      card.dataset.topic = topic.id;
      card.dataset.branchIndex = String(index);
      card.setAttribute('aria-haspopup', 'dialog');
      card.setAttribute('aria-controls', 'thought-dialog');
      card.classList.toggle('is-visited', visited.has(trail.id + ':' + topic.id));
      const meta = document.createElement('span');
      meta.className = 'card-meta';
      meta.textContent = '0' + (index + 1) + ' / ' + branchLabels[index];
      const title = document.createElement('span');
      title.className = 'branch-title';
      title.textContent = topic.title;
      card.append(meta, title);
      if (index === 1) {
        const illustration = document.createElement('span');
        illustration.className = 'mini-layers';
        illustration.setAttribute('aria-hidden', 'true');
        illustration.append(
          document.createElement('i'),
          document.createElement('i'),
          document.createElement('i')
        );
        card.append(illustration);
      }
      const bottom = document.createElement('span');
      bottom.className = 'card-bottom';
      bottom.append(
        document.createTextNode(
          ['Follow this thought', 'A different perspective', 'And another question'][index]
        )
      );
      const arrow = document.createElement('span');
      arrow.textContent = '↗';
      arrow.setAttribute('aria-hidden', 'true');
      bottom.append(arrow);
      card.append(bottom);
      return card;
    });
    branches.replaceChildren(...cards);
    for (const choice of choices)
      choice.setAttribute('aria-pressed', String(choice.dataset.trail === state.trailId));
    for (const path of constellation.querySelectorAll('[data-connection]'))
      path.classList.remove('is-active');
    updateNote();
  }

  root.addEventListener('click', () => openNote('root', root));
  branches.addEventListener('click', (event) => {
    const trigger = event.target.closest('[data-topic]');
    if (trigger && branches.contains(trigger)) openNote(trigger.dataset.topic, trigger);
  });
  for (const choice of choices)
    choice.addEventListener('click', () => {
      state = createTrailState(choice.dataset.trail);
      renderTrail();
    });
  for (const button of dialog.querySelectorAll('[data-close-dialog]'))
    button.addEventListener('click', () => dialog.close());
  dialog.addEventListener('click', (event) => {
    if (event.target !== dialog) return;
    const rect = dialog.getBoundingClientRect();
    if (
      event.clientX < rect.left ||
      event.clientX > rect.right ||
      event.clientY < rect.top ||
      event.clientY > rect.bottom
    )
      dialog.close();
  });
  dialog.addEventListener('close', () => {
    if (returnFocus?.isConnected) returnFocus.focus({ preventScroll: true });
  });
  renderTrail();
  root.disabled = false;
  for (const choice of choices) choice.disabled = false;
}

const screen = document.querySelector('[data-screen]');
const descriptions = {
  whole: 'A map for the big picture. A reader for the finer details.',
  map: 'Follow a branch, retrace your path, and see how the ideas connect.',
  reader: 'An explanation, its sources, and the next question—all within reach.'
};
if (screen) {
  for (const button of document.querySelectorAll('[data-view]')) {
    button.disabled = false;
    button.addEventListener('click', () => {
      screen.dataset.mode = button.dataset.view;
      for (const option of document.querySelectorAll('[data-view]'))
        option.setAttribute('aria-pressed', String(option === button));
      document.querySelector('[data-screen-caption]').textContent =
        descriptions[button.dataset.view];
    });
  }
}
