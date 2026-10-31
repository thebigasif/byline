// The words for who else has a post open, beside en.ts, which is near its size limit.
export const presence = {
  others: (names: string[]) => `Also here: ${names.join(", ")}, reading.`,
  held: (who: string) => `${who} is editing this post`,
  why: (who: string) =>
    `You can read it, but not change it. It is locked so two people never overwrite each other. It opens for you when ${who} closes it.`,
  cost: (who: string) =>
    `Taking over locks it for ${who} instead. Anything ${who} has not saved yet is not kept.`,
  takeOver: "Take over editing",
};
