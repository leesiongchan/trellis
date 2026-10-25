import { newId } from './normalize';
import type { CitedParagraph, Exploration, Source, TopicContent, TopicNode } from './types';

// Written demo content, not provider output. References are bundled links; nothing
// is fetched when this tree is opened, expanded, refreshed or restored.
const preparedAt = '2026-10-25T00:00:00.000Z';
const references: Record<string, Source & { excerpt: string }> = {
  overview: {
    id: 'sd-overview',
    title: 'SSDs, USB Drives, SD Memory Cards: Know the Difference',
    publisher: 'SD Association',
    url: 'https://www.sdcard.org/press/thoughtleadership/ssds-usb-drives-sd-memory-cards-know-the-difference/',
    retrievedAt: preparedAt,
    excerpt: 'NAND flash'
  },
  controller: {
    id: 'sd-controller',
    title: 'Controllers: The Unsung Heroes in Achieving A2',
    publisher: 'SD Association',
    url: 'https://www.sdcard.org/press/thoughtleadership/controllers-the-unsung-heroes-in-achieving-a2/',
    retrievedAt: preparedAt,
    excerpt: 'commands'
  },
  cells: {
    id: 'kioxia-cells',
    title: 'What is NAND Flash Memory?',
    publisher: 'KIOXIA',
    url: 'https://www.kioxia.com/en-jp/rd/technology/nand-flash.html',
    retrievedAt: preparedAt,
    excerpt: 'Memory cells'
  },
  tunneling: {
    id: 'kioxia-quantum',
    title: 'The Shining Sun and the Flash Memory for Storage',
    publisher: 'KIOXIA',
    url: 'https://www.kioxia.com/en-jp/rd/movie.html',
    retrievedAt: preparedAt,
    excerpt: 'quantum tunneling effect'
  },
  pages: {
    id: 'samsung-pages',
    title: 'A Brief History of Data Placement Technologies',
    publisher: 'Samsung Semiconductor',
    url: 'https://semiconductor.samsung.com/news-events/tech-blog/a-brief-history-of-data-placement-technologies/',
    retrievedAt: preparedAt,
    excerpt: 'pages'
  },
  wear: {
    id: 'micron-wear',
    title: 'What is an SSD?',
    publisher: 'Micron',
    url: 'https://www.micron.com/about/micron-glossary/solid-state-drives',
    retrievedAt: preparedAt,
    excerpt: 'wear leveling'
  },
  errors: {
    id: 'microchip-ecc',
    title: 'NAND Flash Boot: PMECC Error Detection and Correction',
    publisher: 'Microchip',
    url: 'https://onlinedocs.microchip.com/oxy/GUID-2ACDA668-0A87-46A1-B7FC-9DC74A5461AD-en-US-3/GUID-765D9391-03BC-403C-812F-F1EA941789C4.html',
    retrievedAt: preparedAt,
    excerpt: 'correct errors'
  }
};

