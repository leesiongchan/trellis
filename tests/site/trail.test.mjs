import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createTrailState, selectTopic, getSelection, trails } from '../../site/trail.js';

test('following a branch updates the explanation and preserves the starting question', () => {
  const initial = createTrailState('leaves');
  const next = selectTopic(initial, 'palette');
  assert.equal(getSelection(next).topic.title, 'A hidden palette');
  assert.equal(getSelection(next).trail.question, 'Why do leaves change color?');
  assert.equal(getSelection(initial).topic.id, 'root');
  assert.ok(getSelection(next).topic.text.length > 80);
});

test('choosing a different trail returns to its root instead of showing the previous explanation', () => {
  const previous = selectTopic(createTrailState('leaves'), 'palette');
  const next = createTrailState('memory');
  assert.notEqual(next.trailId, previous.trailId);
  assert.equal(next.topicId, 'root');
  assert.equal(getSelection(next).topic.title, 'Small card. Remarkable capacity.');
});

test('a branch from another trail cannot produce a mismatched explanation', () => {
  assert.throws(() => selectTopic(createTrailState('leaves'), 'nand'), RangeError);
  assert.throws(() => createTrailState('__proto__'), RangeError);
  assert.throws(() => createTrailState('missing'), RangeError);
});

test('each example has three reachable branches and a public source for its explanation', () => {
  for (const trail of Object.values(trails)) {
    assert.equal(trail.branches.length, 3);
    assert.equal(new Set(trail.branches.map((topic) => topic.id)).size, 3);
    for (const topic of [trail.root, ...trail.branches]) {
      const state = selectTopic(createTrailState(trail.id), topic.id);
      assert.equal(getSelection(state).topic, topic);
      assert.ok(topic.title && topic.text && topic.connection);
      assert.match(topic.source.url, /^https:\/\//);
      assert.ok(topic.source.label);
    }
  }
});
