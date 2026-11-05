import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = { title: "How it works" };

const SECTIONS: Array<{ id: string; title: string; body: ReactNode }> = [
  {
    id: "launches",
    title: "How a launch works",
    body: (
      <>
        <p>
          You sign one transaction to the Orbit factory. The factory checks your configuration with the same
          rules the contract uses, records the launch in its on-chain registry, and creates one account for it
          — <code>&lt;slug&gt;.&lt;factory&gt;</code>. That single contract is the token, the collection, or
          both, together with any holder claims and gated sale.
        </p>
        <p>
          The factory stores one launch contract template and deploys a copy of it to each new account. The
          template&apos;s SHA-256 hash is public in the factory&apos;s configuration and matches the code hash
          explorers show for every launch account. Creating the account, deploying the code and initialising
          it happen in a single atomic receipt: if initialisation fails, the account is never created and its
          balance is refunded.
        </p>
        <p>
          The launch deposit pays for storing that code and the launch&apos;s own data on its account (about
          4.4 testnet NEAR). It stays locked with the keyless account. The unused part of the registry
          allowance is refunded.
        </p>
        <p>
          Launch accounts have no access keys. Nobody — not you, not Orbit — can upgrade them, re-initialise
          them or withdraw from them.
        </p>
      </>
    ),
  },
  {
    id: "tokens",
    title: "Token allocations",
    body: (
      <>
        <p>
          Tokens are standard NEP-141 contracts with NEP-145 storage management and NEP-148 metadata. The
          whole supply is minted in the initialisation call to the allocations you configured, and the
          contract requires those allocations to add up to exactly the total supply. There is no mint method
          afterwards.
        </p>
        <p>
          Allocations are transfers, not vesting schedules: each recipient can move its tokens immediately.
          Orbit does not yet implement on-chain vesting, and does not pretend to with countdowns.
        </p>
      </>
    ),
  },
  {
    id: "nfts",
    title: "NFT collections",
    body: (
      <p>
        Collections are NEP-171 contracts with NEP-177 metadata, NEP-178 approvals and NEP-181 enumeration.
        Token IDs are sequential. Every mint checks the pause switch, the mint window, the supply cap and the
        per-wallet limit, then requires a deposit covering the price and the storage the new NFTs use; the
        price is paid to the creator and the unused deposit is refunded in the same transaction. Royalties
        (NEP-199) are not implemented.
      </p>
    ),
  },
  {
    id: "claims",
    title: "NFT-holder claims",
    body: (
      <>
        <p>
          A connected launch reserves part of the token supply for holders. The token, the collection and the
          reserve live in the same contract: the reserve is minted at launch to the launch account itself, and
          the contract refuses to initialise unless that allocation equals exactly what the claim tiers and
          the sale require.
        </p>
        <p>To claim for NFT #n, the contract, in one step:</p>
        <ol>
          <li>rejects the call if NFT #n is in no tier, or already has a claim record;</li>
          <li>
            reads the owner of NFT #n from its own collection and rejects the call unless it is the caller;
          </li>
          <li>records the claim and transfers the tier amount from the reserve to the caller;</li>
          <li>keeps the storage this added (about 0.003 NEAR) from the deposit and refunds the rest.</li>
        </ol>
        <p>
          These happen in a single receipt with no cross-contract calls, so a claim either completes entirely
          or changes nothing. There is no &ldquo;in progress&rdquo; state to get stuck in.
        </p>
      </>
    ),
  },
  {
    id: "sale",
    title: "NFT-gated token sales",
    body: (
      <>
        <p>
          A connected launch can also reserve tokens for a gated sale. During the holder window, only the
          current owner of an NFT can buy, up to a fixed amount per NFT ID; ownership is read from the
          collection on every purchase. An optional public window then lets any account buy up to a
          per-account limit.
        </p>
        <p>
          Like claims, a purchase is a single step: the ownership check, the limit update, the token transfer
          and the payment to the creator happen together or not at all. Claims and the sale draw on separate
          counters, so neither can spend the other&apos;s share of the reserve. After the sale ends, the
          creator can reclaim unsold tokens.
        </p>
      </>
    ),
  },
  {
    id: "eligibility",
    title: "How ownership eligibility works",
    body: (
      <p>
        Eligibility is checked against <strong>current</strong> ownership at the moment of the claim, not a
        historical snapshot. Whoever holds an NFT can claim its allocation once. If an NFT is sold before it
        is claimed, the buyer can claim; after a claim, that NFT can never claim again, whoever holds it.
        Orbit chose this over a Merkle snapshot because it needs no trusted party to reconstruct past
        ownership and publish a root.
      </p>
    ),
  },
  {
    id: "control",
    title: "What creators control",
    body: (
      <ul>
        <li>
          Collection creators can pause and resume minting, and mint to any account within the supply cap —
          including outside the public mint window. In a connected launch, NFTs the creator mints can claim
          allocations and use sale quota like any other NFT.
        </li>
        <li>
          Nothing else can be changed after launch: not supply, price, mint window, allocations or claim
          tiers.
        </li>
        <li>
          Creators choose the economics. Orbit checks that they are consistent and funded — not that they are
          fair.
        </li>
      </ul>
    ),
  },
  {
    id: "assumptions",
    title: "Assumptions and limitations",
    body: (
      <ul>
        <li>Testnet only. Assets have no monetary value.</li>
        <li>The contracts are tested but have not been audited.</li>
        <li>
          The factory owner can change the template future launches use, and pause new launches. Existing
          launches are unaffected.
        </li>
        <li>
          Media is stored by reference. IPFS content is content-addressed but stays available only while it is
          pinned; plain HTTPS media can be changed by its host, and Orbit labels it as such.
        </li>
        <li>
          Data comes from public RPC endpoints and the FastNEAR index. If they fail, Orbit shows an error
          rather than empty or stale numbers.
        </li>
      </ul>
    ),
  },
];

export default function Page() {
  return (
    <div className="mx-auto grid max-w-5xl gap-10 px-4 py-6 lg:grid-cols-[180px_minmax(0,1fr)] lg:px-8 lg:py-10">
      <nav aria-label="On this page" className="hidden lg:block">
        <ul className="sticky top-20 space-y-1.5 text-xs">
          {SECTIONS.map((s) => (
            <li key={s.id}>
              <a href={`#${s.id}`} className="text-ink-muted hover:text-ink">
                {s.title}
              </a>
            </li>
          ))}
        </ul>
      </nav>
      <article className="space-y-10">
        <header className="space-y-2">
          <h1 className="text-2xl font-semibold tracking-tight text-ink">How Orbit works</h1>
          <p className="text-sm text-ink-muted">
            What is enforced by contracts, what is up to the creator, and what you are trusting.
          </p>
        </header>
        {SECTIONS.map((s) => (
          <section
            key={s.id}
            id={s.id}
            className="scroll-mt-20 space-y-3 border-t border-line pt-6 text-sm leading-6 text-ink-muted [&_code]:tabular [&_code]:text-xs [&_code]:text-ink [&_li]:pl-1 [&_ol]:list-decimal [&_ol]:space-y-1 [&_ol]:pl-5 [&_strong]:text-ink [&_ul]:list-disc [&_ul]:space-y-1.5 [&_ul]:pl-5"
          >
            <h2 className="text-base font-semibold text-ink">{s.title}</h2>
            {s.body}
          </section>
        ))}
      </article>
    </div>
  );
}
