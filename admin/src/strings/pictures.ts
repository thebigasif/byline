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
  turnOpen: "Turn or crop",
  turnTitle: "Turn or crop the picture",
  turnHint:
    "Saving makes a new copy in the media library and puts it in this post. The original stays, so other posts that use it do not change.",
  preview: "The picture as it will look",
  left: "Turn left",
  right: "Turn right",
  shape: "Shape",
  shapes: {
    taken: "As taken",
    square: "Square",
    wide: "Wide, 16 by 9",
    standing: "Standing, 3 by 4",
    own: "Your own box",
  },
  shapeHint: "The crop keeps the picture's focus point in view.",
  ownHint:
    "Drag the box to move it and the round handle to change its size. The four fields below do the same without dragging. Turning starts the box again.",
  stage: "The picture, with the part to keep in a box",
  sides: {
    left: "From the left, %",
    top: "From the top, %",
    width: "Width, %",
    height: "Height, %",
  },
  saveCopy: "Save as a new copy",
  saving: "Making the copy",
  copyFailed: "The blog did not answer, so no copy was made. Check the connection, then try again.",
};
