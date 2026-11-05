// Prints the code hash (base58 SHA-256) of a wasm file, as NEAR explorers and
// the factory's `get_config().template.hash` show it.
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
