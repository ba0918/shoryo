// Rounds as the agent writes them, for the browser tests.

export function option(text, recommended, extra = {}) {
  return {
    text,
    description: [{ type: "text", body: `${text}: what it means in practice.` }],
    recommended,
    consequence: `With ${text}, the topic goes this way.`,
    ...extra,
  };
}

export function question(id, text, { cls = "human", premises = [], options } = {}) {
  return {
    id,
    text,
    class: cls,
    why_now: `Later questions rest on ${id}.`,
    premises,
    background: [{ type: "text", body: `Background for ${id}: the terms it uses.` }],
    options: options ?? [option(`${id} yes`, true), option(`${id} no`, false)],
  };
}

export function decision(id, name, decidedBy) {
  return { id, name, text: `${name}, written out in full.`, decided_by: { question: decidedBy } };
}

export const roundOne = {
  title: "壁打ちの画面",
  original_request: "I want to answer brainstorm questions on a screen, not in the chat.",
  subject: "Purpose",
  questions: [question("q1", "Who reads the screen?")],
  finished_picture: "a = Screen\nb = ? Storage\n| a | b |\na -> b : saves",
};

export const roundTwo = {
  subject: "Storage",
  questions: [
    question("q2", "Where does the data live?", {
      premises: ["d1"],
      options: [
        option("One JSON file", true),
        option("A database", false),
        option("Memory only", false),
      ],
    }),
    question("q3", "What is the file called?", { cls: "provisional", premises: ["d1"] }),
    question("q4", "Who may delete the data?"),
  ],
  records: { decisions: [decision("d1", "Person reads cards", "q1")] },
};

/// Round 1 sent, round 2 on the screen: q2 and q4 are human questions, q3 is provisional.
export async function twoRounds(shoryo) {
  await shoryo.round(roundOne);
  await shoryo.submit();
  await shoryo.round(roundTwo);
}
