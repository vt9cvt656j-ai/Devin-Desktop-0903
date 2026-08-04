/**
 * Turn whatever file the account holder picked into a small square `data:` URL.
 *
 * The resize happens here, in the browser, on purpose. A photo straight off a phone is
 * 3–8 MB and 4000px wide; the profile renders it at 40px. Sending the original would
 * mean uploading megabytes to display a thumbnail, and would put a file-sized blob in a
 * database column. Re-encoding first makes the upload ~30 KB and means the server only
 * ever sees an image it can store inline.
 *
 * Drawing through a canvas also strips EXIF — including the GPS coordinates phones
 * attach to photos, which the account holder is not choosing to publish by setting a
 * profile picture.
 */

/** Rendered at 40px at most, so 256 covers a 2× display with room to spare. */
const SIZE = 256;
/** Above this the browser is being asked to decode something that is not a photo. */
const MAX_SOURCE_BYTES = 12 * 1024 * 1024;

export const ACCEPTED = "image/png,image/jpeg,image/webp,image/gif";

export class AvatarError extends Error {}

function loadImage(file: File): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      URL.revokeObjectURL(url);
      resolve(img);
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new AvatarError("unreadable"));
    };
    img.src = url;
  });
}

/**
 * Centre-cropped to a square before scaling: fitting a portrait photo into a circle by
 * squashing it is worse than trimming the sides.
 */
export async function fileToAvatarDataUrl(file: File): Promise<string> {
  if (!file.type.startsWith("image/")) throw new AvatarError("not-an-image");
  if (file.size > MAX_SOURCE_BYTES) throw new AvatarError("too-large");

  const img = await loadImage(file);
  const side = Math.min(img.naturalWidth, img.naturalHeight);
  if (!side) throw new AvatarError("unreadable");
  const sx = (img.naturalWidth - side) / 2;
  const sy = (img.naturalHeight - side) / 2;

  const canvas = document.createElement("canvas");
  canvas.width = SIZE;
  canvas.height = SIZE;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new AvatarError("unreadable");
  ctx.imageSmoothingQuality = "high";
  ctx.drawImage(img, sx, sy, side, side, 0, 0, SIZE, SIZE);

  // WebP where it exists, JPEG otherwise. A browser that cannot encode the format it was
  // asked for silently hands back a PNG, which for a photo is several times larger — so
  // the result is checked rather than trusted.
  let out = canvas.toDataURL("image/webp", 0.85);
  if (!out.startsWith("data:image/webp")) out = canvas.toDataURL("image/jpeg", 0.85);
  if (!out.startsWith("data:image/")) throw new AvatarError("unreadable");
  return out;
}
