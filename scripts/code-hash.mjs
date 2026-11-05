// Prints the NEP-591 global contract hash (base58 SHA-256) of a wasm file.
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const bytes = createHash("sha256").update(readFileSync(process.argv[2])).digest();
let value = BigInt(`0x${bytes.toString("hex")}`);
let out = "";
while (value > 0n) {
  out = ALPHABET[Number(value % 58n)] + out;
  value /= 58n;
}
for (const byte of bytes) {
  if (byte !== 0) break;
  out = `1${out}`;
}
console.log(out);
