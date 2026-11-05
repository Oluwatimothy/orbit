import { NextResponse, type NextRequest } from "next/server";

import { allowRequest } from "@/lib/server/rate-limit";
import { UPLOAD_MAX_BYTES, UPLOAD_MIME_TYPES } from "@/lib/upload-policy";

const SIGN_ENDPOINT = "https://uploads.pinata.cloud/v3/files/sign";
const URL_TTL_SECONDS = 60;
const REQUESTS_PER_MINUTE = 10;

/** Whether uploads are configured, so the UI can offer or hide them. */
export function GET() {
  return NextResponse.json({ enabled: Boolean(process.env.PINATA_JWT) });
}

/**
 * Issues a short-lived Pinata upload URL restricted to small images. The JWT
 * never leaves the server; the browser uploads directly to Pinata.
 */
export async function POST(request: NextRequest) {
  const jwt = process.env.PINATA_JWT;
  if (!jwt) {
    return NextResponse.json({ error: "Uploads are not configured on this deployment." }, { status: 503 });
  }
  const ip = request.headers.get("x-forwarded-for")?.split(",")[0]?.trim() || "unknown";
  if (!allowRequest(`upload:${ip}`, REQUESTS_PER_MINUTE, 60_000)) {
    return NextResponse.json({ error: "Too many upload requests. Try again in a minute." }, { status: 429 });
  }

  const response = await fetch(SIGN_ENDPOINT, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${jwt}` },
    body: JSON.stringify({
      network: "public",
      date: Math.floor(Date.now() / 1000),
      expires: URL_TTL_SECONDS,
      max_file_size: UPLOAD_MAX_BYTES,
      allow_mime_types: UPLOAD_MIME_TYPES,
    }),
  });
  if (!response.ok) {
    console.error("Pinata sign request failed", response.status, await response.text().catch(() => ""));
    return NextResponse.json({ error: "The storage provider rejected the upload request." }, { status: 502 });
  }
  const body = (await response.json()) as { data?: unknown };
  if (typeof body.data !== "string" || !body.data.startsWith("https://")) {
    return NextResponse.json(
      { error: "The storage provider returned an unexpected response." },
      { status: 502 },
    );
  }
  return NextResponse.json({ url: body.data });
}
