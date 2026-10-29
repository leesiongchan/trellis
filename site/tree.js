const svgNamespace = 'http://www.w3.org/2000/svg';

function svgElement(name, attributes) {
  const element = document.createElementNS(svgNamespace, name);
  for (const [key, value] of Object.entries(attributes)) element.setAttribute(key, value);
  return element;
}

export function createKnowledgeTree(tree, root, branches) {
  const drawing = tree.querySelector('[data-tree-drawing]');
  const trunk = drawing.querySelector('[data-tree-trunk]');
  const activeTrunk = drawing.querySelector('[data-active-trunk]');
  const rootlets = drawing.querySelector('[data-rootlets]');
  const limbs = [...drawing.querySelectorAll('[data-tree-branch]')];
  let frame = 0;
  let selectedBranch = null;
  let activeBranch = null;
  let stemPaths = [];

  function highlight(index) {
    activeBranch = index;
    activeTrunk.setAttribute('d', index === null ? '' : (stemPaths[index] ?? ''));
    if (index === null) delete tree.dataset.activeBranch;
    else tree.dataset.activeBranch = String(index);
    for (const limb of limbs)
      limb.classList.toggle('is-active', limb.dataset.treeBranch === String(index));
  }

  function draw() {
    frame = 0;
    const bounds = tree.getBoundingClientRect();
    if (!bounds.width || !bounds.height) return;
    const rootBounds = root.getBoundingClientRect();
    const cards = [...branches.querySelectorAll('[data-branch-index]')];
    if (cards.length !== limbs.length) return;
    const stacked = getComputedStyle(tree).getPropertyValue('--tree-layout').trim() === 'stacked';
    drawing.setAttribute('viewBox', `0 0 ${bounds.width} ${bounds.height}`);

    const origin = {
      x: stacked ? 22 : rootBounds.left + rootBounds.width / 2 - bounds.left,
      y: (stacked ? rootBounds.bottom : rootBounds.top) - bounds.top
    };
    const tips = cards.map((card, index) => {
      const rect = card.getBoundingClientRect();
      return {
        x:
          (stacked
            ? rect.left
            : index === 0
              ? rect.right
              : index === 1
                ? rect.left + rect.width / 2
                : rect.left) - bounds.left,
        y: (stacked || index !== 1 ? rect.top + rect.height / 2 : rect.bottom) - bounds.top
      };
    });

    const fork = stacked
      ? { x: origin.x, y: tips[0].y }
      : {
          x: origin.x + 8,
          y: origin.y - Math.min(140, (origin.y - Math.max(...tips.map(({ y }) => y))) * 0.55)
        };
    const stem = stacked
      ? `M ${origin.x} ${origin.y} C ${origin.x - 8} ${origin.y + 35} ${origin.x + 5} ${fork.y - 30} ${fork.x} ${fork.y}`
      : `M ${origin.x} ${origin.y} C ${origin.x - 12} ${origin.y - 48} ${fork.x + 9} ${fork.y + 48} ${fork.x} ${fork.y}`;
    trunk.setAttribute('d', stacked ? `${stem} L ${fork.x} ${tips[2].y + 32}` : stem);
    stemPaths = tips.map((tip) => (stacked ? `${stem} L ${fork.x} ${tip.y + 32}` : stem));
    highlight(activeBranch);
    rootlets.setAttribute(
      'd',
      [-1, 0, 1]
        .map((direction) => {
          const y = rootBounds.bottom - bounds.top;
          return `M ${origin.x} ${y} C ${origin.x + direction * 10} ${y + 15} ${origin.x + direction * 35} ${y + 25} ${origin.x + direction * 60} ${y + 45 - Math.abs(direction) * 8}`;
        })
        .join(' ')
    );

    limbs.forEach((limb, index) => {
      const tip = tips[index];
      const path = limb.querySelector('[data-tree-limb]');
      const start = stacked ? { x: origin.x, y: tip.y + 32 } : fork;
      const direction = index === 0 ? -1 : 1;
      path.setAttribute(
        'd',
        stacked
          ? `M ${start.x} ${start.y} C ${start.x} ${tip.y + 8} ${tip.x - 22} ${tip.y} ${tip.x} ${tip.y}`
          : index === 1
            ? `M ${start.x} ${start.y} C ${start.x + 28} ${start.y - 115} ${tip.x - 32} ${tip.y + 105} ${tip.x} ${tip.y}`
            : `M ${start.x} ${start.y} C ${start.x + direction * 55} ${start.y - 110} ${tip.x - direction * 90} ${tip.y + 15} ${tip.x} ${tip.y}`
      );
      const bud = limb.querySelector('[data-tree-bud]');
      bud.setAttribute('cx', tip.x);
      bud.setAttribute('cy', tip.y);
      const foliage = limb.querySelector('[data-tree-foliage]');
      const length = path.getTotalLength();
      const leaves = (stacked ? [0.35] : [0.28, 0.54, 0.72]).map((fraction, leafIndex) => {
        const point = path.getPointAtLength(length * fraction);
        const ahead = path.getPointAtLength(Math.min(length, length * fraction + 1));
        const angle = (Math.atan2(ahead.y - point.y, ahead.x - point.x) * 180) / Math.PI;
        const leaf = svgElement('g', {
          transform: `translate(${point.x} ${point.y}) rotate(${angle + (leafIndex % 2 ? 60 : -65)}) scale(${stacked ? 0.42 : 1})`
        });
        leaf.append(
          svgElement('path', { class: 'tree-leaf', d: 'M0 0C12-18 38-18 48 0C30 8 11 13 0 0Z' }),
          svgElement('path', { class: 'tree-vein', d: 'M2 0Q24-3 43 0' })
        );
        return leaf;
      });
      foliage.replaceChildren(...leaves);
    });
    tree.dataset.ready = '';
  }

  function scheduleDraw() {
    if (!frame) frame = requestAnimationFrame(draw);
  }

  const observer = new ResizeObserver(scheduleDraw);
  function refresh() {
    observer.disconnect();
    for (const element of [tree, root, ...branches.querySelectorAll('[data-branch-index]')])
      observer.observe(element);
    scheduleDraw();
  }

  tree.addEventListener('pointerover', (event) => {
    const card = event.target.closest('[data-root], [data-branch-index]');
    if (card) highlight(card.hasAttribute('data-root') ? null : Number(card.dataset.branchIndex));
  });
  tree.addEventListener('pointerout', (event) => {
    const card = event.target.closest('[data-root], [data-branch-index]');
    if (card && !card.contains(event.relatedTarget)) highlight(selectedBranch);
  });
  tree.addEventListener('pointerleave', () => highlight(selectedBranch));
  tree.addEventListener('focusin', (event) => {
    const card = event.target.closest('[data-root], [data-branch-index]');
    if (card) highlight(card.hasAttribute('data-root') ? null : Number(card.dataset.branchIndex));
  });
  tree.addEventListener('focusout', (event) => {
    if (!tree.contains(event.relatedTarget)) highlight(selectedBranch);
  });
  window.addEventListener('resize', scheduleDraw);
  document.fonts.ready.then(refresh);
  refresh();

  return {
    refresh,
    select(index) {
      selectedBranch = index;
      highlight(index);
    }
  };
}
