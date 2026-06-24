// Marshals snarkjs Groth16 output (verification_key.json, proof.json, public.json)
// into the ecosystem-neutral vector format consumed by zkr-harness. Field elements
// are emitted as 32-byte big-endian hex; Fq2 coordinates keep the mathematical
// (c0, c1) order, leaving any ecosystem-specific reordering to the adapters.
import { readFileSync } from "node:fs";

const dir = process.argv[2] ?? ".";
const read = (name) => JSON.parse(readFileSync(`${dir}/${name}`, "utf8"));

const vk = read("verification_key.json");
const proof = read("proof.json");
const publicSignals = read("public.json");

const fq = (dec) => '"0x' + BigInt(dec).toString(16).padStart(64, "0") + '"';
const g1 = (p) => `{ x = ${fq(p[0])}, y = ${fq(p[1])} }`;
const g2 = (p) => `{ x = [${fq(p[0][0])}, ${fq(p[0][1])}], y = [${fq(p[1][0])}, ${fq(p[1][1])}] }`;

const lines = [
  'proof_system = "groth16"',
  'primitive = "BN254"',
  'expected = "accept"',
  "",
  `public_inputs = [${publicSignals.map(fq).join(", ")}]`,
  "",
  "[vk]",
  `alpha_g1 = ${g1(vk.vk_alpha_1)}`,
  `beta_g2 = ${g2(vk.vk_beta_2)}`,
  `gamma_g2 = ${g2(vk.vk_gamma_2)}`,
  `delta_g2 = ${g2(vk.vk_delta_2)}`,
  `ic = [`,
  vk.IC.map((p) => `  ${g1(p)},`).join("\n"),
  `]`,
  "",
  "[proof]",
  `a = ${g1(proof.pi_a)}`,
  `b = ${g2(proof.pi_b)}`,
  `c = ${g1(proof.pi_c)}`,
];

process.stdout.write(lines.join("\n") + "\n");
