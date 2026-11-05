import { checkUpload } from "@/lib/upload-policy";

export async function uploadsEnabled(): Promise<boolean> {
  const response = await fetch("/api/uploads/sign");
  if (!response.ok) return false;
  return Boolean(((await response.json()) as { enabled?: boolean }).enabled);
}

export async function sha256Base64(file: Blob): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", await file.arrayBuffer());
  return btoa(String.fromCharCode(...new Uint8Array(digest)));
}

/** Uploads an image to IPFS through a server-signed Pinata URL and returns `ipfs://<cid>`. */
export async function uploadImage(file: File): Promise<{ reference: string; sha256: string }> {
  const problem = checkUpload(file);
  if (problem) throw new Error(problem);

  const signed = await fetch("/api/uploads/sign", { method: "POST" });
  const signedBody = (await signed.json()) as { url?: string; error?: string };
  if (!signed.ok || !signedBody.url) throw new Error(signedBody.error ?? "Could not start the upload.");

  const form = new FormData();
  form.append("file", file);
  form.append("network", "public");
  const uploaded = await fetch(signedBody.url, { method: "POST", body: form });
  if (!uploaded.ok) throw new Error(`The upload failed (${uploaded.status}).`);
  const result = (await uploaded.json()) as { data?: { cid?: unknown } };
  const cid = result.data?.cid;
  if (typeof cid !== "string" || !/^[a-zA-Z0-9]+$/.test(cid))
    throw new Error("The storage provider did not return a CID.");

  return { reference: `ipfs://${cid}`, sha256: await sha256Base64(file) };
}
