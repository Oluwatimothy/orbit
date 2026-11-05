/** Shared by the upload route and the browser so both enforce the same rules. */
export const UPLOAD_MAX_BYTES = 5 * 1024 * 1024;
export const UPLOAD_MIME_TYPES = [
  "image/png",
  "image/jpeg",
  "image/webp",
  "image/gif",
  "image/svg+xml",
] as const;

export function checkUpload(file: { size: number; type: string }): string | null {
  if (!(UPLOAD_MIME_TYPES as readonly string[]).includes(file.type)) {
    return "Use a PNG, JPEG, WebP, GIF or SVG image.";
  }
  if (file.size > UPLOAD_MAX_BYTES) return "Images must be 5 MB or smaller.";
  if (file.size === 0) return "This file is empty.";
  return null;
}
