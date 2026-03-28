# VSC-Artemis Ecosafety Spine

## Invariants

| Invariant | Description | Enforcement |
|-----------|-------------|-------------|
| `nocorridor_nobuild` | No actuation without corridor bands | Compile-time |
| `kerdeployable` | K≥0.94, E≥0.91, R≤0.13 for PRODUCTION | CI gate |
| `lyapunov_nonincrease` | Vt+1 ≤ Vt + ε | Runtime |
| `quantumorthogonal_ok` | Quantum fields barred from reward/identity | Compile + Runtime |

## KER Bands

| Lane | K | E | R |
|------|---|---|---|
| RESEARCH | ≥0.85 | ≥0.85 | ≤0.20 |
| PILOT | ≥0.90 | ≥0.90 | ≤0.15 |
| PRODUCTION | ≥0.94 | ≥0.91 | ≤0.13 |

## Repository Matrix

| Repo | Contracts Implemented |
|------|----------------------|
| vsc-artemis | VSCArtemis_ControllerSpec.aln |
| vsc-artemis-quantum | QuantumOrthogonalityGuard.aln |
| artemis-cyboquatic | Cyboquatic_ControllerSpec.aln |
| vsc-artemis-core | EcoSafetyKernel_v1.aln |
