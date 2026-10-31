// The words for photo and GIF search and for turning or cropping a picture,
// beside en.ts, which is near its size limit.
export const pictures = {
  unsplash: "Photo from Unsplash",
  unsplashSlashHint: "A free photo, with its credit under it",
  unsplashHint:
    "Photos stay on Unsplash and load from there. The photographer's name and a link go under the photo, as Unsplash asks.",
  gifs: "GIF",
  gifsSlashHint: "A moving picture from GIPHY",
  gifsHint: "GIFs stay on GIPHY and load from there. “Via GIPHY” goes under the GIF.",
  words: "Words to look for",
  found: "Pictures found",
  searching: "Looking for pictures",
  none: "Nothing was found for those words. Try other words.",
  failed: "The blog did not answer, so nothing was found. Check the connection, then try again.",
  by: (description: string, credit: string) =>
    description === "" ? `By ${credit}` : `${description}, by ${credit}`,
  photoBy: (credit: string) => `Photo by ${credit}`,
  cancel: "Cancel",
};
