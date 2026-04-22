# Artemis Governance Notice – Removal of DocLink / DocRef Surfaces

This document records the decision to treat all “DocLink” / “DocRef” / “file‑web control surface” constructs as **forbidden** in the Artemis constellation, and to replace them with simpler, non‑controlling registries and indices that do not behave as hidden control planes.[file:2]

---

## 1. Decision and Scope

Artemis governance has determined that any construct matching the following patterns is prohibited:

- ALN doctypes named `ArtemisDocLinkRegistry*`, `DocLink*`, `DocRef*`, or similar.  
- Control surfaces that introduce graph‑shaped “implements / depends_on / proves / tests” edges between artifacts in a way that could silently gate CI or runtime behavior.  
- “File‑web” components whose primary function is hidden cross‑artifact control rather than transparent indexing.

Any prior design that relied on *DocLink* style registries must be refactored to use explicit, human‑visible configuration and CI scripts, without abstract control graphs.[file:2]

Scope of removal:

- No new ALN schemas under the “DocLink” or “DocRef” naming family.  
- No CI jobs that depend on DocLink edges to accept or reject changes.  
- No AI‑Chat guidance that proposes adding or extending DocLink registries.

---

## 2. Replacement Principles for Indexing and Navigation

Artemis still needs **discoverability** and **navigation** across specs, crates, kernels, and qpudatashards, but this must not become a hidden control layer.[file:1][file:2]

New principles:

1. **Flat, explicit indices only**  
   - Use simple ALN or JSON tables listing artifacts with fields like `id`, `kind`, `path`, `status`.  
   - Avoid graph edges or inferred dependencies; any relation must be expressed in the artifact itself (e.g., a crate’s `Cargo.toml` referencing a path) or in transparent CI configuration.

2. **CI scripts, not registry logic, enforce rules**  
   - CI jobs read explicit lists of “critical artifacts” (e.g., a flat `CriticalArtifacts2026v1.aln`), not DocLink edges.  
   - All gating logic lives in readable scripts (`.github/workflows/*.yml` and associated CLIs), not in a meta‑graph.

3. **Per‑repo manifests, not global file‑webs**  
   - Each repo (`vsc-artemis-core`, `vsc-artemis`, `Data_Lake`, etc.) maintains its own manifest file enumerating its important files and their roles.  
   - Cross‑repo relationships stay in prose documentation or in clearly scoped ALN specs (e.g., a NexusGraph shard that explicitly imports a kernel parameter shard). There is no global, opaque control registry.

These principles preserve AI‑Chat’s ability to “navigate” the repos by reading manifests and specs, without introducing a central hidden switchboard.

---

## 3. Safer Alternatives to DocLink‑Style Graphs

Instead of DocLink registries, Artemis can rely on a small set of explicit, non‑controlling artifacts and tools.[file:1][file:2]

### 3.1 Per‑Repo Manifest Files

Each repository gets a manifest that:

- Lists ALN shards, Rust crates, C++ modules, qpudatashards, and CI workflows by path.  
- Tags each artifact with simple attributes: `plane`, `domain`, `critical = true/false`, `language`.  
- Contains **no** “implements / depends_on / proves” edges; it is purely descriptive.

Example manifest families (to be created or extended):

- `vsc-artemis-core/docs/research/101-repo-manifest-vsc-artemis.md` (already present as a large inventory; can be extended instead of introducing DocLink).[file:313]  
- Similar `101-repo-manifest-*.md` or `.aln` files for `vsc-artemis`, `vsc-artemis-qcs-aln`, `Data_Lake`, and `artemis-cyboquatic`.

AI‑Chat uses these manifests as the primary map for navigation, with no hidden controls.

### 3.2 Explicit CI Rules

CI pipelines should:

- Hard‑code which files must pass ALN schema validation (e.g., `ArtemisConstellation2026v1.aln`, `ArtemisEnvPlanesKernelParams2026v1.aln`).[file:2][file:1]  
- Hard‑code which crates must compile and run tests (`artemis-constellation-graph`, `artemis-nexusgraph`, validator crates).  
- Avoid any “if X changes, auto‑discover Y from a DocLink graph” behavior. Relationships, where needed, are expressed as direct path lists in CI config.

This keeps the control logic entirely transparent to maintainers and auditors.

---

## 4. AI‑Chat Integration Constraints

To ensure AI‑Chat tools and future generated artifacts respect this decision:

1. **No proposals involving DocLink or DocRef**  
   - AI‑Chat must not suggest creating new `ArtemisDocLinkRegistry*` shards or “file‑web control” graphs.  
   - AI‑Chat must not revive the term “DocLink” as a core pattern; references, if needed historically, must be clearly labeled as deprecated.

2. **Navigation via manifests and ALN imports only**  
   - AI‑Chat should rely on:
     - Per‑repo manifest files (like the existing `101-repo-manifest-vsc-artemis-*` documents).[file:313]  
     - ALN `schemarefs` imports already present in kernels, NexusGraph, constellation, ecoledger specs.[file:2][file:1]  
   - Cross‑artifact relationships are inferred from these explicit imports and paths, not from a separate graph layer.

3. **Governance auditability**  
   - Any new indexing or registry artifact must clearly document:
     - Its purpose.  
     - That it does **not** gate CI or runtime behavior except via transparent, scripted checks.  
   - If an artifact ever starts to behave like a hidden control plane, it must be split or removed.

---

## 5. Migration Notes

If any experimental DocLink‑like shards or diagrams exist in local branches or drafts:

- They should be retired, renamed, or refactored into:
  - Per‑repo manifests (flat inventories).  
  - Diagrams that are strictly documentation (Mermaid in `docs/architecture/*.md`) with no implied controlling semantics.[file:2]  
- Any previous gap‑tracking or cross‑repo analysis should be moved into:
  - A simple `ArtemisImplementationGaps2026v1.aln` indexed by gap ID, with no DocRef edges.[file:1]  
  - CI scripts that enumerate which gaps are considered “blocking” based on explicit lists.

This keeps all control behavior local, reviewable, and independent of any DocLink concept.

---

## 6. Summary for Future Work

Going forward:

- “DocLink registry” and similar constructs are **forbidden** patterns.  
- Indexing and navigation must use simple manifests, ALN `schemarefs`, and explicit CI configuration.  
- AI‑Chat–driven enhancements should strengthen these explicit patterns, not re‑introduce hidden control surfaces.

This notice is the authoritative record within `vsc-artemis-core` for the removal of DocLink‑style mechanisms and establishes the safer alternatives for indexing and navigation in the Artemis constellation.[file:2][file:1][file:313]
