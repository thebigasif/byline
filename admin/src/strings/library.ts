// The words for adding a picture from the media library to a post, beside
// en.ts, which is near its size limit.
export const library = {
  title: "Picture from your library",
  slashHint: "A picture you uploaded before",
  hint: "The picture goes into the post with the alt text and caption saved with it in the library. You can change both in the post afterwards.",
  words: "Words to look for",
  found: "Pictures in your library",
  loading: "Loading your pictures",
  empty: "Your media library has no pictures yet. Upload one on the Media screen, then come back.",
  none: "No picture's name or alt text has those words. Try other words.",
  failed:
    "The blog did not answer, so your pictures could not be listed. Check the connection, then try again.",
  noAlt: (name: string) => `No alt text yet: ${name}`,
  more: "Show more pictures",
  cancel: "Cancel",
};