interface DemoTopic {
  id: string;
  title: string;
  relationship: string;
  teaser: string;
  source: keyof typeof references;
  summary: string;
  sections: [string, string][];
  children: string[];
}
const topics: DemoTopic[] = [
  {
    id: 'card',
    title: 'microSD cards',
    relationship: 'Starting topic',
    teaser: 'A tiny object with a whole stack of ideas inside.',
    source: 'overview',
    summary:
      'A microSD card combines NAND flash, which retains the stored data, with a controller that communicates with the device using the card. Following either part leads to a different explanation of how the same small object works.',
    sections: [
      [
        'Two parts, two questions',
        'The memory answers “where does the information stay?” The controller answers “how does a camera or phone use it?” This tree separates those questions so you can follow the physical mechanism or the coordination around it.'
      ],
      [
        'The same idea in other shapes',
        'USB drives and SSDs also combine flash with a controller. Their packaging and interfaces differ, but this shared foundation makes one small card a useful starting point for understanding a larger family of storage devices.'
      ]
    ],
    children: ['nand', 'controller']
  },
  {
    id: 'nand',
    title: 'NAND flash',
    relationship: 'Inside the card',
    teaser: 'Meet the memory that keeps data without power.',
    source: 'cells',
    summary:
      'NAND flash arranges memory cells in dense arrays. Each cell stores an electrical state that remains after power is removed. The name describes an arrangement of cells, rather than a separate substance inside the card.',
    sections: [
      [
        'Zoom in',
        'A memory cell retains charge in an insulated storage region. Reading distinguishes electrical states; it does not look for a miniature picture or letter.'
      ],
      [
        'Zoom out',
        'Many cells work together to store a file. Follow Memory cells for the device physics, or Pages and blocks for how larger groups are read, programmed and erased.'
      ]
    ],
    children: ['cells', 'pages']
  },
  {
    id: 'cells',
    title: 'Memory cells',
    relationship: 'Smaller than a bit of text',
    teaser: 'How a stored electrical charge becomes readable information.',
    source: 'cells',
    summary:
      'A flash memory cell is a transistor with a region that can retain charge. Stored electrons change the threshold voltage: the gate voltage at which current begins to flow. A read operation tests the electrical response.',
    sections: [
      [
        'Charge changes the threshold',
        'The reader distinguishes states by applying voltages and sensing current. The stored information is represented by that measurable difference.'
      ],
      [
        'An insulating barrier',
        'The storage region is enclosed by insulation. Moving electrons across that barrier during writing leads to an unexpected branch: quantum tunneling.'
      ]
    ],
    children: ['quantum']
  },
  {
    id: 'quantum',
    title: 'Quantum tunneling',
    relationship: 'An unexpected connection',
    teaser: 'The physics inside an everyday memory card.',
    source: 'tunneling',
    summary:
      'Flash memory uses quantum tunneling to move electrons through a very thin insulating barrier under suitable electrical conditions. A microscopic quantum effect becomes part of the mechanism that stores everyday photos and documents.',
    sections: [
      [
        'A barrier at a different scale',
        'At this scale, a particle is described by quantum mechanics. Crossing a barrier is not the same process as a ball acquiring enough energy to roll over a hill.'
      ],
      [
        'Back to the card',
        'The useful connection is charge storage: moving charge changes a cell’s readable state. KIOXIA’s linked explanation connects flash memory and the Sun through tunneling, showing how one physical idea appears in very different settings.'
      ]
    ],
    children: []
  },
  {
    id: 'controller',
    title: 'Flash controller',
    relationship: 'The coordinator',
    teaser: 'The card has to do more than hold electrons.',
    source: 'controller',
    summary:
      'The controller connects the host device to the NAND memory. It handles commands, coordinates transfers and supports functions such as error correction. Memory capacity alone does not describe this coordination work.',
    sections: [
      [
        'Speaking both languages',
        'A host sends requests through the card interface. The controller handles that interface and directs operations on the NAND side, connecting the two parts of the storage system.'
      ],
      [
        'Two more paths',
        'Error correction deals with errors in the data that is read. Wear leveling deals with how use is spread across memory over time. Both show why a storage product needs management as well as memory cells.'
      ]
    ],
    children: ['errors', 'wear']
  },
  {
    id: 'errors',
    title: 'Error correction',
    relationship: 'Keeping data readable',
    teaser: 'A physical memory is not a perfect stream of bits.',
    source: 'errors',
    summary:
      'Error correction helps detect and repair some errors when NAND data is read. It is a practical layer between imperfect physical storage and the reliable information a device needs to use.',
    sections: [
      [
        'More than checking twice',
        'An error-correcting scheme uses extra information associated with data. Its capabilities are finite: the scheme and the number of errors determine what can be corrected.'
      ],
      [
        'A concrete implementation',
        'The linked Microchip documentation describes PMECC during NAND boot reads. That is one specific implementation, not a claim that every microSD card uses the same correction engine or settings.'
      ]
    ],
    children: []
  },
  {
    id: 'pages',
    title: 'Pages and blocks',
    relationship: 'From cells to operations',
    teaser: 'Writing and erasing do not use the same-sized unit.',
    source: 'pages',
    summary:
      'NAND operations work on groups of cells. Programming works in pages, while erasing works in larger blocks. This mismatch matters when a storage device has to update data repeatedly.',
    sections: [
      [
        'A page is part of a block',
        'A block contains multiple pages. Changing stored data can therefore involve more work than replacing exactly the bytes the host requested. Physical organization shapes the work that the controller must coordinate.'
      ],
      [
        'Why organization matters',
        'The linked Samsung article discusses data placement in SSDs. Its page-and-block explanation is useful background for NAND; specific SSD interfaces and policies should not be assumed to describe every memory card.'
      ]
    ],
    children: []
  },
  {
    id: 'wear',
    title: 'Wear leveling',
    relationship: 'A question of longevity',
    teaser: 'Why spread writes instead of repeatedly using the same place?',
    source: 'wear',
    summary:
      'Programming and erasing flash electrically stress its cells. Wear leveling spreads use across storage rather than letting a small set of locations receive all the repeated updates.',
    sections: [
      [
        'Logical location, physical work',
        'The address a host uses is an interface to storage. Management inside a device can choose physical locations so that repeated logical updates do not always stress the same cells.'
      ],
      [
        'A useful limit',
        'Spreading use can help extend useful life; it does not make flash immortal or replace a backup. The Micron reference explains this idea in the context of SSD management, which shares the underlying endurance concern.'
      ]
    ],
    children: []
  }
];

export function createDemoExploration(): Exploration {
  const nodes: Record<string, TopicNode> = {};
  for (const topic of topics) {
    const reference = references[topic.source];
    const paragraph = (id: string, text: string): CitedParagraph => ({
      id,
      text,
      evidence: [{ sourceId: reference.id, quote: reference.excerpt }]
    });
    const content: TopicContent = {
      title: topic.title,
      summary: [paragraph('summary', topic.summary)],
      sections: topic.sections.map(([heading, text], index) => ({
        heading,
        paragraphs: [paragraph('section-' + index, text)]
      })),
      analogy: null,
      limitations: null,
      sources: [
        {
          id: reference.id,
          title: reference.title,
          publisher: reference.publisher,
          url: reference.url,
          retrievedAt: reference.retrievedAt
        }
      ],
      branches: topic.children.map((id) => {
        const child = topics.find((item) => item.id === id)!;
        return {
          title: child.title,
          question: child.title,
          relationship: child.relationship,
          reason: paragraph('branch-' + id, child.teaser)
        };
      }),
      researchedAt: preparedAt
    };
    nodes[topic.id] = {
      id: topic.id,
      parentId: topics.find((item) => item.children.includes(topic.id))?.id ?? null,
      question: topic.title,
      relationship: topic.relationship,
      teaser: topic.teaser,
      status: 'ready',
      content,
      childIds: [...topic.children],
      lastError: null,
      isRefreshing: false
    };
  }
  const now = new Date().toISOString();
  return {
    schemaVersion: 1,
    mode: 'demo',
    id: newId(),
    originalPrompt: 'microSD cards',
    rootId: 'card',
    nodes,
    revision: 0,
    createdAt: now,
    updatedAt: now,
    view: {
      selectedNodeId: 'card',
      expandedNodeIds: ['card'],
      viewport: { x: 0, y: 0, zoom: 1 },
      mobilePane: 'map'
    }
  };
}
