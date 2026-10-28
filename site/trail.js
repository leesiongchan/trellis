const leafSource = {
  label: 'NOAA · The science of autumn leaves',
  url: 'https://www.noaa.gov/stories/cool-autumn-weather-reveals-nature-s-true-hues'
};
const memorySource = {
  label: 'Kingston · NAND flash technology',
  url: 'https://www.kingston.com/en/blog/pc-performance/nand-flash-technology-and-ssd'
};
const skySource = {
  label: 'NASA Space Place · Why is the sky blue?',
  url: 'https://spaceplace.nasa.gov/blue-sky/en/'
};

export const trails = {
  leaves: {
    id: 'leaves',
    label: 'An autumn leaf',
    question: 'Why do leaves change color?',
    root: {
      id: 'root',
      title: 'A familiar sight. A lovely mystery.',
      text: 'An autumn leaf is a small lesson in chemistry. As the days shorten, the green of chlorophyll fades, revealing other pigments. A familiar walk becomes a doorway into light, seasons, and the life of a tree.',
      connection: 'Choose a branch. There is more than one way into a question.',
      source: leafSource
    },
    branches: [
      {
        id: 'chlorophyll',
        title: 'Where does the green go?',
        text: 'Chlorophyll gives a leaf its green appearance and helps it capture light. As autumn brings shorter days, chlorophyll production slows and the pigment breaks down. The leaf’s other colors become easier to see.',
        connection: 'A question about color becomes a question about light.',
        source: leafSource
      },
      {
        id: 'palette',
        title: 'A hidden palette',
        text: 'Yellow and orange pigments are already present in many green leaves. When chlorophyll fades, those colors can show through. Red pigments follow a different story: many leaves produce them in the fall.',
        connection: 'Keep following: why are some leaves red, and others golden?',
        source: leafSource
      },
      {
        id: 'seasons',
        title: 'How does a tree tell time?',
        text: 'The changing length of the day helps set autumn’s timetable. Weather also shapes the display: temperature and moisture influence how vivid the colors become and how long the leaves remain on the tree.',
        connection: 'One leaf leads to the rhythm of an entire season.',
        source: leafSource
      }
    ]
  },
  memory: {
    id: 'memory',
    label: 'A tiny memory card',
    question: 'How can a tiny card hold so much?',
    root: {
      id: 'root',
      title: 'Small card. Remarkable capacity.',
      text: 'A memory card combines flash memory with a controller. Its capacity comes from enormous numbers of tiny cells, arranged to store information. Something you can balance on a fingertip opens a path into electronics.',
      connection: 'Follow the memory, the cells, or the decisions behind them.',
      source: memorySource
    },
    branches: [
      {
        id: 'nand',
        title: 'Meet NAND flash',
        text: 'NAND flash is nonvolatile memory: it keeps stored information when power is removed. Its cells represent information using electrical states. Reading and writing those states is the beginning of the story behind your files.',
        connection: 'An everyday object leads into the physics of memory.',
        source: memorySource
      },
      {
        id: 'layers',
        title: 'Think in three dimensions',
        text: 'Flash memory does not have to spread out across a single plane. In 3D NAND, manufacturers stack layers of memory cells. Building upward makes it possible to fit more storage into a small physical footprint.',
        connection: 'A storage question takes an architectural turn.',
        source: memorySource
      },
      {
        id: 'bits',
        title: 'More than one bit',
        text: 'Different types of NAND store different numbers of bits in a cell by distinguishing more electrical states. This creates tradeoffs between capacity, endurance, and performance. A tiny card contains a whole collection of design decisions.',
        connection: 'Keep going: what do we trade for more capacity?',
        source: memorySource
      }
    ]
  },
  sky: {
    id: 'sky',
    label: 'A patch of blue sky',
    question: 'Why is the sky blue?',
    root: {
      id: 'root',
      title: 'Look up. There is a story there.',
      text: 'Sunlight looks white, but contains many colors. Molecules in the air scatter some colors more strongly than others. The blue above us is an everyday invitation to understand how light meets our atmosphere.',
      connection: 'Follow a color, a sunset, or a sky on another world.',
      source: skySource
    },
    branches: [
      {
        id: 'blue',
        title: 'Why this particular blue?',
        text: 'Blue light has a shorter wavelength than red light. The small molecules in our atmosphere scatter it more strongly, sending it in many directions. Some of that scattered light reaches your eyes when you look away from the Sun.',
        connection: 'A simple color opens a door into waves and wavelengths.',
        source: skySource
      },
      {
        id: 'sunset',
        title: 'Then why a red sunset?',
        text: 'Near sunset, sunlight travels through more air before it reaches you. Much of the blue light has scattered away from that direct path. More of the reds and oranges remain, giving the evening sky its familiar warmth.',
        connection: 'The same mechanism tells a different story at dusk.',
        source: skySource
      },
      {
        id: 'mars',
        title: 'A sky on another world',
        text: 'A planet’s sky depends on its atmosphere. The fine dust in the thin Martian air scatters sunlight differently from Earth’s air. Images from Mars show reddish daytime skies and a blue glow near the setting Sun.',
        connection: 'A glance out the window becomes a journey to Mars.',
        source: skySource
      }
    ]
  }
};

export function createTrailState(trailId = 'leaves') {
  if (!Object.hasOwn(trails, trailId)) throw new RangeError('Unknown trail.');
  return { trailId, topicId: 'root' };
}

export function getSelection(state) {
  if (!Object.hasOwn(trails, state.trailId)) throw new RangeError('Unknown trail.');
  const trail = trails[state.trailId];
  const topic = [trail.root, ...trail.branches].find(({ id }) => id === state.topicId);
  if (!topic) throw new RangeError('Unknown topic for this trail.');
  return { trail, topic };
}

export function selectTopic(state, topicId) {
  const next = { ...state, topicId };
  getSelection(next);
  return next;
}
